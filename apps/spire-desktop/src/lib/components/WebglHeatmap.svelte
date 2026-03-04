<!--
  SPIRE — GPU-Accelerated WebGL2 Heatmap

  Renders 2D histogram data using a WebGL2 fragment shader with
  colourmap support (Viridis / Magma). Replaces the previous
  Canvas 2D heatmap for significantly better performance on large grids.

  Props:
    data        — Row-major Float32Array of bin contents.
    nx, ny      — Grid dimensions (columns × rows).
    maxVal      — Maximum bin value for colour normalisation.
    xEdges      — X-axis bin edge array.
    yEdges      — Y-axis bin edge array.
    title       — Plot title string.
    colorScale  — Colour map: "viridis" | "magma".
-->
<script lang="ts">
  import { onMount, onDestroy, afterUpdate } from "svelte";

  // ── Props ──
  export let data: Float32Array | number[] = new Float32Array(0);
  export let nx: number = 0;
  export let ny: number = 0;
  export let maxVal: number = 1;
  export let xEdges: number[] = [];
  export let yEdges: number[] = [];
  export let title: string = "";
  export let colorScale: "viridis" | "magma" = "viridis";

  // ── Internal state ──
  let canvasEl: HTMLCanvasElement;
  let overlayEl: HTMLDivElement;
  let gl: WebGL2RenderingContext | null = null;
  let program: WebGLProgram | null = null;
  let dataTexture: WebGLTexture | null = null;
  let vao: WebGLVertexArrayObject | null = null;

  // Uniform locations
  let uTexLoc: WebGLUniformLocation | null = null;
  let uNxLoc: WebGLUniformLocation | null = null;
  let uNyLoc: WebGLUniformLocation | null = null;
  let uMaxValLoc: WebGLUniformLocation | null = null;
  let uColorScaleLoc: WebGLUniformLocation | null = null;

  // Layout constants
  const MARGIN_LEFT = 55;
  const MARGIN_BOTTOM = 40;
  const MARGIN_TOP = 28;
  const MARGIN_RIGHT = 70;

  // ---------------------------------------------------------------------------
  // Shaders
  // ---------------------------------------------------------------------------

  const VERT_SRC = `#version 300 es
    precision highp float;
    in vec2 aPos;
    out vec2 vUV;
    void main() {
      vUV = aPos * 0.5 + 0.5;
      gl_Position = vec4(aPos, 0.0, 1.0);
    }
  `;

  const FRAG_SRC = `#version 300 es
    precision highp float;
    uniform sampler2D uTex;
    uniform int uNx;
    uniform int uNy;
    uniform float uMaxVal;
    uniform int uColorScale; // 0 = viridis, 1 = magma
    in vec2 vUV;
    out vec4 fragColor;

    // Simplified 5-stop Viridis
    vec3 viridis(float t) {
      vec3 c0 = vec3(0.267, 0.004, 0.329);
      vec3 c1 = vec3(0.231, 0.322, 0.545);
      vec3 c2 = vec3(0.129, 0.569, 0.549);
      vec3 c3 = vec3(0.369, 0.788, 0.384);
      vec3 c4 = vec3(0.992, 0.906, 0.145);
      float s = clamp(t, 0.0, 1.0) * 4.0;
      float i = floor(s);
      float f = fract(s);
      if (i < 1.0) return mix(c0, c1, f);
      if (i < 2.0) return mix(c1, c2, f);
      if (i < 3.0) return mix(c2, c3, f);
      return mix(c3, c4, f);
    }

    // Simplified 5-stop Magma
    vec3 magma(float t) {
      vec3 c0 = vec3(0.001, 0.000, 0.014);
      vec3 c1 = vec3(0.380, 0.099, 0.534);
      vec3 c2 = vec3(0.784, 0.197, 0.373);
      vec3 c3 = vec3(0.988, 0.522, 0.216);
      vec3 c4 = vec3(0.987, 0.991, 0.749);
      float s = clamp(t, 0.0, 1.0) * 4.0;
      float i = floor(s);
      float f = fract(s);
      if (i < 1.0) return mix(c0, c1, f);
      if (i < 2.0) return mix(c1, c2, f);
      if (i < 3.0) return mix(c2, c3, f);
      return mix(c3, c4, f);
    }

    void main() {
      // Map UV to bin indices
      int ix = int(vUV.x * float(uNx));
      int iy = int(vUV.y * float(uNy));
      ix = clamp(ix, 0, uNx - 1);
      iy = clamp(iy, 0, uNy - 1);

      // Fetch from R32F texture (row-major: row = iy, col = ix)
      float val = texelFetch(uTex, ivec2(ix, iy), 0).r;
      float t = uMaxVal > 0.0 ? val / uMaxVal : 0.0;

      vec3 col;
      if (uColorScale == 1) {
        col = magma(t);
      } else {
        col = viridis(t);
      }
      fragColor = vec4(col, 1.0);
    }
  `;

  // ---------------------------------------------------------------------------
  // WebGL Setup
  // ---------------------------------------------------------------------------

  function compileShader(src: string, type: number): WebGLShader | null {
    if (!gl) return null;
    const s = gl.createShader(type);
    if (!s) return null;
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
      console.error("Shader compile error:", gl.getShaderInfoLog(s));
      gl.deleteShader(s);
      return null;
    }
    return s;
  }

  function initWebGL(): boolean {
    if (!canvasEl) return false;
    gl = canvasEl.getContext("webgl2", { antialias: false, alpha: false });
    if (!gl) {
      console.warn("WebGL2 not available, falling back.");
      return false;
    }

    const vs = compileShader(VERT_SRC, gl.VERTEX_SHADER);
    const fs = compileShader(FRAG_SRC, gl.FRAGMENT_SHADER);
    if (!vs || !fs) return false;

    program = gl.createProgram();
    if (!program) return false;
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);

    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.error("Program link error:", gl.getProgramInfoLog(program));
      return false;
    }

    uTexLoc = gl.getUniformLocation(program, "uTex");
    uNxLoc = gl.getUniformLocation(program, "uNx");
    uNyLoc = gl.getUniformLocation(program, "uNy");
    uMaxValLoc = gl.getUniformLocation(program, "uMaxVal");
    uColorScaleLoc = gl.getUniformLocation(program, "uColorScale");

    // Full-screen quad VAO (two triangles)
    const quadVerts = new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]);
    vao = gl.createVertexArray();
    gl.bindVertexArray(vao);

    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, quadVerts, gl.STATIC_DRAW);

    const posLoc = gl.getAttribLocation(program, "aPos");
    gl.enableVertexAttribArray(posLoc);
    gl.vertexAttribPointer(posLoc, 2, gl.FLOAT, false, 0, 0);
    gl.bindVertexArray(null);

    // Data texture (R32F)
    dataTexture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, dataTexture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

    return true;
  }

  // ---------------------------------------------------------------------------
  // Upload & Draw
  // ---------------------------------------------------------------------------

  function uploadAndDraw(): void {
    if (!gl || !program || !dataTexture || !vao) return;
    if (nx <= 0 || ny <= 0) return;

    // Upload histogram data as R32F texture
    const floatData = data instanceof Float32Array ? data : new Float32Array(data);
    gl.bindTexture(gl.TEXTURE_2D, dataTexture);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R32F, nx, ny, 0, gl.RED, gl.FLOAT, floatData);

    // Render
    gl.viewport(0, 0, canvasEl.width, canvasEl.height);
    gl.clearColor(0.118, 0.118, 0.118, 1.0);
    gl.clear(gl.COLOR_BUFFER_BIT);

    gl.useProgram(program);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, dataTexture);
    gl.uniform1i(uTexLoc, 0);
    gl.uniform1i(uNxLoc, nx);
    gl.uniform1i(uNyLoc, ny);
    gl.uniform1f(uMaxValLoc, maxVal);
    gl.uniform1i(uColorScaleLoc, colorScale === "magma" ? 1 : 0);

    gl.bindVertexArray(vao);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
    gl.bindVertexArray(null);
  }

  // ---------------------------------------------------------------------------
  // Overlay axis labels (HTML, positioned over the canvas)
  // ---------------------------------------------------------------------------

  function buildAxisLabels(): string {
    if (nx <= 0 || ny <= 0) return "";
    let html = "";

    // Title
    html += `<div class="hm-title">${title}</div>`;

    // X-axis labels
    const xCount = Math.min(xEdges.length, 8);
    const xStep = Math.max(1, Math.floor((xEdges.length - 1) / (xCount - 1)));
    for (let i = 0; i < xEdges.length; i += xStep) {
      const pct = (i / (xEdges.length - 1)) * 100;
      html += `<span class="hm-xlabel" style="left:${pct}%">${xEdges[i].toFixed(1)}</span>`;
    }

    // Y-axis labels
    const yCount = Math.min(yEdges.length, 8);
    const yStep = Math.max(1, Math.floor((yEdges.length - 1) / (yCount - 1)));
    for (let i = 0; i < yEdges.length; i += yStep) {
      const pct = 100 - (i / (yEdges.length - 1)) * 100;
      html += `<span class="hm-ylabel" style="top:${pct}%">${yEdges[i].toFixed(1)}</span>`;
    }

    // Colour bar labels
    html += `<span class="hm-cbar-lo">0</span>`;
    html += `<span class="hm-cbar-hi">${maxVal.toExponential(1)}</span>`;

    return html;
  }

  $: axisHtml = buildAxisLabels();

  // ---------------------------------------------------------------------------
  // Reactivity — re-upload texture when data changes
  // ---------------------------------------------------------------------------

  $: if (gl && data && nx > 0 && ny > 0) {
    uploadAndDraw();
  }

  // Also redraw when colorScale changes
  $: if (gl && colorScale) {
    uploadAndDraw();
  }

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  let glReady = false;

  onMount(() => {
    glReady = initWebGL();
    if (glReady && data && nx > 0 && ny > 0) {
      uploadAndDraw();
    }
  });

  onDestroy(() => {
    if (gl) {
      if (dataTexture) gl.deleteTexture(dataTexture);
      if (program) gl.deleteProgram(program);
      if (vao) gl.deleteVertexArray(vao);
    }
  });
