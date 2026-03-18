<script lang="ts">
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
    key: string;
  }

  interface NucleonPoint extends Point3D {
    type: "proton" | "neutron";
  }

  const GOLDEN_ANGLE = Math.PI * (3 - Math.sqrt(5));
  let viewportEl: HTMLDivElement | null = null;
  let rotating = false;
  let activePointerId: number | null = null;
  let startX = 0;
  let startY = 0;
  let startYaw = -0.55;
  let startPitch = 0.34;
  let yaw = -0.55;
  let pitch = 0.34;

  function activeElement(): ElementData | null {
    return isotope?.element ?? element;
  }

  function atomicNumber(): number {
    return isotope?.Z ?? activeElement()?.Z ?? 0;
  }

  function massNumber(): number {
    return isotope?.A ?? Math.max(1, Math.round(activeElement()?.atomic_mass ?? 1));
  }

  function parseShellOccupancy(config: string | undefined, z: number): number[] {
    const shellCounts = new Map<number, number>();
    const matches = config?.matchAll(/(\d+)([spdf])(\d+)/g) ?? [];
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
    for (let i = 0; remaining > 0; i += 1) {
      const cap = caps[i] ?? 2;
      const current = next[i] ?? 0;
      const add = Math.min(cap - current, remaining);
      if (add > 0) {
        next[i] = current + add;
        remaining -= add;
      }
    }
    return next;
  }

  function nucleonCloud(protons: number, neutrons: number, radius: number): NucleonPoint[] {
    const total = Math.max(0, protons) + Math.max(0, neutrons);
    const sequence: ("proton" | "neutron")[] = [];
    const points: NucleonPoint[] = [];

    for (let i = 0; i < Math.max(protons, neutrons); i += 1) {
      if (i < protons) sequence.push("proton");
      if (i < neutrons) sequence.push("neutron");
    }

    for (let index = 0; index < total; index += 1) {
      const radial = total <= 1 ? 0 : Math.sqrt(index / Math.max(1, total - 1)) * radius * 0.82;
      const angle = index * GOLDEN_ANGLE;
      const spiral = ((index / Math.max(1, total - 1)) - 0.5) * radius * 1.2;
      points.push({
        x: Math.cos(angle) * radial,
        y: Math.sin(angle) * radial * 0.88,
        z: spiral,
        type: sequence[index] ?? "neutron",
        key: `nucleon-${index}`,
      });
    }

    return points;
  }

  function electronCloud(shells: number[]): Point3D[] {
    const points: Point3D[] = [];

    shells.forEach((count, shellIndex) => {
      const radius = 46 + shellIndex * 20;
      for (let index = 0; index < count; index += 1) {
        const angle = ((index + 0.5) / Math.max(1, count)) * Math.PI * 2;
        const tilt = shellIndex % 2 === 0 ? 0.42 : -0.42;
        const x = Math.cos(angle) * radius;
        const y = Math.sin(angle) * radius * Math.cos(tilt);
        const z = Math.sin(angle) * radius * Math.sin(tilt);
        points.push({
          x,
          y,
          z,
          key: `electron-${shellIndex + 1}-${index}`,
        });
      }
    });

    return points;
  }

  function orbitCurves(shells: number[]): Point3D[][] {
    return shells.map((_, shellIndex) => {
      const radius = 46 + shellIndex * 20;
      return Array.from({ length: 56 }, (_, index) => {
        const angle = (index / 56) * Math.PI * 2;
        const tilt = shellIndex % 2 === 0 ? 0.42 : -0.42;
        return {
          x: Math.cos(angle) * radius,
          y: Math.sin(angle) * radius * Math.cos(tilt),
          z: Math.sin(angle) * radius * Math.sin(tilt),
          key: `orbit-${shellIndex + 1}-${index}`,
        };
      });
    });
  }

  function rotatePoint(point: Point3D): Point3D {
    const cosY = Math.cos(yaw);
    const sinY = Math.sin(yaw);
    const cosX = Math.cos(pitch);
    const sinX = Math.sin(pitch);

    const x1 = point.x * cosY + point.z * sinY;
    const z1 = -point.x * sinY + point.z * cosY;
    const y2 = point.y * cosX - z1 * sinX;
    const z2 = point.y * sinX + z1 * cosX;

    return {
      x: x1,
      y: y2,
      z: z2,
      key: point.key,
    };
  }

  function projectPoint(point: Point3D, centerX = 160, centerY = 124, perspective = 420): { x: number; y: number; scale: number; depth: number } {
    const depth = perspective / (perspective - point.z);
    return {
      x: centerX + point.x * depth,
      y: centerY + point.y * depth,
      scale: depth,
      depth: point.z,
    };
  }

  function beginRotate(event: PointerEvent): void {
    if (!viewportEl) return;
    rotating = true;
    activePointerId = event.pointerId;
    startX = event.clientX;
    startY = event.clientY;
    startYaw = yaw;
    startPitch = pitch;
    viewportEl.setPointerCapture(event.pointerId);
  }

  function moveRotate(event: PointerEvent): void {
    if (!rotating || event.pointerId !== activePointerId) return;
    yaw = startYaw + (event.clientX - startX) * 0.01;
    pitch = Math.max(-1.2, Math.min(1.2, startPitch + (event.clientY - startY) * 0.008));
  }

  function endRotate(event: PointerEvent): void {
    if (!rotating || event.pointerId !== activePointerId) return;
    if (viewportEl?.hasPointerCapture(event.pointerId)) viewportEl.releasePointerCapture(event.pointerId);
    rotating = false;
    activePointerId = null;
  }

  $: z = atomicNumber();
  $: a = massNumber();
  $: n = Math.max(0, a - z);
  $: shellCounts = activeElement() ? parseShellOccupancy(activeElement()?.electron_configuration, z) : [];
  $: nucleusRadius = 12 + Math.cbrt(Math.max(1, a)) * 2.4;
  $: nucleons = nucleonCloud(z, n, nucleusRadius);
  $: electrons = electronCloud(shellCounts);
  $: orbits = orbitCurves(shellCounts);
  $: projectedNucleons = nucleons
    .map((point) => {
      const rotated = rotatePoint(point);
      const projected = projectPoint(rotated);
      return { ...point, ...projected };
    })
    .sort((aPoint, bPoint) => aPoint.depth - bPoint.depth);
  $: projectedElectrons = electrons
    .map((point) => {
      const rotated = rotatePoint(point);
      const projected = projectPoint(rotated);
      return { ...projected, key: point.key };
    })
    .sort((aPoint, bPoint) => aPoint.depth - bPoint.depth);
  $: projectedOrbits = orbits.map((curve, index) => {
    const rotated = curve.map((point) => {
      const projected = projectPoint(rotatePoint(point));
      return `${projected.x},${projected.y}`;
    });
    return {
      key: `orbit-${index + 1}`,
      points: rotated.join(" "),
    };
  });
  $: sceneLabel = subtitle ? `${activeElement()?.name ?? "selected element"} · ${subtitle}` : `${activeElement()?.name ?? "selected element"}`;
