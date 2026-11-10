# PodLogs: full-Rust migration (gpui UI) — design

Date: 2026-11-11 · Branch: `andrisasuke/Rust-full-migrate`

## 1. Intent

PodLogs today is a Tauri 2 app: Rust backend (kube-rs) + React/TypeScript UI rendered in the OS webview.
The owner wants the **whole app in Rust, with no webview and no JavaScript**, so the app is lighter
(no WebKit/WebView2 processes, no Node toolchain). The UI is rebuilt natively with **gpui** (Zed's
GPU UI framework) through the **gpui-kit / gpui-component** library.

Success = a user switching from the current build to the new one sees **the same colors, the same
layout, and the same features**, on **macOS, Windows and Linux**.

## 2. Goals, non-goals, anti-goals

**Goals**
- Pixel-faithful port of every screen, drawer, modal and state listed in §6 (colors, spacing, sizes,
  typography incl. letter-spacing, hover/active states, empty/loading/error states).
- Every current feature keeps working (§6 is the checklist).
- One native binary per platform; no webview, no JS, no Node/npm in the build.
- Ships for macOS (arm64 + x64), Linux (amd64 + arm64) and **Windows x64 (new)** in the same
  release (owner decision). Only macOS is tested by hand for now. Linux/Windows ship as long as
  their CI build succeeds. They get a manual test pass and bug fixes later, once the owner has
  machines for them.
- Existing K8s backend logic is reused, not rewritten.
- Two deliberate changes vs. today (owner decision, §6):
  1. the Connection Error dialog shows the specific message it was always meant to show;
  2. the sidebar gets a collapse/expand toggle so the existing 60 px collapsed variant is reachable.

**Non-goals**
- No other new features, no redesign, no behavior changes in K8s logic.
- No Cmd/Ctrl+K shortcut (the README mentions it, but it was never implemented).
- No web/WASM build, no mobile.
- No auto-updater (none exists today).

**Anti-goals (a failure even if it "works")**
- Any visible color/layout drift from the current app beyond the known gaps in §7.
- Losing a feature or a state (e.g. error dialog, skeletons, "filters changed" hint, partial-search warning).
- The app getting heavier at runtime than the Tauri build (RAM incl. webview processes).

## 3. Constraints

- **Rust toolchain ≥ 1.93** (gpui-pre uses `slice_as_array`). Pin `1.99.1` with `rust-toolchain.toml`
  (spike-verified). The machine's default stable is 1.92, so the pin is required. Nightly does **not**
  work (`pathfinder_simd` breaks).
