<!--
  SPIRE Desktop — Main Workspace Page

  Dashboard layout with:
    • Left sidebar  — Model Loader + Reaction Workspace
    • Centre main   — Feynman Diagram Visualizer
    • Right sidebar  — Amplitude Panel + Kinematics View
    • Bottom footer  — Log Console
-->
<script lang="ts">
  import ModelLoader from "$lib/components/ModelLoader.svelte";
  import ReactionWorkspace from "$lib/components/ReactionWorkspace.svelte";
  import DiagramVisualizer from "$lib/components/DiagramVisualizer.svelte";
  import AmplitudePanel from "$lib/components/AmplitudePanel.svelte";
  import KinematicsView from "$lib/components/KinematicsView.svelte";
  import DalitzPlotter from "$lib/components/DalitzPlotter.svelte";
  import LogConsole from "$lib/components/LogConsole.svelte";
</script>

<div class="workspace">
  <!-- ─── Left Sidebar ─── -->
  <aside class="sidebar left-sidebar">
    <section class="panel">
      <ModelLoader />
    </section>
    <section class="panel">
      <ReactionWorkspace />
    </section>
  </aside>

  <!-- ─── Centre Column ─── -->
  <div class="centre-column">
    <section class="panel diagram-area">
      <DiagramVisualizer />
    </section>
    <section class="panel dalitz-area">
      <DalitzPlotter />
    </section>
  </div>

  <!-- ─── Right Sidebar ─── -->
  <aside class="sidebar right-sidebar">
    <section class="panel">
      <AmplitudePanel />
    </section>
    <section class="panel">
      <KinematicsView />
    </section>
  </aside>

  <!-- ─── Bottom Console ─── -->
  <footer class="console-bar">
    <LogConsole />
  </footer>
</div>

<style>
  .workspace {
    display: grid;
    grid-template-columns: 280px 1fr 300px;
    grid-template-rows: 1fr auto;
    gap: 0.6rem;
    height: 100%;
    min-height: 0;
  }

  /* Sidebars span the first row */
  .left-sidebar {
    grid-column: 1;
    grid-row: 1;
  }
  .centre-column {
    grid-column: 2;
    grid-row: 1;
    min-height: 0;
  }
  .right-sidebar {
    grid-column: 3;
    grid-row: 1;
  }

  /* Console bar spans all columns at the bottom */
  .console-bar {
    grid-column: 1 / -1;
    grid-row: 2;
    height: 140px;
    background: #16213e;
    border-radius: 6px;
    padding: 0.5rem 0.6rem;
    color: #e0e0e0;
  }

  /* Sidebar layout */
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    overflow-y: auto;
  }

  /* Centre column fills available space */
  .centre-column {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .diagram-area {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .dalitz-area {
    flex: 0 0 auto;
    overflow: auto;
  }

  /* Panel base style */
  .panel {
    background: #16213e;
    border-radius: 6px;
    padding: 0.6rem 0.7rem;
    color: #e0e0e0;
  }

  /* Responsive: collapse to single column on narrow screens */
  @media (max-width: 900px) {
    .workspace {
      grid-template-columns: 1fr;
      grid-template-rows: auto auto auto auto;
    }
    .left-sidebar,
    .centre-column,
    .right-sidebar {
      grid-column: 1;
    }
    .left-sidebar  { grid-row: 1; }
    .centre-column { grid-row: 2; }
    .right-sidebar { grid-row: 3; }
    .console-bar   { grid-column: 1; grid-row: 4; }
  }
</style>
