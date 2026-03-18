<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import type { ElementData, IsotopeData } from "$lib/core/physics/nuclearDataLoader";

  export let element: ElementData | null = null;
  export let isotope:
    | {
        Z: number;
        A: number;
        symbol: string;
        name: string;
        isotopeData: IsotopeData;
        element: ElementData;
      }
    | null = null;
  export let subtitle = "";

  interface Point3D {
    x: number;
    y: number;
    z: number;
  }

  let hostEl: HTMLDivElement | null = null;
  let canvasEl: HTMLCanvasElement | null = null;
  let webglReady = false;
  let webglError = "";
  let disposed = false;

  let scene: any = null;
  let camera: any = null;
  let renderer: any = null;
  let controls: any = null;
  let modelRoot: any = null;
  let ThreeLib: typeof import("three") | null = null;
  let frameHandle: number | null = null;
  let resizeObserver: ResizeObserver | null = null;

  let modelSignature = "";

  function activeElement(): ElementData | null {
    return isotope?.element ?? element;
  }

  function atomicNumber(): number {
    return isotope?.Z ?? activeElement()?.Z ?? 0;
  }

  function massNumber(): number {
    return isotope?.A ?? Math.max(1, Math.round(activeElement()?.atomic_mass ?? 1));
  }

  function parseShellOccupancy(configuration: string | undefined, z: number): number[] {
    const shellCounts = new Map<number, number>();
    const matches = configuration?.matchAll(/(\d+)([spdf])(\d+)/g) ?? [];
    let counted = 0;

    for (const match of matches) {
      const shell = Number.parseInt(match[1] ?? "0", 10);
      const occupancy = Number.parseInt(match[3] ?? "0", 10);
      if (!Number.isFinite(shell) || !Number.isFinite(occupancy) || shell <= 0 || occupancy <= 0) continue;
      shellCounts.set(shell, (shellCounts.get(shell) ?? 0) + occupancy);
      counted += occupancy;
    }

    if (counted === 0) {
      const fallbackCaps = [2, 8, 18, 32, 32, 18, 8];
      const fallback: number[] = [];
      let remaining = Math.max(0, z);
      for (const cap of fallbackCaps) {
        if (remaining <= 0) break;
        const fill = Math.min(cap, remaining);
        fallback.push(fill);
        remaining -= fill;
      }
      return fallback;
    }

    const ordered = Array.from(shellCounts.entries())
      .sort((a, b) => a[0] - b[0])
      .map(([, count]) => count);
    const total = ordered.reduce((sum, value) => sum + value, 0);
    if (total >= z) return ordered;

    const next = [...ordered];
    const caps = [2, 8, 18, 32, 32, 18, 8];
    let remaining = z - total;
    for (let index = 0; remaining > 0; index += 1) {
      const cap = caps[index] ?? 2;
      const current = next[index] ?? 0;
      const add = Math.min(cap - current, remaining);
      if (add > 0) {
        next[index] = current + add;
        remaining -= add;
      }
    }

    return next;
  }

  function hashSeed(input: string): number {
    let hash = 2166136261;
    for (let index = 0; index < input.length; index += 1) {
      hash ^= input.charCodeAt(index);
      hash = Math.imul(hash, 16777619);
    }
    return hash >>> 0;
  }

  function seededRandom(seed: number): () => number {
    let state = seed || 1;
    return () => {
      state ^= state << 13;
      state ^= state >>> 17;
      state ^= state << 5;
      return ((state >>> 0) & 0xffffffff) / 0x100000000;
    };
  }

  function seededNucleonSequence(protons: number, neutrons: number, seedKey: string): ("proton" | "neutron")[] {
    const sequence: ("proton" | "neutron")[] = [
      ...Array.from({ length: protons }, () => "proton" as const),
      ...Array.from({ length: neutrons }, () => "neutron" as const),
    ];
    const random = seededRandom(hashSeed(seedKey));
    for (let index = sequence.length - 1; index > 0; index -= 1) {
      const swapIndex = Math.floor(random() * (index + 1));
      const tmp = sequence[index];
      sequence[index] = sequence[swapIndex];
      sequence[swapIndex] = tmp;
    }
    return sequence;
  }

  function nucleonRadiusFor(A: number): number {
    if (A <= 4) return 0.38;
    if (A <= 12) return 0.32;
    if (A <= 32) return 0.27;
    return 0.22;
  }

  function packedNucleonPoints(total: number, nucleonRadius: number): Point3D[] {
    if (total <= 0) return [];
    if (total === 1) return [{ x: 0, y: 0, z: 0 }];

    const contact = nucleonRadius * 2 * 0.98;
    if (total === 2) {
      return [
        { x: -contact * 0.5, y: 0, z: 0 },
        { x: contact * 0.5, y: 0, z: 0 },
      ];
    }
    if (total === 3) {
      const r = contact / Math.sqrt(3);
      return [
        { x: 0, y: r, z: 0 },
        { x: -contact * 0.5, y: -r * 0.5, z: 0 },
        { x: contact * 0.5, y: -r * 0.5, z: 0 },
      ];
    }
    if (total === 4) {
      const r = contact * 0.58;
      return [
        { x: r, y: r, z: r },
        { x: -r, y: -r, z: r },
        { x: -r, y: r, z: -r },
        { x: r, y: -r, z: -r },
      ];
    }

    const diameter = contact;
    const points: Point3D[] = [{ x: 0, y: 0, z: 0 }];
    let remaining = total - 1;
    let shell = 1;

    while (remaining > 0) {
      const shellCapacity = Math.max(8, Math.round(12 * shell * shell));
      const count = Math.min(shellCapacity, remaining);
      const shellRadius = diameter * shell * 0.84;

      for (let index = 0; index < count; index += 1) {
        const u = (index + 0.5) / count;
        const theta = index * Math.PI * (3 - Math.sqrt(5));
        const phi = Math.acos(1 - 2 * u);
        points.push({
          x: Math.cos(theta) * Math.sin(phi) * shellRadius,
          y: Math.sin(theta) * Math.sin(phi) * shellRadius,
          z: Math.cos(phi) * shellRadius,
        });
      }

      remaining -= count;
      shell += 1;
    }

    return points.slice(0, total);
  }

  function electronCloud(shells: number[]): Point3D[] {
    const points: Point3D[] = [];
    shells.forEach((count, shellIndex) => {
      const radius = 2.3 + shellIndex * 0.95;
      for (let index = 0; index < count; index += 1) {
        const angle = (index / Math.max(1, count)) * Math.PI * 2;
        points.push({
          x: Math.cos(angle) * radius,
          y: Math.sin(angle) * radius,
          z: 0,
        });
      }
    });
    return points;
  }

  function makeRingPoints(radius: number, segments = 160): Point3D[] {
    const points: Point3D[] = [];
    for (let index = 0; index <= segments; index += 1) {
      const angle = (index / segments) * Math.PI * 2;
      points.push({ x: Math.cos(angle) * radius, y: Math.sin(angle) * radius, z: 0 });
    }
    return points;
  }

  function clearModel(): void {
    if (!modelRoot) return;
    const children = [...modelRoot.children];
    for (const child of children) {
      modelRoot.remove(child);
      if (child.geometry) child.geometry.dispose?.();
      if (Array.isArray(child.material)) {
        child.material.forEach((mat: any) => mat.dispose?.());
      } else {
        child.material?.dispose?.();
      }
    }
  }

  function rebuildModel(): void {
    if (!webglReady || !scene || !modelRoot || !ThreeLib) return;

    const THREE = ThreeLib;

    const z = atomicNumber();
    const a = massNumber();
    const n = Math.max(0, a - z);
    const shells = activeElement() ? parseShellOccupancy(activeElement()?.electron_configuration, z) : [];

    clearModel();

    if (z <= 0 || a <= 0) return;

    const nucleonRadius = nucleonRadiusFor(a);
    const nucleonPoints = packedNucleonPoints(a, nucleonRadius);
    const sequence = seededNucleonSequence(z, n, `${activeElement()?.symbol ?? "X"}:${z}:${a}`);

    const protonMaterial = new THREE.MeshStandardMaterial({ color: 0xd85d5d, roughness: 0.35, metalness: 0.12 });
    const neutronMaterial = new THREE.MeshStandardMaterial({ color: 0xe8edf3, roughness: 0.4, metalness: 0.05 });
    const nucleonGeometry = new THREE.SphereGeometry(nucleonRadius, 22, 22);

    const protonCount = sequence.filter((entry) => entry === "proton").length;
    const neutronCount = sequence.length - protonCount;
    const protonMesh = new THREE.InstancedMesh(nucleonGeometry, protonMaterial, protonCount);
    const neutronMesh = new THREE.InstancedMesh(nucleonGeometry, neutronMaterial, neutronCount);

    let protonIndex = 0;
    let neutronIndex = 0;
    const matrix = new THREE.Matrix4();
    nucleonPoints.forEach((point, index) => {
      matrix.makeTranslation(point.x, point.y, point.z);
      if (sequence[index] === "proton") {
        protonMesh.setMatrixAt(protonIndex, matrix);
        protonIndex += 1;
      } else {
        neutronMesh.setMatrixAt(neutronIndex, matrix);
        neutronIndex += 1;
      }
    });
    protonMesh.instanceMatrix.needsUpdate = true;
    neutronMesh.instanceMatrix.needsUpdate = true;
    modelRoot.add(protonMesh);
    modelRoot.add(neutronMesh);

    const electronPoints = electronCloud(shells);
    if (electronPoints.length > 0) {
      const electronGeometry = new THREE.SphereGeometry(0.092, 14, 14);
      const electronMaterial = new THREE.MeshStandardMaterial({ color: 0xb5bec9, roughness: 0.25, metalness: 0.2 });
      const electronMesh = new THREE.InstancedMesh(electronGeometry, electronMaterial, electronPoints.length);

      electronPoints.forEach((point, index) => {
        matrix.makeTranslation(point.x, point.y, point.z);
        electronMesh.setMatrixAt(index, matrix);
      });
      electronMesh.instanceMatrix.needsUpdate = true;
      modelRoot.add(electronMesh);
    }

    const orbitMaterial = new THREE.LineBasicMaterial({
      color: 0x98a7b8,
      transparent: true,
      opacity: 0.42,
    });

    shells.forEach((_, shellIndex) => {
      const radius = 2.3 + shellIndex * 0.95;
      const points = makeRingPoints(radius).map((point) => new THREE.Vector3(point.x, point.y, point.z));
      const geometry = new THREE.BufferGeometry().setFromPoints(points);
      const line = new THREE.LineLoop(geometry, orbitMaterial.clone());
      modelRoot.add(line);
    });
  }

  function updateViewportSize(): void {
    if (!hostEl || !renderer || !camera) return;
    const width = hostEl.clientWidth;
    const height = Math.max(220, Math.round(width * 0.62));
    renderer.setSize(width, height, false);
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
  }

  function animate(): void {
    if (disposed || !renderer || !scene || !camera) return;
    controls?.update?.();
    renderer.render(scene, camera);
    frameHandle = requestAnimationFrame(animate);
  }

  function setupResizeObserver(): void {
    if (!hostEl) return;
    resizeObserver = new ResizeObserver(() => updateViewportSize());
    resizeObserver.observe(hostEl);
  }

  onMount(async () => {
    if (!hostEl || !canvasEl) return;
    try {
      const THREE = await import("three");
      const controlsModule = await import("three/examples/jsm/controls/OrbitControls.js");
      if (disposed) return;

      ThreeLib = THREE;
      const { OrbitControls } = controlsModule;

      scene = new ThreeLib.Scene();
      camera = new THREE.PerspectiveCamera(36, 1, 0.1, 100);
      camera.position.set(0, 0, 16.8);

      renderer = new THREE.WebGLRenderer({
        canvas: canvasEl,
        antialias: true,
        alpha: true,
        powerPreference: "high-performance",
      });
      renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.8));
      renderer.outputColorSpace = THREE.SRGBColorSpace;

      const ambient = new ThreeLib.AmbientLight(0xffffff, 0.66);
      const key = new ThreeLib.DirectionalLight(0xffffff, 0.74);
      key.position.set(8, 9, 10);
      const fill = new ThreeLib.DirectionalLight(0xffffff, 0.32);
      fill.position.set(-8, -6, 7);
      scene.add(ambient, key, fill);

      modelRoot = new ThreeLib.Group();
      scene.add(modelRoot);

      controls = new OrbitControls(camera, renderer.domElement);
      controls.enablePan = false;
      controls.enableDamping = true;
      controls.dampingFactor = 0.08;
      controls.minDistance = 6;
      controls.maxDistance = 28;

      webglReady = true;
      updateViewportSize();
      rebuildModel();
      setupResizeObserver();
      animate();
    } catch (error) {
      webglError = error instanceof Error ? error.message : "WebGL renderer initialization failed";
      webglReady = false;
    }
  });

  onDestroy(() => {
    disposed = true;
    if (frameHandle !== null) cancelAnimationFrame(frameHandle);
    resizeObserver?.disconnect();
    clearModel();
    controls?.dispose?.();
    renderer?.dispose?.();
  });

  $: sceneLabel = subtitle
    ? `${activeElement()?.name ?? "selected element"} | ${subtitle}`
    : `${activeElement()?.name ?? "selected element"}`;
  $: modelSignature = `${activeElement()?.symbol ?? "none"}:${atomicNumber()}:${massNumber()}:${activeElement()?.electron_configuration ?? "none"}:${isotope?.isotopeData.spin_parity ?? ""}`;
  $: if (webglReady && modelSignature) rebuildModel();
</script>

<section class="atomic-model-card" aria-label={sceneLabel}>
  <div class="atomic-viewport-shell" bind:this={hostEl} role="application">
    {#if webglError}
      <div class="atomic-error">3D renderer unavailable: {webglError}</div>
    {:else}
      <canvas class="atomic-canvas" bind:this={canvasEl} aria-label={sceneLabel}></canvas>
    {/if}
  </div>
</section>

<style>
  .atomic-model-card {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.28rem;
  }

  .atomic-viewport-shell {
    width: 100%;
    min-height: 220px;
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.16);
    background: var(--color-bg-inset, var(--bg-inset));
    touch-action: none;
  }

  .atomic-canvas {
    width: 100%;
    height: 100%;
    display: block;
    cursor: grab;
  }

  .atomic-canvas:active {
    cursor: grabbing;
  }

  .atomic-error {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 220px;
    padding: 0.4rem;
    color: var(--color-text-muted, var(--fg-secondary));
    font-family: var(--font-mono);
    font-size: 0.62rem;
    text-align: center;
  }
</style>