- **Pin `gpui-kit = "=0.9.1"`** (it pins `gpui-pre = 0.3.12`, a crates.io snapshot of Zed's gpui).
  The official `gpui` crate on crates.io is stale (0.2.2, Oct 2025). gpui-kit releases weekly with
  breaking changes → upgrades are deliberate, never `cargo update`-by-accident.
- Licenses: gpui-pre, gpui-kit, gpui-component, Lucide icons — Apache-2.0 / ISC. Fonts: IBM Plex Sans
  and JetBrains Mono — OFL 1.1 (embedding allowed; ship the license texts).
- Linux needs gpui's windowing deps (X11 and/or Wayland, xkbcommon, Vulkan/GL); Windows uses
  Win32 + DirectWrite. Windows/Linux are less mature than macOS in gpui — budget time for
  platform bugs (§9).
- kube-rs requires **tokio**; gpui has its own executor (see §5.3).

## 4. Decided by the owner

- UI framework: **gpui** (via gpui-kit/gpui-component). egui was considered and dropped because exact
  color/layout parity is far easier with gpui's flexbox + Tailwind-like styling API.
- Parity bar: **colors, layout and features must be exactly the same.**
- Platforms: **macOS, Windows, Linux.**
- A throwaway spike (Pods + Logs screens, fake data) was built and accepted as "sesuai" (matches).
- **Release:** macOS, Linux and Windows all ship in the release that replaces Tauri.
  Windows build failures must not block macOS/Linux.
- **Testing:** the owner has macOS only. Parity checks happen on macOS. Linux/Windows are **not**
  manually tested in this project. They are tested and fixed later, as follow-up work, when
  machines are available (§8).
- **Fix, don't copy, two dead behaviors:**
  - Error-message mapping in the Connection Error dialog → make it work (see §6 Errors).
  - Collapsed sidebar → build it **and** add a toggle to reach it (see §6 Sidebar).
- **Letter-spacing must match exactly** (`tracking-wider` = 0.05em on uppercase labels) → a custom
  tracked-text element is built first, before any screen (§7).

## 5. Architecture (builder's calls are marked *[my call]*)

### 5.1 Repository layout *[my call]*
React/Vite/Node/Tauri are removed. The Rust crate moves to the repo root:

```
Cargo.toml              # single binary crate "podlogs", edition 2024
rust-toolchain.toml     # channel = "1.99.1"
assets/fonts/           # IBM Plex Sans 400/500/600, JetBrains Mono 400/500 (TTF) + OFL.txt
assets/icon.png         # sidebar logo (today: public/icon.png)
icons/                  # app icons (today: src-tauri/icons/) + new icon.ico for Windows
src/
  main.rs               # app bootstrap: fonts, theme, menus, window, tokio runtime
  error.rs  k8s/  models/   # moved from src-tauri/src, #[tauri::command] attributes removed
  app/                  # gpui state + views (one file per current React component group)
```
Deleted: `src/**/*.ts(x)`, `index.html`, `vite.config.*`, `tailwind.config.js`, `postcss.config.*`,
`package*.json`, `tsconfig*.json`, `public/`, `src-tauri/` (after its contents move), `src/test/`.

### 5.2 Backend reuse
`src-tauri/src/k8s/*.rs`, `models/`, `error.rs` move as-is. Remove `#[tauri::command]` and the
`serde::Serialize` impl on `K8sError` only if nothing needs it. Function signatures stay
(`get_pods(context, namespace, deployment)` etc.). `logs_tests.rs` keeps passing unchanged.
The 30 s timeouts stay in `client.rs`; the frontend's extra 30 s `invokeWithTimeout` is replaced by
`tokio::time::timeout(30s, …)` around each call (same user-visible message:
"Connection timed out after 30 seconds"). `get_clusters` stays without timeout (local file read).

### 5.3 Async bridge *[my call — spike-verified]*
One multi-thread tokio runtime (2 workers) owned by the app. A K8s call is
`runtime.spawn(k8s::pods::get_pods(...))`; the gpui side awaits the `JoinHandle` inside `cx.spawn`
and writes the result into state with `this.update(cx, …)` + `cx.notify()`. No extra bridge crate.

### 5.4 State (replaces Zustand + TanStack Query)
- `UiState` (theme, sidebar_collapsed, current_view, selected_pod, selected_deployment_info,
  drawer, log_viewer_pod, refresh_interval) and `ClusterState` (context, namespace, deployment,
  last_namespace_by_cluster) as plain structs inside the root view or as gpui `Entity`s.
- **Query layer *[my call]*:** a small `Query<T>` struct per resource
  (`data: Option<T>, loading: bool, fetching: bool, error: Option<String>, key`) driven by one
  refresh loop per active query. It must reproduce today's semantics exactly:
  clusters (no refetch, `staleTime ∞`), namespaces (stale 60 s), deployments/pods/pod-logs
  (poll every `refresh_interval`), pod/deployment details (fetch on open, no poll), `retry: 0`
  for K8s queries except pod-logs, which uses the QueryClient default `retry: 1` today (keep it),
  results for a stale key (context/namespace/deployment changed mid-flight) are
  discarded. The status-bar countdown resets when a pods fetch completes.
- **Error dialog rule:** report an error only when the query has no cached data (initial load),
  never on background refetch; ignore errors for 1 s after a namespace/context switch
  (`isTransitioning`). Retry resets and refetches clusters, namespaces, deployments, pods.
- **Persistence *[my call]*:** JSON file at `dirs::config_dir()/podlogs/state.json` with
  `theme`, `sidebar_collapsed`, `refresh_interval`, `context`, `namespace`, `deployment`,
  `last_namespace_by_cluster` (same fields Zustand persists today). Write on change; tolerate a
  missing/corrupt file by falling back to defaults. Old localStorage values are **not** migrated.

### 5.5 Theming
- One `Palette` struct with the exact light (Solarized) and dark (Slate) values from
  `src/index.css`, plus the Tailwind palette constants used by components (emerald/amber/red/
  purple/gray/blue/cyan 400–600, accent `#0EA5E9`, accent-hover `#0284C7`).
- `theme = system` follows the OS appearance (`window.appearance()` / observe changes).
- Push palette + fonts into gpui-component's `Theme` so its `Input` matches (spike: set
  `background, foreground, muted_foreground, border, input, ring, caret, selection`, `radius = 6`).