</script>

<div class="webgl-heatmap">
  <div class="hm-overlay" bind:this={overlayEl}>
    {@html axisHtml}
  </div>
  <canvas bind:this={canvasEl} width={600} height={400} class="hm-canvas"></canvas>
  {#if !glReady && nx > 0}
    <div class="hm-fallback">WebGL2 not available.</div>
  {/if}
</div>

<style>
  .webgl-heatmap {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 180px;
  }

  .hm-canvas {
    display: block;
    width: 100%;
    height: 100%;
    border-radius: 4px;
  }

  .hm-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 1;
  }

  .hm-overlay :global(.hm-title) {
    position: absolute;
    top: 4px;
    left: 50%;
    transform: translateX(-50%);
    color: #e0e0e0;
    font-size: 0.78rem;
    font-weight: 600;
    white-space: nowrap;
  }

  .hm-overlay :global(.hm-xlabel) {
    position: absolute;
    bottom: 4px;
    transform: translateX(-50%);
    color: #999;
    font-size: 0.65rem;
    font-family: "JetBrains Mono", monospace;
  }

  .hm-overlay :global(.hm-ylabel) {
    position: absolute;
    left: 2px;
    transform: translateY(-50%);
    color: #999;
    font-size: 0.65rem;
    font-family: "JetBrains Mono", monospace;
  }

  .hm-overlay :global(.hm-cbar-lo) {
    position: absolute;
    bottom: 4px;
    right: 4px;
    color: #999;
    font-size: 0.6rem;
    font-family: "JetBrains Mono", monospace;
  }

  .hm-overlay :global(.hm-cbar-hi) {
    position: absolute;
    top: 28px;
    right: 4px;
    color: #999;
    font-size: 0.6rem;
    font-family: "JetBrains Mono", monospace;
  }

  .hm-fallback {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #ef5350;
    font-size: 0.82rem;
    background: rgba(0, 0, 0, 0.6);
    border-radius: 4px;
  }
</style>
