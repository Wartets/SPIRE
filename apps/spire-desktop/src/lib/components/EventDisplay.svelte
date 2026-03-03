<!--
  SPIRE — 3D Event Display

  Interactive Three.js-powered visualization of particle physics events.
  Shows a wireframe detector barrel with jet cones, lepton tracks,
  photon lines, and missing transverse energy (MET) arrows.

  Uses the Tauri `generate_display_event` command to produce a single
  Monte-Carlo event, run it through the detector simulation, and
  render the reconstructed objects in 3D.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { appendLog } from "$lib/stores/physicsStore";
  import { generateDisplayEvent } from "$lib/api";
  import type {
    EventDisplayData,
    DisplayJet,
    DisplayTrack,
    DisplayMET,
    Vec3,
    DetectorPreset,
  } from "$lib/types/spire";
  import * as THREE from "three";
  import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";

  // ---------------------------------------------------------------------------
  // Configuration State
  // ---------------------------------------------------------------------------
  let cmsEnergy: number = 200;
  let nFinal: number = 4;
  let detectorPreset: DetectorPreset = "lhc_like";
  let loading: boolean = false;
  let errorMsg: string = "";
  let eventData: EventDisplayData | null = null;

  // ---------------------------------------------------------------------------
  // Three.js State
  // ---------------------------------------------------------------------------
  let containerEl: HTMLDivElement;
  let renderer: THREE.WebGLRenderer | null = null;
  let scene: THREE.Scene | null = null;
  let camera: THREE.PerspectiveCamera | null = null;
  let controls: OrbitControls | null = null;
  let animFrameId: number = 0;

  // Groups for easy cleanup.
  let detectorGroup: THREE.Group;
  let eventGroup: THREE.Group;

  // ---------------------------------------------------------------------------
  // Detector Geometry
  // ---------------------------------------------------------------------------
  const DETECTOR_RADIUS = 5;
  const DETECTOR_HALF_LENGTH = 8;

  function buildDetector(): THREE.Group {
    const group = new THREE.Group();
    const barrelGeo = new THREE.CylinderGeometry(
      DETECTOR_RADIUS,
      DETECTOR_RADIUS,
      DETECTOR_HALF_LENGTH * 2,
      32,
      1,
      true,
    );
    const wireMat = new THREE.MeshBasicMaterial({
      color: 0x334455,
      wireframe: true,
      transparent: true,
      opacity: 0.15,
    });
    const barrel = new THREE.Mesh(barrelGeo, wireMat);
    barrel.rotation.x = Math.PI / 2; // align along Z
    group.add(barrel);

    // Endcaps (flat rings).
    const endcapGeo = new THREE.RingGeometry(0, DETECTOR_RADIUS, 32);
    const endcapMat = new THREE.MeshBasicMaterial({
      color: 0x334455,
      wireframe: true,
      transparent: true,
      opacity: 0.1,
      side: THREE.DoubleSide,
    });
    const front = new THREE.Mesh(endcapGeo, endcapMat);
    front.position.z = DETECTOR_HALF_LENGTH;
    group.add(front);
    const back = new THREE.Mesh(endcapGeo, endcapMat);
    back.position.z = -DETECTOR_HALF_LENGTH;
    group.add(back);

    // Axes helper (faint).
    const axes = new THREE.AxesHelper(DETECTOR_RADIUS * 1.3);
    (axes.material as THREE.Material).opacity = 0.2;
    (axes.material as THREE.Material).transparent = true;
    group.add(axes);

    return group;
  }

  // ---------------------------------------------------------------------------
  // Event Rendering Helpers
  // ---------------------------------------------------------------------------

  /** Convert physics η, φ, magnitude to a THREE.Vector3 direction scaled by len. */
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

  /** Add a cone for a jet. */
  function addJet(group: THREE.Group, jet: DisplayJet): void {
    const energyScale = Math.min(jet.energy / 50, 1);
    const coneLen = 1.5 + 3 * energyScale;
    const coneRad = 0.3 + 0.5 * energyScale;

    const dir = etaPhiToDir(jet.eta, jet.phi, coneLen);
    const geo = new THREE.ConeGeometry(coneRad, coneLen, 16, 1, true);
    const mat = new THREE.MeshBasicMaterial({
      color: 0xff8800,
      transparent: true,
      opacity: 0.35,
      wireframe: false,
      side: THREE.DoubleSide,
    });
    const cone = new THREE.Mesh(geo, mat);

    // Orient cone along direction.
    cone.position.copy(dir.clone().multiplyScalar(0.5));
    cone.lookAt(dir);
    cone.rotateX(Math.PI / 2);
    group.add(cone);

    // Wireframe outline.
    const wireGeo = new THREE.ConeGeometry(coneRad, coneLen, 16, 1, true);
    const wireMat = new THREE.MeshBasicMaterial({
      color: 0xffaa44,
      wireframe: true,
      transparent: true,
      opacity: 0.5,
    });
    const wire = new THREE.Mesh(wireGeo, wireMat);
    wire.position.copy(cone.position);
    wire.rotation.copy(cone.rotation);
    group.add(wire);
  }

  /** Add a track line for a lepton or photon. */
  function addTrack(
    group: THREE.Group,
    track: DisplayTrack,
    color: number,
    dashed: boolean = false,
  ): void {
    const len = Math.min(2 + track.energy / 20, DETECTOR_RADIUS * 0.9);
    const dir = vec3ToThree(track.direction).normalize().multiplyScalar(len);
    const points = [new THREE.Vector3(0, 0, 0), dir];
    const geo = new THREE.BufferGeometry().setFromPoints(points);

    let mat: THREE.Material;
    if (dashed) {
      mat = new THREE.LineDashedMaterial({
        color,
        dashSize: 0.15,
        gapSize: 0.1,
        linewidth: 1,
        transparent: true,
        opacity: 0.85,
      });
    } else {
      mat = new THREE.LineBasicMaterial({
        color,
        linewidth: 2,
        transparent: true,
        opacity: 0.9,
      });
    }

    const line = new THREE.Line(geo, mat);
    if (dashed) line.computeLineDistances();
    group.add(line);
  }

  /** Add a MET arrow (dashed, in the transverse plane). */
  function addMET(group: THREE.Group, met: DisplayMET): void {
    if (met.magnitude < 1) return;
    const scale = Math.min(met.magnitude / 30, DETECTOR_RADIUS * 0.8);
    const dir = vec3ToThree(met.direction).normalize().multiplyScalar(scale);

    const arrowDir = dir.clone().normalize();
    const arrow = new THREE.ArrowHelper(
      arrowDir,
      new THREE.Vector3(0, 0, 0),
      scale,
      0xff00ff,
      0.4,
      0.2,
    );
    group.add(arrow);

    // Dashed line along the arrow for visual distinction.
    const pts = [new THREE.Vector3(0, 0, 0), dir];
    const geo = new THREE.BufferGeometry().setFromPoints(pts);
    const mat = new THREE.LineDashedMaterial({
      color: 0xff00ff,
      dashSize: 0.2,
      gapSize: 0.15,
      transparent: true,
      opacity: 0.7,
    });
    const line = new THREE.Line(geo, mat);
    line.computeLineDistances();
    group.add(line);
  }

  /** Populate the event group from EventDisplayData. */
  function renderEvent(data: EventDisplayData): void {
    // Clear previous event objects.
    while (eventGroup.children.length) {
      const child = eventGroup.children[0];
      eventGroup.remove(child);
    }

    for (const jet of data.jets) addJet(eventGroup, jet);
    for (const e of data.electrons) addTrack(eventGroup, e, 0x44ff44);      // green
    for (const mu of data.muons) addTrack(eventGroup, mu, 0xff4444);        // red
    for (const ph of data.photons) addTrack(eventGroup, ph, 0xffff44, true); // yellow dashed
    if (data.met) addMET(eventGroup, data.met);
  }

  // ---------------------------------------------------------------------------
  // Generate Event
  // ---------------------------------------------------------------------------
  async function handleGenerate(): Promise<void> {
    loading = true;
    errorMsg = "";
    eventData = null;

    if (cmsEnergy <= 0) {
      errorMsg = "CMS energy must be positive.";
      loading = false;
      return;
    }

    const finalMasses = Array(nFinal).fill(0.0);

    try {
      appendLog(
        `Generating event display: √s = ${cmsEnergy} GeV, ${nFinal}-body, ${detectorPreset}`,
      );

      const data = await generateDisplayEvent(
        cmsEnergy,
        finalMasses,
        detectorPreset,
      );

      eventData = data;
      renderEvent(data);

      const summary = [
        `${data.jets.length} jets`,
        `${data.electrons.length} e`,
        `${data.muons.length} μ`,
        `${data.photons.length} γ`,
      ].join(", ");
      appendLog(`Event display: ${summary} | MET = ${data.met.magnitude.toFixed(1)} GeV`);
    } catch (err) {
      errorMsg = String(err);
      appendLog(`Event display error: ${errorMsg}`);
    } finally {
      loading = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Three.js Lifecycle
  // ---------------------------------------------------------------------------
  function initScene(): void {
    if (!containerEl) return;

    scene = new THREE.Scene();
    scene.background = new THREE.Color(0x111118);

    camera = new THREE.PerspectiveCamera(
      50,
      containerEl.clientWidth / containerEl.clientHeight,
      0.1,
      100,
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

    // Ambient + directional light for visual depth.
    scene.add(new THREE.AmbientLight(0xffffff, 0.4));
    const dirLight = new THREE.DirectionalLight(0xffffff, 0.6);
    dirLight.position.set(5, 10, 7);
    scene.add(dirLight);

    // Detector wireframe.
    detectorGroup = buildDetector();
    scene.add(detectorGroup);

    // Event objects group.
    eventGroup = new THREE.Group();
    scene.add(eventGroup);

    // Animation loop.
    function animate(): void {
      animFrameId = requestAnimationFrame(animate);
      controls?.update();
      if (renderer && scene && camera) {
        renderer.render(scene, camera);
      }
    }
    animate();
  }

  function handleResize(): void {
    if (!containerEl || !camera || !renderer) return;
    camera.aspect = containerEl.clientWidth / containerEl.clientHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(containerEl.clientWidth, containerEl.clientHeight);
  }

  onMount(() => {
    initScene();
    window.addEventListener("resize", handleResize);
  });

  onDestroy(() => {
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
        <input id="ed-energy" type="number" bind:value={cmsEnergy} min="10" step="any" />
      </div>
      <div class="field compact">
        <label for="ed-nfinal">Final N:</label>
        <input id="ed-nfinal" type="number" bind:value={nFinal} min="2" max="8" />
      </div>
      <div class="field compact">
        <label for="ed-detector">Detector:</label>
        <select id="ed-detector" bind:value={detectorPreset}>
          <option value="lhc_like">LHC-like</option>
          <option value="ilc_like">ILC-like</option>
          <option value="perfect">Perfect</option>
        </select>
      </div>
    </div>

    <div class="run-row">
      <button class="run-btn" on:click={handleGenerate} disabled={loading}>
        {#if loading}
          ⏳ Generating…
        {:else}
          🔬 Generate Event
        {/if}
      </button>
    </div>

    {#if errorMsg}
      <div class="error-box">{errorMsg}</div>
    {/if}

    {#if eventData}
      <div class="event-summary">
        <span class="badge jet">{eventData.jets.length} jets</span>
        <span class="badge electron">{eventData.electrons.length} e⁻</span>
        <span class="badge muon">{eventData.muons.length} μ</span>
        <span class="badge photon">{eventData.photons.length} γ</span>
        <span class="badge met">MET {eventData.met.magnitude.toFixed(1)} GeV</span>
      </div>
    {/if}
  </div>

  <!-- 3D Viewport -->
  <div class="viewport" bind:this={containerEl}>
    {#if !eventData && !loading}
      <div class="placeholder">
        Click <strong>Generate Event</strong> to visualise a collision.
      </div>
    {/if}
  </div>

  <!-- Legend -->
  <div class="legend">
    <span class="legend-item" style="color: #ff8800;">■ Jets</span>
    <span class="legend-item" style="color: #44ff44;">— Electrons</span>
    <span class="legend-item" style="color: #ff4444;">— Muons</span>
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

  .field-row {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .field.compact {
    flex: 1;
    min-width: 80px;
  }

  .field label {
    color: #aaa;
    font-size: 0.75rem;
  }

  input,
  select {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 4px;
    color: #e0e0e0;
    padding: 0.25rem 0.4rem;
    font-size: 0.8rem;
  }

  .run-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

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

  .run-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .run-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .error-box {
    padding: 0.3rem 0.5rem;
    background: rgba(255, 50, 50, 0.12);
    border: 1px solid rgba(255, 50, 50, 0.3);
    border-radius: 4px;
    color: #ef5350;
    font-size: 0.78rem;
    word-break: break-word;
  }

  .event-summary {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .badge {
    padding: 0.15rem 0.5rem;
    border-radius: 3px;
    font-size: 0.72rem;
    font-weight: 600;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .badge.jet      { background: rgba(255, 136, 0, 0.15); color: #ff8800; }
  .badge.electron { background: rgba(68, 255, 68, 0.12); color: #44ff44; }
  .badge.muon     { background: rgba(255, 68, 68, 0.12); color: #ff4444; }
  .badge.photon   { background: rgba(255, 255, 68, 0.12); color: #ffff44; }
  .badge.met      { background: rgba(255, 0, 255, 0.12); color: #ff00ff; }

  .viewport {
    position: relative;
    flex: 1;
    min-height: 300px;
    border-radius: 6px;
    overflow: hidden;
    background: #111118;
  }

  .viewport :global(canvas) {
    display: block;
    width: 100% !important;
    height: 100% !important;
  }

  .placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #555;
    font-size: 0.85rem;
    text-align: center;
    padding: 1rem;
    pointer-events: none;
  }

  .legend {
    display: flex;
    gap: 0.8rem;
    justify-content: center;
    padding: 0.25rem 0;
    font-size: 0.72rem;
    flex-shrink: 0;
  }

  .legend-item {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    opacity: 0.8;
  }
</style>