- Tailwind font sizes set **size and line-height** (`text-xs 12/16, text-sm 14/20, text-lg 18/28`,
  body 14 / 1.5). gpui only sets size → use a helper trait (spike: `t_xs/t_sm/t_lg/mono`).

### 5.6 Components to build (one Rust fn/struct per React component)
Button (primary/secondary/ghost × sm/md/lg/icon, disabled = 50 % opacity, no hover),
Input (gpui-component `Input` + search-icon prefix), Dropdown (custom: trigger + deferred popup +
click-outside backdrop + optional search; spike-verified), Badge/StatusBadge/LogLevelBadge/LabelBadge,
Modal (backdrop black/50 light, /70 dark; sizes sm/md/lg/xl 400/500/700/900; `persistent` disables
ESC/backdrop), Drawer (480 px, right, backdrop black/20 light, /40 dark, 300 ms slide/fade),
Skeleton (pulse), StatusBar, Sidebar (expanded 260 px / collapsed 60 px), tables.
Icons: Lucide via `gpui_kit::assets::IconName`; register only the used icons with
`icon_assets!` instead of `AllAssets` (saves ~1 MB).

### 5.7 Native shell
- Window: title "PodLogs", 1400×900, min 1000×700, starts maximized, background `#0F172A`.
- Menus identical to today (`src-tauri/src/lib.rs`): PodLogs (About w/ version + icon,
  Services, Hide, Hide Others, Show All, Quit), Edit (Undo/Redo/Cut/Copy/Paste/Select All),
  Window (Minimize, Zoom, Close). macOS: native menu via `cx.set_menus`. Linux/Windows: Tauri's
  `set_menu` shows an in-window menu bar today, so render the same menus with gpui-component's
  `AppMenuBar` at the top of the window *[my call]*; macOS-only items (Services, Hide Others,
  Show All) are omitted there, as Tauri does.
- Quit when the last window closes.
- Export: native save dialog (`cx.prompt_for_new_path`) + `std::fs::write` (replaces Blob download).
- Clipboard: `cx.write_to_clipboard`.

### 5.8 Packaging & CI *[my call]*
`cargo-packager` produces `.dmg`/`.app` (macOS), `.deb` + `.AppImage` (Linux), `.msi` or NSIS `.exe`
(Windows), using the existing icons + a new `icon.ico`. `.github/workflows/release.yml` drops
Node/tauri-action, keeps the artifact naming `PodLogs_${VERSION}_<os>_<arch>.<ext>`, and reads the
version from `Cargo.toml` (today: `tauri.conf.json`). macOS minimum stays 10.15 if gpui allows;
otherwise use gpui's minimum and say so in release notes.
- The Windows x64 job is **non-blocking** (`continue-on-error`). Its artifact is attached to the
  release when it builds; if it fails, macOS/Linux still release *[my call]*.
- Windows x64 only, unsigned — same as the macOS builds today, which are not signed or notarized
  *[my call]*. SmartScreen will warn; document it in the release notes.

## 6. Parity checklist (source of truth = current React code)

