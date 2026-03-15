<!--
  SPIRE - 3D Event Display with Time-of-Flight Animation

  Interactive Three.js-powered visualization of particle physics events.
  Shows a wireframe detector barrel with jet cones, lepton tracks,
  photon lines, and missing transverse energy (MET) arrows.

  Features:
  - Batch event generation (up to 100 events buffered)
  - Time-of-flight animation: particles propagate from the interaction point
  - Playback controls: Play, Pause, Step Forward/Back, Speed slider
  - Event index navigation across the generated batch
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { appendLog } from "$lib/stores/physicsStore";
  import { generateDisplayBatch } from "$lib/api";
  import type {
    EventDisplayData,
    DisplayJet,
    DisplayTrack,
    DisplayMET,
    Vec3,
    DetectorPreset,
  } from "$lib/types/spire";
  import SpireNumberInput from "$lib/components/ui/SpireNumberInput.svelte";
  import SpireSlider from "$lib/components/ui/SpireSlider.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import * as THREE from "three";
  import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";

  // ---------------------------------------------------------------------------
  // Configuration State
  // ---------------------------------------------------------------------------
  let cmsEnergy: number = 200;
  let nFinal: number = 4;
  let detectorPreset: DetectorPreset = "lhc_like";
  let batchSize: number = 20;
  let loading: boolean = false;
  let errorMsg: string = "";

  // ---------------------------------------------------------------------------
  // Event Batch & Playback State
  // ---------------------------------------------------------------------------
  let eventBatch: EventDisplayData[] = [];
  let currentEventIdx: number = 0;

  /** Normalised simulation time in [0, 1]. 0 = interaction point, 1 = fully propagated. */
  let simTime: number = 0;
  let playbackState: "idle" | "playing" | "paused" = "idle";
  let playbackSpeed: number = 1.0;

  /** Time (ms) for a single event's full propagation at 1× speed. */
  const EVENT_DURATION_MS = 2000;

  /** Whether to auto-advance to the next event when propagation completes. */
  let autoAdvance: boolean = true;

  // ---------------------------------------------------------------------------
  // Three.js State
  // ---------------------------------------------------------------------------
  let containerEl: HTMLDivElement;
  let renderer: THREE.WebGLRenderer | null = null;
  let scene: THREE.Scene | null = null;
  let camera: THREE.PerspectiveCamera | null = null;
  let controls: OrbitControls | null = null;
  let animFrameId: number = 0;
  let lastFrameTime: number = 0;

  let detectorGroup: THREE.Group;
  let eventGroup: THREE.Group;

  // Animated object references for time-of-flight updates.
  interface AnimatedTrack {
    line: THREE.Line;
    totalLength: number;
  }
  interface AnimatedJet {
    cone: THREE.Mesh;
    wire: THREE.Mesh;
    fullScale: THREE.Vector3;
  }
  interface AnimatedArrow {
    group: THREE.Group;
  }

  let animatedTracks: AnimatedTrack[] = [];
  let animatedJets: AnimatedJet[] = [];
  let animatedMET: AnimatedArrow | null = null;

  // ---------------------------------------------------------------------------
  // Derived State
  // ---------------------------------------------------------------------------
  $: currentEvent = eventBatch.length > 0 ? eventBatch[currentEventIdx] : null;
  $: batchInfo = eventBatch.length > 0
    ? `Event ${currentEventIdx + 1} / ${eventBatch.length}`
    : "No events";

  // ---------------------------------------------------------------------------
  // Detector Geometry
  // ---------------------------------------------------------------------------
  const DETECTOR_RADIUS = 5;
  const DETECTOR_HALF_LENGTH = 8;

  function buildDetector(): THREE.Group {
    const group = new THREE.Group();
    const barrelGeo = new THREE.CylinderGeometry(
      DETECTOR_RADIUS, DETECTOR_RADIUS, DETECTOR_HALF_LENGTH * 2, 32, 1, true,
    );
    const wireMat = new THREE.MeshBasicMaterial({
      color: 0x334455, wireframe: true, transparent: true, opacity: 0.15,
    });
    const barrel = new THREE.Mesh(barrelGeo, wireMat);
    barrel.rotation.x = Math.PI / 2;
    group.add(barrel);

    const endcapGeo = new THREE.RingGeometry(0, DETECTOR_RADIUS, 32);
    const endcapMat = new THREE.MeshBasicMaterial({
      color: 0x334455, wireframe: true, transparent: true, opacity: 0.1, side: THREE.DoubleSide,
    });
    const front = new THREE.Mesh(endcapGeo, endcapMat);
    front.position.z = DETECTOR_HALF_LENGTH;
    group.add(front);
    const back = new THREE.Mesh(endcapGeo, endcapMat);
    back.position.z = -DETECTOR_HALF_LENGTH;
    group.add(back);

    const axes = new THREE.AxesHelper(DETECTOR_RADIUS * 1.3);
    (axes.material as THREE.Material).opacity = 0.2;
    (axes.material as THREE.Material).transparent = true;
    group.add(axes);

    return group;
  }

  // ---------------------------------------------------------------------------
  // Event Rendering Helpers
  // ---------------------------------------------------------------------------

  function etaPhiToDir(eta: number, phi: number, len: number): THREE.Vector3 {
    const theta = 2 * Math.atan(Math.exp(-eta));
    return new THREE.Vector3(
      len * Math.sin(theta) * Math.cos(phi),
      len * Math.sin(theta) * Math.sin(phi),
      len * Math.cos(theta),
    );
  }

  function vec3ToThree(v: Vec3): THREE.Vector3 {
    return new THREE.Vector3(v.x, v.y, v.z);
  }

  /** Add an animated cone for a jet. Starts at scale 0; grows with simTime. */
  function addJet(group: THREE.Group, jet: DisplayJet): void {
    const energyScale = Math.min(jet.energy / 50, 1);
    const coneLen = 1.5 + 3 * energyScale;
    const coneRad = 0.3 + 0.5 * energyScale;
    const dir = etaPhiToDir(jet.eta, jet.phi, coneLen);

    const geo = new THREE.ConeGeometry(coneRad, coneLen, 16, 1, true);
    const mat = new THREE.MeshBasicMaterial({
      color: 0xff8800, transparent: true, opacity: 0.35, side: THREE.DoubleSide,
    });
    const cone = new THREE.Mesh(geo, mat);
    cone.position.copy(dir.clone().multiplyScalar(0.5));
    cone.lookAt(dir);
    cone.rotateX(Math.PI / 2);
    const fullScale = cone.scale.clone();
    cone.scale.set(0, 0, 0);
    group.add(cone);

    const wireGeo = new THREE.ConeGeometry(coneRad, coneLen, 16, 1, true);
    const wireMat = new THREE.MeshBasicMaterial({
      color: 0xffaa44, wireframe: true, transparent: true, opacity: 0.5,
    });
    const wire = new THREE.Mesh(wireGeo, wireMat);
    wire.position.copy(cone.position);
    wire.rotation.copy(cone.rotation);
    wire.scale.set(0, 0, 0);
    group.add(wire);

    animatedJets.push({ cone, wire, fullScale });
  }

  /** Add an animated track line. Uses setDrawRange for time-of-flight. */
  function addTrack(
    group: THREE.Group,
    track: DisplayTrack,
    color: number,
    dashed: boolean = false,
  ): void {
    const len = Math.min(2 + track.energy / 20, DETECTOR_RADIUS * 0.9);
    const dir = vec3ToThree(track.direction).normalize().multiplyScalar(len);

    // Subdivide into many segments for smooth draw-range animation.
    const segCount = 64;
    const positions: number[] = [];
    for (let i = 0; i <= segCount; i++) {
      const t = i / segCount;
      positions.push(dir.x * t, dir.y * t, dir.z * t);
    }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
    geo.setDrawRange(0, 0);

    let mat: THREE.Material;
    if (dashed) {
      mat = new THREE.LineDashedMaterial({
        color, dashSize: 0.15, gapSize: 0.1, transparent: true, opacity: 0.85,
      });
    } else {
      mat = new THREE.LineBasicMaterial({
        color, linewidth: 2, transparent: true, opacity: 0.9,
      });
    }

    const line = new THREE.Line(geo, mat);
    if (dashed) line.computeLineDistances();
    group.add(line);

    animatedTracks.push({ line, totalLength: segCount + 1 });
  }

  /** Add an animated MET arrow. Scales from 0 to full with simTime. */
  function addMET(group: THREE.Group, met: DisplayMET): void {
    if (met.magnitude < 1) return;
    const scale = Math.min(met.magnitude / 30, DETECTOR_RADIUS * 0.8);
    const dir = vec3ToThree(met.direction).normalize().multiplyScalar(scale);
    const arrowDir = dir.clone().normalize();

    const metGroup = new THREE.Group();
    const arrow = new THREE.ArrowHelper(
      arrowDir, new THREE.Vector3(0, 0, 0), scale, 0xff00ff, 0.4, 0.2,
    );
    metGroup.add(arrow);

    const pts = [new THREE.Vector3(0, 0, 0), dir];
    const geo = new THREE.BufferGeometry().setFromPoints(pts);
    const mat = new THREE.LineDashedMaterial({
      color: 0xff00ff, dashSize: 0.2, gapSize: 0.15, transparent: true, opacity: 0.7,
    });
    const line = new THREE.Line(geo, mat);
    line.computeLineDistances();
    metGroup.add(line);
    metGroup.scale.set(0, 0, 0);
    group.add(metGroup);

    animatedMET = { group: metGroup };
  }

  /** Build all animated objects for a given event. */
  function buildEventObjects(data: EventDisplayData): void {
    clearEventObjects();
    for (const jet of data.jets) addJet(eventGroup, jet);
    for (const e of data.electrons) addTrack(eventGroup, e, 0x44ff44);
    for (const mu of data.muons) addTrack(eventGroup, mu, 0xff4444);
    for (const ph of data.photons) addTrack(eventGroup, ph, 0xffff44, true);
    if (data.met) addMET(eventGroup, data.met);
  }

  function clearEventObjects(): void {
    while (eventGroup.children.length) {
      eventGroup.remove(eventGroup.children[0]);
    }
    animatedTracks = [];
    animatedJets = [];
    animatedMET = null;
  }

  /** Update animated objects based on current simTime ∈ [0, 1]. */
  function updateTimeOfFlight(t: number): void {
    const progress = Math.max(0, Math.min(1, t));

    for (const at of animatedTracks) {
      const count = Math.floor(progress * at.totalLength);
      at.line.geometry.setDrawRange(0, count);
    }

    for (const aj of animatedJets) {
      aj.cone.scale.set(
        aj.fullScale.x * progress, aj.fullScale.y * progress, aj.fullScale.z * progress,
      );
      aj.wire.scale.set(
        aj.fullScale.x * progress, aj.fullScale.y * progress, aj.fullScale.z * progress,
      );
    }

    if (animatedMET) {
      animatedMET.group.scale.set(progress, progress, progress);
    }
  }

  // ---------------------------------------------------------------------------
  // Batch Generation
  // ---------------------------------------------------------------------------
  async function handleGenerateBatch(): Promise<void> {
    loading = true;
    errorMsg = "";
    eventBatch = [];
    currentEventIdx = 0;
    simTime = 0;
    playbackState = "idle";
    clearEventObjects();

    if (cmsEnergy <= 0) {
      errorMsg = "CMS energy must be positive.";
      loading = false;
      return;
    }

    const finalMasses = Array(nFinal).fill(0.0);

    try {
      appendLog(
        `Generating ${batchSize} events: √s = ${cmsEnergy} GeV, ${nFinal}-body, ${detectorPreset}`,
      );

      const batch = await generateDisplayBatch(
        cmsEnergy, finalMasses, detectorPreset, batchSize,
      );

      eventBatch = batch;
      currentEventIdx = 0;
      simTime = 0;

      if (batch.length > 0) {
        buildEventObjects(batch[0]);
        updateTimeOfFlight(0);
      }

      appendLog(`Batch ready: ${batch.length} events buffered for playback`);
    } catch (err) {
      errorMsg = String(err);
      appendLog(`Batch generation error: ${errorMsg}`);
    } finally {
      loading = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Playback Controls
  // ---------------------------------------------------------------------------
  function play(): void {
    if (eventBatch.length === 0) return;
    playbackState = "playing";
    lastFrameTime = performance.now();
  }

  function pause(): void {
    playbackState = "paused";
  }

  function stepForward(): void {
    if (eventBatch.length === 0) return;
    playbackState = "paused";
    goToEvent((currentEventIdx + 1) % eventBatch.length);
  }

  function stepBack(): void {
    if (eventBatch.length === 0) return;
    playbackState = "paused";
    goToEvent(currentEventIdx > 0 ? currentEventIdx - 1 : eventBatch.length - 1);
  }

  function goToEvent(idx: number): void {
    currentEventIdx = idx;
    simTime = 1;
    buildEventObjects(eventBatch[idx]);
    updateTimeOfFlight(1);
  }

  // ---------------------------------------------------------------------------
  // Three.js Lifecycle & Animation Loop
  // ---------------------------------------------------------------------------
  function initScene(): void {
    if (!containerEl) return;

    scene = new THREE.Scene();
    scene.background = new THREE.Color(0x111118);

    camera = new THREE.PerspectiveCamera(
      50, containerEl.clientWidth / containerEl.clientHeight, 0.1, 100,
    );
    camera.position.set(8, 6, 10);
    camera.lookAt(0, 0, 0);

    renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(window.devicePixelRatio);
    renderer.setSize(containerEl.clientWidth, containerEl.clientHeight);
    containerEl.appendChild(renderer.domElement);

    controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.minDistance = 3;
    controls.maxDistance = 30;

    scene.add(new THREE.AmbientLight(0xffffff, 0.4));
    const dirLight = new THREE.DirectionalLight(0xffffff, 0.6);
    dirLight.position.set(5, 10, 7);
    scene.add(dirLight);

    detectorGroup = buildDetector();
    scene.add(detectorGroup);
    eventGroup = new THREE.Group();
    scene.add(eventGroup);

    lastFrameTime = performance.now();

    function animate(now: number): void {
      animFrameId = requestAnimationFrame(animate);
      controls?.update();

      if (playbackState === "playing" && eventBatch.length > 0) {
        const dt = (now - lastFrameTime) / EVENT_DURATION_MS;
        simTime += dt * playbackSpeed;

        if (simTime >= 1.0) {
          simTime = 1.0;
          updateTimeOfFlight(1);

          if (autoAdvance) {
            const nextIdx = (currentEventIdx + 1) % eventBatch.length;
            currentEventIdx = nextIdx;
            simTime = 0;
            buildEventObjects(eventBatch[nextIdx]);
            updateTimeOfFlight(0);
          } else {
            playbackState = "paused";
          }
        } else {
          updateTimeOfFlight(simTime);
        }
      }

      lastFrameTime = now;

      if (renderer && scene && camera) {
        renderer.render(scene, camera);
      }
    }
    animate(performance.now());
  }

  function handleResize(): void {
    if (!containerEl || !camera || !renderer) return;
    camera.aspect = containerEl.clientWidth / containerEl.clientHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(containerEl.clientWidth, containerEl.clientHeight);
  }

  let resizeObserver: ResizeObserver | null = null;

  onMount(() => {
    initScene();
    // Use ResizeObserver for container-level resize (canvas mode, docking splits)
    resizeObserver = new ResizeObserver(() => handleResize());
    if (containerEl) resizeObserver.observe(containerEl);
    window.addEventListener("resize", handleResize);
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    window.removeEventListener("resize", handleResize);
    cancelAnimationFrame(animFrameId);
    controls?.dispose();
    renderer?.dispose();
    if (renderer && containerEl?.contains(renderer.domElement)) {
      containerEl.removeChild(renderer.domElement);
    }
  });
</script>

<!-- ======================================================================= -->
<!-- Template                                                                 -->
<!-- ======================================================================= -->

<div class="event-display-widget">
  <!-- Config Panel -->
  <div class="config-panel">
    <div class="field-row">
      <div class="field compact">
        <label for="ed-energy">√s (GeV):</label>
        <SpireNumberInput inputId="ed-energy" bind:value={cmsEnergy} min={10} step={0.5} ariaLabel="Centre-of-mass energy" />
      </div>
      <div class="field compact">
        <label for="ed-nfinal">Final N:</label>
        <SpireNumberInput inputId="ed-nfinal" bind:value={nFinal} min={2} max={8} step={1} ariaLabel="Final-state multiplicity" />
      </div>
      <div class="field compact">
        <label for="ed-detector">Detector:</label>
        <select id="ed-detector" bind:value={detectorPreset}>
          <option value="lhc_like">LHC-like</option>
          <option value="ilc_like">ILC-like</option>
          <option value="perfect">Perfect</option>
        </select>
      </div>
      <div class="field compact">
        <label for="ed-batch">Batch:</label>
        <SpireNumberInput inputId="ed-batch" bind:value={batchSize} min={1} max={100} step={1} ariaLabel="Batch size" />
      </div>
    </div>

    <div class="run-row">
      <button class="run-btn" on:click={handleGenerateBatch} disabled={loading}>
        {#if loading}
          Generating…
        {:else}
          Generate Batch
        {/if}
      </button>
    </div>

    {#if errorMsg}
      <div class="error-box">{errorMsg}</div>
    {/if}

    <!-- Playback Controls -->
    {#if eventBatch.length > 0}
      <div class="playback-bar">
        <button class="ctrl-btn" on:click={stepBack} use:tooltip={{ text: "Previous Event" }}>⏮</button>
        {#if playbackState === "playing"}
          <button class="ctrl-btn ctrl-pause" on:click={pause} use:tooltip={{ text: "Pause" }}>⏸</button>
        {:else}
          <button class="ctrl-btn ctrl-play" on:click={play} use:tooltip={{ text: "Play" }}>▶</button>
        {/if}
        <button class="ctrl-btn" on:click={stepForward} use:tooltip={{ text: "Next Event" }}>⏭</button>

        <div class="speed-control">
          <label for="ed-speed">Speed:</label>
          <SpireSlider inputId="ed-speed" min={0.25} max={4} step={0.25} bind:value={playbackSpeed} ariaLabel="Playback speed" />
          <span class="speed-label">{playbackSpeed.toFixed(2)}×</span>
        </div>

        <label class="auto-advance-label">
          <input type="checkbox" bind:checked={autoAdvance} />
          Loop
        </label>

        <span class="event-index">{batchInfo}</span>
      </div>

      <!-- Time progress bar -->
      <div class="time-bar">
        <div class="time-fill" style="width: {simTime * 100}%"></div>
      </div>
    {/if}

    {#if currentEvent}
      <div class="event-summary">
        <span class="badge jet">{currentEvent.jets.length} jets</span>
        <span class="badge electron">{currentEvent.electrons.length} e⁻</span>
        <span class="badge muon">{currentEvent.muons.length} μ</span>
        <span class="badge photon">{currentEvent.photons.length} γ</span>
        <span class="badge met">MET {currentEvent.met.magnitude.toFixed(1)} GeV</span>
      </div>
    {/if}
  </div>

  <!-- 3D Viewport -->
  <div class="viewport" bind:this={containerEl} data-wheel-capture>
    {#if eventBatch.length === 0 && !loading}
      <div class="placeholder">
        Click <strong>Generate Batch</strong> to produce events for animated playback.
      </div>
    {/if}
  </div>

  <!-- Legend -->
  <div class="legend">
    <span class="legend-item" style="color: #ff8800;">■ Jets</span>
    <span class="legend-item" style="color: #44ff44;">- Electrons</span>
    <span class="legend-item" style="color: #ff4444;">- Muons</span>
    <span class="legend-item" style="color: #ffff44;">┅ Photons</span>
    <span class="legend-item" style="color: #ff00ff;">⇢ MET</span>
  </div>
</div>

<!-- ======================================================================= -->
<!-- Styles                                                                   -->
<!-- ======================================================================= -->

<style>
  .event-display-widget {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    height: 100%;
    overflow: hidden;
    font-size: 0.85rem;
  }

  .config-panel {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.5rem;
    background: rgba(255, 255, 255, 0.03);
    border-radius: 6px;
    flex-shrink: 0;
  }

  .field-row { display: flex; gap: 0.5rem; flex-wrap: wrap; }
  .field { display: flex; flex-direction: column; gap: 0.15rem; }
  .field.compact { flex: 1; min-width: 70px; }
  .field label { color: #aaa; font-size: 0.75rem; }

  select {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 4px;
    color: #e0e0e0;
    padding: 0.25rem 0.4rem;
    font-size: 0.8rem;
  }

  .run-row { display: flex; gap: 0.5rem; align-items: center; }

  .run-btn {
    padding: 0.35rem 1rem;
    font-size: 0.82rem;
    font-weight: 600;
    border: none;
    border-radius: 5px;
    background: linear-gradient(135deg, #1e88e5, #1565c0);
    color: #fff;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .run-btn:hover:not(:disabled) { opacity: 0.9; }
  .run-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .error-box {
    padding: 0.3rem 0.5rem;
    background: rgba(255, 50, 50, 0.12);
    border: 1px solid rgba(255, 50, 50, 0.3);
    border-radius: 4px;
    color: #ef5350;
    font-size: 0.78rem;
    word-break: break-word;
  }

  /* --- Playback Controls --- */
  .playback-bar {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
    padding: 0.3rem 0;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  .ctrl-btn {
    width: 2rem; height: 2rem; font-size: 1rem;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.05);
    color: #ccc; cursor: pointer;
    display: flex; align-items: center; justify-content: center;
    transition: background 0.12s;
  }
  .ctrl-btn:hover { background: rgba(255, 255, 255, 0.12); color: #fff; }
  .ctrl-play { color: #4caf50; border-color: rgba(76, 175, 80, 0.3); }
  .ctrl-pause { color: #ff9800; border-color: rgba(255, 152, 0, 0.3); }

  .speed-control {
    display: flex; align-items: center; gap: 0.3rem; margin-left: 0.5rem;
  }
  .speed-control label { font-size: 0.72rem; color: #888; }
  .speed-control :global(.spire-slider) { width: 80px; }
  .speed-label {
    font-size: 0.72rem; color: #8ab4f8;
    font-family: "JetBrains Mono", monospace; min-width: 3em;
  }

  .auto-advance-label {
    display: flex; align-items: center; gap: 0.2rem;
    font-size: 0.72rem; color: #888; margin-left: 0.5rem; cursor: pointer;
  }
  .auto-advance-label input[type="checkbox"] { accent-color: #8ab4f8; }

  .event-index {
    margin-left: auto; font-size: 0.75rem; color: #8ab4f8;
    font-family: "JetBrains Mono", monospace;
  }

  /* --- Time progress bar --- */
  .time-bar {
    height: 3px; background: rgba(255, 255, 255, 0.08);
    border-radius: 2px; overflow: hidden;
  }
  .time-fill {
    height: 100%;
    background: linear-gradient(90deg, #1e88e5, #42a5f5);
    border-radius: 2px;
    transition: width 50ms linear;
  }

  /* --- Event Summary --- */
  .event-summary { display: flex; gap: 0.4rem; flex-wrap: wrap; }
  .badge {
    padding: 0.15rem 0.5rem; border-radius: 3px;
    font-size: 0.72rem; font-weight: 600;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }
  .badge.jet      { background: rgba(255, 136, 0, 0.15); color: #ff8800; }
  .badge.electron { background: rgba(68, 255, 68, 0.12); color: #44ff44; }
  .badge.muon     { background: rgba(255, 68, 68, 0.12); color: #ff4444; }
  .badge.photon   { background: rgba(255, 255, 68, 0.12); color: #ffff44; }
  .badge.met      { background: rgba(255, 0, 255, 0.12); color: #ff00ff; }

  /* --- Viewport --- */
  .viewport {
    position: relative; flex: 1; min-height: 250px;
    border-radius: 6px; overflow: hidden; background: #111118;
  }
  .viewport :global(canvas) { display: block; width: 100% !important; height: 100% !important; }
  .placeholder {
    position: absolute; inset: 0;
    display: flex; align-items: center; justify-content: center;
    color: #555; font-size: 0.85rem; text-align: center;
    padding: 1rem; pointer-events: none;
  }

  .legend {
    display: flex; gap: 0.8rem; justify-content: center;
    padding: 0.25rem 0; font-size: 0.72rem; flex-shrink: 0;
  }
  .legend-item { font-family: "JetBrains Mono", "Fira Code", monospace; opacity: 0.8; }
</style>
