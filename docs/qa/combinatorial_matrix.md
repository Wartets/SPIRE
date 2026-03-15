# Phase 64 - Cross-Feature Combinatorial Matrix

## Scope

This matrix audits cross-feature interactions across:
- All registered widget types
- Both workbench modes (`docking`, `canvas`)
- Mode toggling and persistence reload behavior
- IPC/back-end compatibility safety in web fallback mode

## Widget Universe

| # | Widget | Type Key |
|---|---|---|
| 1 | Model Loader | `model` |
| 2 | Reaction Workspace | `reaction` |
| 3 | Diagram Visualizer | `diagram` |
| 4 | Amplitude Panel | `amplitude` |
| 5 | Kinematics | `kinematics` |
| 6 | Dalitz Plot | `dalitz` |
| 7 | Analysis | `analysis` |
| 8 | Event Display | `event_display` |
| 9 | Particle Atlas | `particle_atlas` |
| 10 | Diagram Editor | `diagram_editor` |
| 11 | Lagrangian Workbench | `lagrangian_workbench` |
| 12 | External Models | `external_models` |
| 13 | Compute Grid | `compute_grid` |
| 14 | References | `references` |
| 15 | Telemetry | `telemetry` |
| 16 | Console | `log` |
| 17 | Notebook | `notebook` |
| 18 | Parameter Scanner | `parameter_scanner` |
| 19 | Decay Calculator | `decay_calculator` |
| 20 | Cosmology | `cosmology` |
| 21 | Flavor Workbench | `flavor_workbench` |
| 22 | Plugin Manager | `plugin_manager` |
| 23 | Global Fit Dashboard | `global_fit_dashboard` |

## Matrix Dimensions

| Dimension | Values |
|---|---|
| Mode | `docking`, `canvas` |
| Render path | full widget, summary LOD, unknown fallback |
| Lifecycle | spawn, close, reload |
| Cross-feature | model → reaction → diagram → analysis → compute grid |
| IPC safety | no hard dependency on `window.__TAURI_IPC__` in browser path |

## Automated Coverage

| Scenario | Test File | Status |
|---|---|---|
| Critical workflow (model→reaction→diagram→analysis→compute grid) | `apps/spire-desktop/e2e/specs/critical_workflow.spec.ts` | Implemented |
| Full widget matrix across docking+canvas with reload persistence | `apps/spire-desktop/e2e/specs/widget_matrix.spec.ts` | Implemented |
| IPC fallback guard (`__TAURI_IPC__` browser error) | `critical_workflow.spec.ts` console assertion | Implemented |

## Manual Spot Checks Executed

| Check | Result |
|---|---|
| Dynamic registry in `InfiniteCanvas.svelte` | Pass |
| Dynamic registry in `WidgetContainer.svelte` | Pass |
| Unknown-widget fallback component wired | Pass |
| Hardcoded widget branch chains removed (target files) | Pass |

## Notes

- Matrix tests are intentionally run against the web fallback runtime via Playwright web server.
- Backend behavior is validated through the environment-aware API layer and console-error guardrails.
- For CI hardening, add these specs to nightly regression and capture Playwright HTML artifacts.