| Area | Source | Must keep |
|---|---|---|
| Layout | `layout/MainLayout.tsx` | sidebar + main + 28 px status bar; drawers & error dialog overlay; namespace/context change → back to Deployments, close drawer, cancel in-flight logs/details |
| Sidebar | `layout/Sidebar.tsx` | logo+name, theme toggle, Cluster dropdown, searchable Namespace dropdown, nav (Deployments, Log Search; active = accent text + accent/10 bg), deployment filter, list with health dot + replica count, toggle-select → Pods view, auto-pick current context, namespace fallback to `default`/first. **Collapsed 60 px variant (exists in code, unreachable today) + new toggle** *[placement: my call]*: expanded header gets a ghost icon button (`PanelLeftClose`, 32×32, same style as the theme toggle) left of the theme toggle; the collapsed rail gets a `PanelLeftOpen` ghost button directly above its theme toggle. State persists via `sidebar_collapsed`. |
| Status bar | `layout/StatusBar.tsx` | Live / Syncing (pulsing amber), "Showing N pod(s)" on Pods view, "Refresh in Ns" countdown hidden while fetching |
| Deployments | `deployments/DeploymentList.tsx` | header + search + settings, table (Name w/ Layers icon, Replicas, Available colored, Ready, Status dot+Healthy/Degraded/Unhealthy, Info), row click → Pods, skeleton, empty state |
| Deployment drawer | `deployments/DeploymentDetails.tsx` | Overview, Strategy, Selector, Labels, collapsible Annotations, Conditions (open by default) |
| Pods | `pods/PodList.tsx` | title + accent deployment name, search, settings, table (Name mono, StatusBadge, Ready, Restarts red if >0, Age, IP mono, Logs/Info disabled for Terminating/ContainerCreating/Evicted — no tooltip, because `pointer-events-none` hides it today), skeleton, empty state |
| Pod drawer | `pods/PodDetails.tsx` | Overview, Labels, container cards (image, restarts, resources, ports, collapsible env vars, `<secret>` masking), collapsible Conditions |
| Log viewer | `logs/LogViewer.tsx` | Back, title + pod, settings, Container/Time dropdowns, filter, JSON/Raw toggle, virtualized rows (ERROR 52 px 2-line clamp, others 36 px, raw 24 px, level tints), auto-scroll (<100 px from bottom), footer (Live/Syncing, N lines with thousands separator, Auto-scroll ON/OFF, Export .txt), skeleton |
| Log detail modal | `logs/LogDetailModal.tsx` | metadata grid, Message / Raw / JSON (formatted) with Copy → "Copied" 2 s |
| Log search | `logs/LogSearch.tsx` (+ `LogSearch.test.tsx`, `hooks/useLogs.ts`) | 4-column form (searchable Deployment, Keyword, Level incl. Any, Time default 1h), Enter submits, Search disabled rules, "Filters changed" hint, applied-filters line, partial-failure alert with `<details>` list + Retry, all-failed/error state, no-containers state, no-results state, results header (matches/pods counts in accent), resizable Pod column 100–500 px, keyword highlight (amber/30 mark), error rows 2-line clamp, footer pod chips + Export CSV, stale-response guard (snapshot vs current scope) |
| Settings drawer | `settings/Settings.tsx` | Theme (Dark/Light/System), Refresh interval (5/10/15/20/30 s), helper texts |
| Errors | `common/ErrorDialog.tsx` | persistent Connection Error modal, Dismiss + Retry. **Message mapping is fixed, not copied:** today `invoke` rejects with a plain string (`K8sError` serializes to its `Display`), `error.message` is undefined, so every error shows "Failed to connect to Kubernetes cluster / Unknown error occurred". The new app feeds the `K8sError` text into the existing mapping (kubeconfig / connection refused / Unauthorized·401 / Forbidden·403 / timeout / certificate) and also matches `timed out` so the 30 s timeout maps to "Connection timed out" *[my call]*. Unmatched errors keep the generic title with the raw error text as details. |
| Behaviors | stores, hooks | everything in §5.4 |

## 7. Known gaps (from the spike) and how to handle them

| Gap | Handling |
|---|---|
| Tailwind `tracking-wider` (letter-spacing on uppercase labels) — gpui has no letter-spacing | **Must match (owner decision).** Build a `tracked_text` element first, before any screen: shape the string with `window.text_system()`, then paint each glyph with an extra `0.05em` advance (0.6 px at 12 px), keeping kerning and baseline. It also has to measure its own width for layout. Used for every `uppercase tracking-wider` label: sidebar section labels, all table headers, Log Search form labels, Settings labels, drawer section titles. Done when, at 2× scale, a label's glyph positions match the Tauri build within 1 device pixel on macOS. |
| HTML `<table>` auto column widths | Fixed per-column widths sized for the widest real content; for Pods/Deployments measure text with `window.text_system()` if fixed widths visibly clip. |
| Mouse text selection (log rows, modal, drawers) | Use gpui-component's selectable text (`text::…selectable(true)`, present in 0.9.1 source, not yet tried) in the Log detail modal and drawers; log rows stay click-to-open as today. |
| Native tooltips (`title=` attributes) | gpui-component `Tooltip`. |
| `<details>` disclosure in Log Search | Custom collapsible with chevron. |

## 8. Testing

- `logs_tests.rs` and any Rust unit tests keep passing (`cargo test`).
- Port the behaviors covered by `LogSearch.test.tsx` to gpui-kit UI tests (`test-support` feature,
  headless window) — same cases, not the same code.