</script>

<section class="atomic-model-card">
  <div
    class="atomic-viewport-shell"
    bind:this={viewportEl}
    role="application"
    aria-label={sceneLabel}
    on:pointerdown={beginRotate}
    on:pointermove={moveRotate}
    on:pointerup={endRotate}
    on:pointercancel={endRotate}
  >
    <svg class="atomic-viewport" viewBox="0 0 320 248" role="img" aria-label={sceneLabel}>
      <rect class="viewport-bg" x="8" y="8" width="304" height="232" rx="12" />

      {#each projectedOrbits as orbit (orbit.key)}
        <polyline class="orbit" points={orbit.points} />
      {/each}

      {#each projectedNucleons as dot (dot.key)}
        <circle
          class="nucleon"
          class:nucleon-proton={dot.type === "proton"}
          class:nucleon-neutron={dot.type === "neutron"}
          cx={dot.x}
          cy={dot.y}
          r={Math.max(1.9, 2.8 * dot.scale)}
        />
      {/each}

      {#each projectedElectrons as electron (electron.key)}
        <circle class="electron" cx={electron.x} cy={electron.y} r={Math.max(1.6, 2.8 * electron.scale)} />
      {/each}
    </svg>
  </div>
</section>

<style>
  .atomic-model-card {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.35rem;
  }

  .atomic-viewport-shell {
    cursor: grab;
    touch-action: none;
  }

  .atomic-viewport-shell:active {
    cursor: grabbing;
  }

  .atomic-viewport {
    width: 100%;
    height: auto;
    display: block;
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.14);
    background: #08111a;
  }

  .viewport-bg {
    fill: rgba(8, 17, 25, 0.96);
    stroke: rgba(141, 161, 181, 0.1);
  }

  .orbit {
    fill: none;
    stroke: rgba(145, 179, 212, 0.24);
    stroke-width: 1;
  }

  .electron {
    fill: #79b4ff;
    stroke: rgba(225, 238, 255, 0.24);
    stroke-width: 0.5;
  }

  .nucleon {
    stroke-width: 0.45;
  }

  .nucleon-proton {
    fill: #d75b5b;
    stroke: rgba(71, 16, 16, 0.48);
  }

  .nucleon-neutron {
    fill: #f4f6f8;
    stroke: rgba(105, 116, 132, 0.42);
  }
</style>