- Pure logic gets unit tests: pod status → badge variant, error-message mapping, query
  key/staleness rules, persistence load/save fallback, thousands formatting, timestamp formatting.
- **Parity baseline:** the React app at `c70bd05` (v1.0.2) *[my call]*. Before cut-over, diff `main`
  against `c70bd05`. Port any UI or behavior change that landed since then, and add it to §6.
- **Manual parity (owner, macOS only):** go through every screen, light + dark, side by side with the
  Tauri v1.0.2 build. The owner checks at milestones the plan defines.
- **Linux/Windows (owner decision):** no manual testing in this project. The only gate is automated:
  - CI must build the Linux `.deb` + `.AppImage` (amd64 + arm64).
  - `cargo test` and the headless UI tests must pass on the Linux runner.
  - Windows must build, but a failure doesn't block the release.

  Follow-up work, once machines exist: a smoke checklist on real hardware with a GPU (window,
  fonts, input/IME, dropdowns, save dialog, clipboard, HiDPI), a parity pass against the macOS
  build, and bug fixes.
- **"Not heavier" measurement** *[my call]*, on macOS, Tauri v1.0.2 vs. the gpui build, same cluster:
  - Metric: total physical footprint. For Tauri this is the app process plus all of its
    `WebKit.WebContent`, `WebKit.Networking` and `WebKit.GPU` processes; for gpui it is the single
    process.
  - Scenario (a): Pods view idle for 2 minutes.
  - Scenario (b): Log Viewer open for 2 minutes on a pod with ≥ 3,000 lines.
  - Gate: gpui must be ≤ Tauri in both scenarios, or the release does not ship until it is.
  - The log buffer stays unbounded, like today.

## 9. Risks

- gpui on Windows/Linux has open bugs (e.g. drag-and-drop crash on Windows in gpui 0.2.2). Plan a
  per-platform smoke test early (window, fonts, input, IME, dialogs, clipboard, HiDPI).
- Linux/Windows ship untested by hand (owner decision), so platform bugs may reach users until the
  follow-up test pass. Mitigation: get Linux/Windows CI builds green early in the plan, not at the end.
- The custom letter-spacing element (§7) sits on the critical path. If it can't hit the 1-px bar,
  stop and bring it back to the owner before building screens on top of it.
- gpui-kit API churn: pinned version; upgrade only as its own task.
- Binary size grows (spike release: 11.2 MB vs 6.0 MB Tauri binary); RAM is the win
  (spike: ~85 MB RSS, single process). Measure the Tauri app's full RAM (incl. webview
  processes) before declaring the "lighter" goal met.

## 10. Open items for the owner

None — all builder-check questions were answered on 2026-11-11 (§4). Calls marked *[my call]* are
the designer's and are open to the owner's veto during review.

## 11. Left to the builder

Module split inside `src/app/`, exact `Query<T>` shape, how drawers animate in gpui (any approach
that matches 300 ms slide + fade), which packager flags, CI caching.

## Appendix: spike learnings (gpui-kit 0.9.1 APIs that worked)

`gpui_kit::application().with_assets(..).run`, `gpui_kit::init`, `gpui_kit::open_window`,
`cx.text_system().add_fonts`, `Theme::change` + `Theme::global_mut`, `InputState::new(..).placeholder`,
`Input::new(&state).prefix(Icon)`, `cx.subscribe(&input, |.., ev: &InputEvent, cx|)`,
`v_virtual_list(cx.entity(), id, Rc<Vec<Size>>, |this, range, window, cx| ..)` +
`VirtualListScrollHandle` (`scroll_to_item(ix, ScrollStrategy::Bottom)`, `offset()`, `max_offset()`),
`deferred(anchored().position_mode(AnchoredPositionMode::Window)…)` for overlays/backdrops,
`.with_animation(id, Animation::new(2s).repeat(), ..)` for pulse, `.line_clamp(2)`, `.truncate()`,
`KeyBinding::new("escape", CloseModal, None)` + `.track_focus` + `.on_action`,
`cx.prompt_for_new_path`, `cx.write_to_clipboard(ClipboardItem::new_string(..))`.
Edition-2024 gotcha: `impl IntoElement` return types capture `cx`; convert with
`.into_any_element()` at the call site when collecting into children.
