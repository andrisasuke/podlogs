# PodLogs full-Rust (gpui) Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Tauri + React app with a single native Rust binary whose UI is built with gpui (gpui-kit 0.9.1). Colors, layout and features stay identical, and it ships for macOS, Linux and Windows.

**Architecture:** One binary crate at the repo root. The existing kube-rs backend (`k8s/`, `models/`, `error.rs`) moves over unchanged. A `Store` gpui entity replaces Zustand + TanStack Query: it holds the state, runs a 1-second tick that polls queries, and calls the backend through an `Api` trait. The real implementation runs every call on a tokio runtime with a 30 s timeout. Screens are gpui view entities that observe the `Store`. Shared widgets live in `src/ui/`; the palette lives in a gpui `Global`.

**Tech Stack:** Rust 1.99.1 (edition 2024), gpui-kit =0.9.1 (gpui-pre 0.3.12, gpui-component 0.9.1), tokio, kube 0.98 / k8s-openapi 0.24, serde/serde_json, chrono, dirs, cargo-packager, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-11-gpui-full-rust-migration-design.md`. Read it before starting any task. §6 is the parity checklist.

**Parity source of truth:** the React app at commit `c70bd05`. Read any old file with `git show c70bd05:<path>` (e.g. `git show c70bd05:src/components/pods/PodList.tsx`). Every "match React" in this plan refers to that commit. Keep the installed Tauri v1.0.2 app (`/Applications/PodLogs.app`) for side-by-side checks.

## Global Constraints

- Toolchain: `rust-toolchain.toml` → `channel = "1.99.1"`. Nightly does not work. The default stable on this machine (1.92) is too old.
- `gpui-kit = "=0.9.1"`, exact pin. Never `cargo update -p gpui-kit` inside a task.
- No webview, no JavaScript, no Node/npm anywhere in the build.
- Window: title "PodLogs", 1400×900, min 1000×700, starts maximized, background `#0F172A`.
- Palette — dark: bg `#0F172A/#1E293B/#334155`, text `#F8FAFC/#CBD5E1/#64748B`, border `#334155`, border-subtle `#1E293B`, accent-subtle `#0C4A6E`. Light: bg `#FDF6E3/#EEE8D5/#E6DFC6`, text `#073642/#586E75/#93A1A1`, border `#D3CDB8`, border-subtle `#E6DFC6`, accent-subtle `#D4E9E2`. Accent `#0EA5E9`, accent-hover `#0284C7`.
- Fonts: IBM Plex Sans 400/500/600 (UI), JetBrains Mono 400/500 (mono). Embedded; ship the OFL license texts.
- Typography (Tailwind size/line-height): xs 12/16, sm 14/20, base 16/24, lg 18/28. Body 14 with line-height 1.5. Uppercase labels use `tracking-wider` = 0.05em.
- Sizes: sidebar 260 / 60 px collapsed, drawer 480 px, status bar 28 px, header 56 px. Modal widths sm/md/lg/xl = 400/500/700/900.
- K8s call timeout 30 s, message exactly `Connection timed out after 30 seconds`. `get_clusters` has no timeout.
- Refresh intervals 5/10/15/20/30 s, default 5 s.
- Persisted state lives at `dirs::config_dir()/podlogs/state.json`. Old localStorage values are not migrated.
- macOS menus: PodLogs / Edit / Window, the same items as `c70bd05:src-tauri/src/lib.rs`. Linux/Windows: the same menus in an in-window `AppMenuBar`, without Services, Hide Others and Show All.
- Linux/Windows are not manually tested in this project. CI must build them; a Windows build failure must not block a release.

## Review Focus

1. **Missing or broken kubeconfig at first launch.**
   - Expected: the persistent Connection Error dialog shows "Kubernetes configuration not found", the sidebar shows placeholders, and the app does not crash.
   - Test: Task 8, `store_reports_missing_kubeconfig_on_startup`.
2. **Late responses after a context/namespace/deployment switch.**
   - Expected: data from the old scope never appears, and errors from the old scope never open the dialog.
   - Tests: Task 5, `stale_generation_is_ignored`; Task 8, `late_pods_after_namespace_switch_are_dropped`.
3. **A fetch slower than the refresh interval.**
   - Expected: no overlapping fetches of the same query; the countdown resets only when the fetch completes.
   - Test: Task 8, `slow_fetch_does_not_overlap_and_resets_countdown`.
4. **Huge or odd log content** (≥100k lines, a 10k-character line, a multi-line non-error message).
   - Expected: typing in the filter stays responsive, and rows keep their fixed 24/36/52 px heights.
   - Test: Task 13, `filter_cache_recomputes_only_on_input_change` and `row_heights_ignore_content_length`.
5. **A pod with zero containers, or a pod deleted while its logs are open.**
   - Expected: the container dropdown is empty, no pod-logs polling runs, and refetch errors with cached logs do not pop the dialog.
   - Test: Task 8, `pod_logs_inactive_without_container` and `refetch_error_with_cache_is_silent`.

---

## File Structure

```
Cargo.toml                    # bin crate "podlogs" v2.0.0, edition 2024, packager metadata
rust-toolchain.toml
assets/fonts/                 # 5 TTFs + OFL-IBMPlexSans.txt + OFL-JetBrainsMono.txt
assets/icon.png               # sidebar logo (was public/icon.png)
assets/icon_256x256.png       # About icon (was src-tauri/icons/icon_256x256.png)
icons/                        # packaging icons (was src-tauri/icons/) + icon.ico
src/main.rs                   # bootstrap: fonts, theme, runtime, menus, window
src/error.rs  src/models/  src/k8s/      # moved backend (unchanged logic)
src/api.rs                    # Api trait, KubeApi, with_timeout, FakeApi (cfg(test))
src/format.rs                 # timestamps, thousands, highlight_match, TimeRange, intervals
src/status.rs                 # pod status → BadgeVariant, deployment Health
src/persist.rs                # PersistedState load/save
src/app/query.rs              # Query<K,T> cache/poll/generation logic (pure)
src/app/search_state.rs       # log-search request state machine (pure)
src/app/store.rs              # Store entity: state, actions, tick, fetch, errors
src/app/root.rs               # PodLogsApp root view, overlays, ESC, menubar slot
src/app/sidebar.rs  status_bar.rs  deployments.rs  deployment_details.rs
src/app/pods.rs  pod_details.rs  log_viewer.rs  log_detail_modal.rs
src/app/log_search.rs  settings.rs  error_dialog.rs  menus.rs
src/ui/theme.rs               # Palette, tw colors, Tw trait, CurrentPalette global, fonts
src/ui/tracked_text.rs        # letter-spacing text element
src/ui/button.rs  input.rs  badge.rs  skeleton.rs  dropdown.rs  modal.rs  drawer.rs  collapsible.rs
.github/workflows/ci.yml       # build/test on every branch push (Task 4)
.github/workflows/release.yml  # packaging + release on main (Task 14)
```

---

### Task 1: Crate at repo root, Tauri and React removed, backend tests green

**Files:**
- Move: `src-tauri/src/{error.rs,k8s/,models/}` → `src/` with `git mv`
- Move: `src-tauri/icons/` → `icons/`. Move `public/icon.png` → `assets/icon.png`. Move `src-tauri/icons/icon_256x256.png` → `assets/icon_256x256.png`.
- Create: `Cargo.toml`, `rust-toolchain.toml`, `src/main.rs`
- Delete: `src-tauri/` (rest), all React/Vite/Node files (`src/**/*.ts*`, `src/index.css`, `src/test/`, `index.html`, `vite.config.*`, `vitest.config.*` if present, `tailwind.config.js`, `postcss.config.*`, `package.json`, `package-lock.json`, `tsconfig*.json`, `public/`)

**Interfaces:**
- Produces: the module tree `error`, `models`, `k8s::{client, clusters, namespaces, deployments, pods, logs}`, with signatures unchanged (e.g. `k8s::pods::get_pods(context: String, namespace: String, deployment: Option<String>) -> Result<Vec<PodInfo>, K8sError>`, `k8s::logs::get_pod_logs(context, namespace, pod_name, container: Option<String>, since_seconds: Option<i64>, tail_lines: Option<i64>)`, `k8s::logs::search_deployment_logs(context, namespace, deployment, keyword: Option<String>, log_level: Option<String>, since_seconds: Option<i64>) -> Result<LogSearchResponse, K8sError>`).

- [ ] **Step 1:** Write `rust-toolchain.toml` (`channel = "1.99.1"`, `profile = "minimal"`, `components = ["rustfmt","clippy"]`).
- [ ] **Step 2:** Write the root `Cargo.toml`:
  - `[package]` name `podlogs`, version `2.0.0`, edition `2024`.
  - Dependencies copied from `src-tauri/Cargo.toml`, minus `tauri*`. Add `gpui-kit = "=0.9.1"`, `chrono` (already present), `futures = "0.3"`.
  - Dev-deps: `http`, `tower` (as today) and `gpui-kit = { version = "=0.9.1", features = ["test-support"] }`.
  - The same `[profile.release]` as `src-tauri/Cargo.toml`.
- [ ] **Step 3:** `git mv` the backend. Delete every `#[tauri::command]` line. Keep `impl serde::Serialize for K8sError` (harmless). Keep the `#[path = "logs_tests.rs"]` include.
- [ ] **Step 4:** In `src/main.rs`, declare `mod error; mod k8s; mod models;` and `fn main() {}` with `#![allow(dead_code)]` (the UI comes later).
- [ ] **Step 5:** Run `cargo test`. Expected: the 7 tests in `k8s::logs::tests` pass (`finds_forecast_before_last_thousand_lines_and_matches_pod_log_parser` … `searches_all_regular_containers_with_at_most_four_requests_and_stable_order`), plus any existing unit tests in `pods.rs`.
- [ ] **Step 6:** Delete the React/Node/Tauri files listed above.
  - In `.gitignore`, add `/target` and remove `node_modules`/`dist` entries.
  - Expected: `cargo build` still passes, `git status` shows no `.ts`/`.tsx` files and nothing under `target/`.
- [ ] **Step 7:** Commit: `refactor: move Rust backend to repo root and remove Tauri/React`.

### Task 2: Api layer with timeout and test fake

**Files:**
- Create: `src/api.rs`

**Interfaces:**
- Consumes: the Task 1 backend functions.
- Produces:
  - `pub type ApiResult<T> = Result<T, String>;`
  - `pub const TIMEOUT_MESSAGE: &str = "Connection timed out after 30 seconds";`
  - `pub async fn with_timeout<T>(fut: impl Future<Output = Result<T, K8sError>>) -> ApiResult<T>`. Uses `tokio::time::timeout(Duration::from_secs(30), …)`; the error is `K8sError::to_string()` or `TIMEOUT_MESSAGE`.
  - `pub trait Api: Send + Sync + 'static`. Every method returns `BoxFuture<'static, ApiResult<_>>` (`futures::future::BoxFuture`) and takes owned `String`s:
    - `clusters() -> Vec<ClusterInfo>`
    - `namespaces(context) -> Vec<NamespaceInfo>`
    - `deployments(context, namespace) -> Vec<DeploymentInfo>`
    - `deployment_details(context, namespace, name) -> DeploymentDetails`
    - `pods(context, namespace, deployment: Option<String>) -> Vec<PodInfo>`
    - `pod_details(context, namespace, pod) -> PodDetails`
    - `pod_logs(context, namespace, pod, container: Option<String>, since_seconds: Option<i64>) -> Vec<LogEntry>` (passes `tail_lines: None`)
    - `search_logs(context, namespace, deployment, keyword: Option<String>, log_level: Option<String>, since_seconds: Option<i64>) -> LogSearchResponse`
  - `pub struct KubeApi { rt: tokio::runtime::Handle }` with `KubeApi::new(rt)`. Each method does `rt.spawn(with_timeout(k8s::…))` and maps `JoinError` with `.to_string()`. `clusters` skips the timeout.
  - `#[cfg(test)] pub mod fake`:
    - `pub enum Endpoint { Clusters, Namespaces, Deployments, DeploymentDetails, Pods, PodDetails, PodLogs, SearchLogs }`
    - `pub struct FakeApi` with:
      - `reply(&self, Endpoint, Result<serde_json::Value, String>)`: queues a ready reply.
      - `gate(&self, Endpoint) -> futures::channel::oneshot::Sender<Result<serde_json::Value, String>>`: queues a reply that resolves when the test sends.
      - `calls(&self, Endpoint) -> usize`.
      - `last_args(&self, Endpoint) -> Vec<String>`: `Debug` strings of the arguments.
    - An unqueued call returns `Err("no fake reply")`. Values are deserialized into the model type.

- [ ] **Step 1:** Write the failing tests in `src/api.rs`:

```rust
#[tokio::test(start_paused = true)]
async fn timeout_maps_to_exact_message() {
    let r: ApiResult<()> = with_timeout(std::future::pending()).await;
    assert_eq!(r.unwrap_err(), "Connection timed out after 30 seconds");
}
#[tokio::test]
async fn backend_error_uses_display_text() {
    let r: ApiResult<()> = with_timeout(async { Err(K8sError::NoKubeconfig) }).await;
    assert_eq!(r.unwrap_err(), "No kubeconfig found");
}
#[tokio::test]
async fn fake_gate_resolves_when_sent() {
    let fake = fake::FakeApi::default();
    let tx = fake.gate(fake::Endpoint::Pods);
    let fut = fake.pods("c".into(), "ns".into(), None);
    tx.send(Ok(serde_json::json!([]))).unwrap();
    assert!(fut.await.unwrap().is_empty());
    assert_eq!(fake.calls(fake::Endpoint::Pods), 1);
}
```

- [ ] **Step 2:** Run `cargo test api::`. Expected: FAIL (items not defined).
- [ ] **Step 3:** Implement the interfaces above.
- [ ] **Step 4:** Run `cargo test api::`. Expected: 3 passed.
- [ ] **Step 5:** Commit: `feat: add Api trait with 30s timeout and test fake`.

### Task 3: Pure helpers (formatting, status, error text, persistence)

**Files:**
- Create: `src/format.rs`, `src/status.rs`, `src/persist.rs`, `src/app/mod.rs` (empty module list for now), `src/app/error_dialog.rs` (only `describe_connection_error` + its tests in this task)

**Interfaces:**
- Produces from `format.rs`:
  - `format_timestamp(ts: Option<&str>) -> String` and `format_timestamp_in<Tz: TimeZone>(ts: Option<&str>, tz: &Tz) -> String`:
    - Output `yyyy-MM-dd HH:mm:ss` in local time.
    - `None` → `"-"`.
    - Unparseable input → returned unchanged.
  - `format_short_timestamp(..)` and `format_short_timestamp_in(..)`: same rules, output `HH:mm:ss`.
  - `thousands(n: usize) -> String`.
  - `highlight_match(text: &str, keyword: &str) -> Option<(String, String, String)>`: first case-insensitive match; `None` for an empty keyword.
  - `enum TimeRange { M15, M30, H1, H3, H6, H12, H24, H48 }`:
    - `TimeRange::ALL`
    - `.seconds() -> i64` (900, 1800, 3600, 10800, 21600, 43200, 86400, 172800)
    - `.label() -> &'static str` ("Last 15 minutes" … "Last 48 hours")
    - `.key() -> &'static str` ("15m" … "48h")
  - `REFRESH_INTERVALS_MS: [u32; 5] = [5000, 10000, 15000, 20000, 30000]` and `refresh_label(ms) -> String` ("5 seconds").
- Produces from `status.rs`:
  - `enum BadgeVariant { Success, Warning, Error, Terminating, Default }`
  - `pod_status_variant(status: &str, ready: Option<&str>) -> BadgeVariant`: the rules of `c70bd05:src/components/common/Badge.tsx` `getStatusVariant`.
  - `enum Health { Healthy, Degraded, Unhealthy }` with `.label()` and `health(replicas: i32, available: i32) -> Health` (equal → Healthy, available > 0 → Degraded, else Unhealthy).
- Produces from `persist.rs`:
  - `#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Debug)] #[serde(default)] pub struct PersistedState { theme: ThemePref, sidebar_collapsed: bool, refresh_interval_ms: u32, context: String, namespace: String, deployment: Option<String>, last_namespace_by_cluster: BTreeMap<String, String> }`.
  - Defaults: theme `Dark`, interval 5000, namespace `"default"`.
  - `ThemePref { Light, Dark, System }` lives here.
  - `load(path: &Path) -> PersistedState`: defaults on a missing or corrupt file.
  - `save(path: &Path, s: &PersistedState) -> std::io::Result<()>`: creates parent dirs and writes via a temp file + rename.
  - `default_path() -> Option<PathBuf>`.
- Produces from `error_dialog.rs`: `describe_connection_error(raw: &str) -> (String, String)`, returning (message, details). It applies the mapping of `c70bd05:src/components/common/ErrorDialog.tsx`, checked in its order, and also matches `"timed out"` together with `"timeout"`. The generic fallback is `("Failed to connect to Kubernetes cluster", raw)`.

- [ ] **Step 1:** Write the failing tests, one `#[cfg(test)] mod tests` per file. Pinned values:

```rust
// format.rs
let utc = chrono::Utc;
assert_eq!(format_timestamp_in(Some("2026-01-09T17:22:13Z"), &utc), "2026-01-09 17:22:13");
assert_eq!(format_timestamp_in(Some("2026-01-09T17:22:13Z"), &chrono::FixedOffset::east_opt(7*3600).unwrap()), "2026-01-10 00:22:13");
assert_eq!(format_timestamp_in(None, &utc), "-");
assert_eq!(format_timestamp_in(Some("not a date"), &utc), "not a date");
assert_eq!(format_short_timestamp_in(Some("2026-01-09T17:22:13.123456Z"), &utc), "17:22:13");
assert_eq!(thousands(0), "0"); assert_eq!(thousands(1234567), "1,234,567"); assert_eq!(thousands(999), "999");
assert_eq!(highlight_match("Hello World", "WORLD"), Some(("Hello ".into(), "World".into(), "".into())));
assert_eq!(highlight_match("abc", ""), None); assert_eq!(highlight_match("abc", "x"), None);
assert_eq!(TimeRange::H48.seconds(), 172800); assert_eq!(TimeRange::M30.label(), "Last 30 minutes");
// status.rs
assert_eq!(pod_status_variant("Running", Some("0/1")), BadgeVariant::Warning);
assert_eq!(pod_status_variant("Running", Some("1/1")), BadgeVariant::Success);
assert_eq!(pod_status_variant("Terminating", Some("1/1")), BadgeVariant::Terminating);
assert_eq!(pod_status_variant("ContainerCreating", None), BadgeVariant::Warning);
assert_eq!(pod_status_variant("ContainerStatusUnknown", None), BadgeVariant::Error);
assert_eq!(pod_status_variant("Unknown", None), BadgeVariant::Default);
assert_eq!(health(0, 0), Health::Healthy); assert_eq!(health(3, 1), Health::Degraded); assert_eq!(health(3, 0), Health::Unhealthy);
// error_dialog.rs
assert_eq!(describe_connection_error("No kubeconfig found").0, "Kubernetes configuration not found");
assert_eq!(describe_connection_error("Connection timed out after 30 seconds").0, "Connection timed out");
assert_eq!(describe_connection_error("Kubernetes error: ApiError: Unauthorized (401)").0, "Authentication failed");
assert_eq!(describe_connection_error("Kubernetes error: Forbidden").0, "Access denied");
assert_eq!(describe_connection_error("weird").0, "Failed to connect to Kubernetes cluster");
assert_eq!(describe_connection_error("weird").1, "weird");
// persist.rs
let dir = std::env::temp_dir().join(format!("podlogs-test-{}", std::process::id()));
assert_eq!(load(&dir.join("missing.json")), PersistedState::default());
std::fs::create_dir_all(&dir).unwrap(); std::fs::write(dir.join("bad.json"), "{nope").unwrap();
assert_eq!(load(&dir.join("bad.json")), PersistedState::default());
let s = PersistedState { context: "prod".into(), ..Default::default() };
save(&dir.join("a/state.json"), &s).unwrap(); assert_eq!(load(&dir.join("a/state.json")), s);
std::fs::write(dir.join("partial.json"), r#"{"context":"x"}"#).unwrap();
assert_eq!(load(&dir.join("partial.json")).refresh_interval_ms, 5000);
```

- [ ] **Step 2:** Run `cargo test -- format:: status:: persist:: app::error_dialog::`. Expected: FAIL.
- [ ] **Step 3:** Implement. For `persist.rs`, `Default` must be implemented by hand for the 5000 and "default" values.
- [ ] **Step 4:** Run the same command. Expected: all pass.
- [ ] **Step 5:** Commit: `feat: add formatting, status, error-text and persistence helpers`.

### Task 4: Theme, fonts and an empty window on screen

**Files:**
- Create: `src/ui/mod.rs`, `src/ui/theme.rs`, `assets/fonts/*` (TTFs from IBM/plex `packages/plex-sans/fonts/complete/ttf/` and JetBrains/JetBrainsMono `fonts/ttf/`, plus both OFL texts), `.github/workflows/ci.yml`
- Modify: `src/main.rs`

**Interfaces:**
- Produces:
  - `#[derive(Clone, Copy)] pub struct Palette { pub dark: bool, pub bg1, bg2, bg3, text1, text2, muted, border, border_subtle, accent_subtle: Hsla }` and `palette(dark: bool) -> Palette` with the Global Constraints values.
  - `pub mod tw`, constants `u32`:
    - `EMERALD_400/500/600`, `AMBER_400/500/600`, `RED_400/500/600`, `PURPLE_400/500/600`, `GRAY_400/500`, `BLUE_500`, `CYAN_400/500` (Tailwind v3 hex values)
    - `ACCENT = 0x0EA5E9`, `ACCENT_HOVER = 0x0284C7`
  - `hex(u32) -> Hsla` and `hexa(u32, alpha: f32) -> Hsla`.
  - `pub trait Tw: Styled + Sized` with `t_xs`, `t_sm`, `t_base`, `t_lg` (size + line-height pairs) and `mono()`, blanket-implemented for all `Styled`.
  - `pub struct CurrentPalette(pub Palette)` as a gpui `Global`, with `pal(cx: &App) -> Palette`.
  - `pub fn set_palette(p: Palette, cx: &mut App)`: sets the global and pushes into gpui-component `Theme` (mode, font family "IBM Plex Sans", size 14, mono "JetBrains Mono", radius 6, colors `background, foreground, muted_foreground, border, input, ring = accent, caret, selection = accent/0.3`).
  - `pub fn load_fonts(cx: &mut App)`: embeds via `include_bytes!` and calls `cx.text_system().add_fonts`.
- `main.rs`:
  - Calls `gpui_kit::application().with_assets(<icon source from Task 6>).run(…)`. Until Task 6, use `gpui_kit::assets::Assets`.
  - Calls `gpui_kit::init`, `load_fonts`, `set_palette(palette(true))`, and opens a window per the Global Constraints that renders a full-size div with `bg1`.
  - `cmd-q`/`ctrl-q` quits; closing the window quits (`cx.on_window_closed(|cx, _| cx.quit())`).

- [ ] **Step 1:** Write a failing unit test `palette_matches_index_css`: `palette(true).bg1 == hex(0x0F172A)`, `palette(false).bg1 == hex(0xFDF6E3)`, `palette(false).border == hex(0xD3CDB8)`, `palette(true).accent_subtle == hex(0x0C4A6E)`.
- [ ] **Step 2:** Run `cargo test ui::theme`. Expected: FAIL.
- [ ] **Step 3:** Implement `theme.rs` and update `main.rs`.
- [ ] **Step 4:** Run `cargo test ui::theme`. Expected: PASS. Then run `cargo run`: a maximized window titled "PodLogs" with a `#0F172A` background. Cmd-Q quits, and closing the window quits.
- [ ] **Step 5: Early cross-platform CI** (spec §9: Linux/Windows green early). Create `.github/workflows/ci.yml`:
  - Triggers: `push` to non-`main` branches and `pull_request`.
  - Matrix: `macos-latest`, `ubuntu-22.04` and `windows-latest`. Each job runs `rustup show` then `cargo build --locked`.
  - Ubuntu also runs `cargo test --locked` after installing `libxkbcommon-x11-dev libwayland-dev libx11-xcb-dev libfontconfig-dev libvulkan1` (add whatever the linker asks for).
  - Windows uses `continue-on-error: true`.
  - Push the branch. Expected: macOS and Linux green; Windows result noted. From here on every task's push keeps macOS and Linux green.
- [ ] **Step 6:** Commit: `feat: theme palette, embedded fonts, main window and CI`.

### Task 5: Query cache and polling rules

**Files:**
- Create: `src/app/query.rs`

**Interfaces:**
- Produces:
  - `pub struct Query<K, T>` where `K: Clone + Eq + Hash`, `T: Clone`. Methods:
    - `new(stale: Duration, poll: bool) -> Self`. Stale is ∞ for clusters (`Duration::MAX`), 60 s for namespaces, 30 s for deployments, 0 for the others. Poll is true for deployments, pods and pod-logs.
    - `set_active(&mut self, key: Option<K>, now: Instant)`: on a key change, marks a fresh activation and re-anchors the poll timer to `now`.
    - `data(&self) -> Option<&T>`: cached entry for the active key; it survives refetches and errors.
    - `is_loading(&self) -> bool`: in flight for the active key with no data.
    - `is_fetching(&self) -> bool`
    - `error(&self) -> Option<&str>`
    - `needs_fetch(&self, now: Instant, interval: Duration) -> bool`
    - `start(&mut self, now: Instant) -> Option<(K, u64)>`
    - `finish(&mut self, key: &K, gen: u64, result: Result<T, String>, now: Instant) -> Outcome`
    - `cancel(&mut self)`
    - `clear(&mut self)`: drops all cached entries and errors, used by Retry.
  - `pub enum Outcome { Stale, Ok, Err }`.
  - `pub trait CacheEmpty { fn cache_empty(&self) -> bool }`, implemented for `Vec<T>` (`is_empty`) and for `PodDetails`/`DeploymentDetails` (`false`).
  - Rule for `needs_fetch`: false when nothing is active or when a fetch is in flight for the active key. Otherwise true if either:
    - (a) a fresh activation and the cached entry is missing or at least `stale` old; or
    - (b) `poll` is set and `now - anchor >= interval`, where the anchor is the last `start` or activation.

    Errors do not re-trigger a fetch except through (b).

- [ ] **Step 1:** Write the failing tests, each a few lines over `Query<String, Vec<u8>>` and a hand-made `Instant` base:
  - `fetches_on_first_activation`
  - `fresh_cache_skips_refetch_within_stale_time`: 30 s stale, switch A→B→A within 10 s → no fetch for A.
  - `zero_stale_refetches_on_switch_back_but_keeps_showing_cache`: `data()` is `Some` while `is_fetching()`.
  - `polls_after_interval_from_last_start`: not due at 4.9 s, due at 5 s.
  - `no_overlap_while_in_flight`
  - `stale_generation_is_ignored`: `start`, then `cancel` (or a key change), then `finish` with the old gen → `Outcome::Stale` and the data is unchanged.
  - `error_keeps_previous_data`
  - `no_refetch_after_error_without_poll`
  - `clear_drops_cache`
- [ ] **Step 2:** Run `cargo test app::query`. Expected: FAIL.
- [ ] **Step 3:** Implement `Query`. Bookkeeping: `entries: HashMap<K, (T, Instant)>`, `active`, `fresh`, `anchor`, `in_flight: Option<(K, u64)>`, `next_gen`, `error`. *ponytail:* the cache has no GC; per-key entries are tiny. Add a GC if the RAM gate in Task 16 says otherwise.
- [ ] **Step 4:** Run `cargo test app::query`. Expected: all 9 pass.
- [ ] **Step 5:** Commit: `feat: Query cache with stale/poll/generation rules`.

### Task 6: Letter-spacing text element (critical path)

**Files:**
- Create: `src/ui/tracked_text.rs`

**Interfaces:**
- Produces:
  - `pub fn tracked(text: impl Into<SharedString>) -> TrackedText`: 0.05 em by default, `.spacing_em(f32)` to override. It uses the inherited text style (`window.text_style()`: font, size, weight, color, line height).
  - `TrackedText` implements gpui `Element` + `IntoElement`:
    - Layout: `window.request_measured_layout`. Width = `shaped.width + spacing * char_count`, height = line height.
    - Paint: `window.text_system().shape_line(..)`, then for each `run.glyphs[i]`, call `window.paint_glyph(origin + point(glyph.position.x + spacing * chars_before(glyph.index), baseline), run.font_id, glyph.id, font_size, color)`. Use the same baseline that `ShapedLine::paint` uses (read `gpui-pre-0.3.12/src/text_system/line.rs`).
  - Pure helpers `tracked_width(shaped: f32, chars: usize, spacing: f32) -> f32` and `glyph_offset(text: &str, byte_index: usize, spacing: f32) -> f32`.
- Also in this task: `src/ui/mod.rs` exports `section_label(text) -> impl IntoElement` = `tracked(text)` with `t_xs`, weight 500 and `muted` color. Callers pass the uppercase string.

- [ ] **Step 1:** Write the failing tests:

```rust
assert!((tracked_width(100.0, 8, 0.6) - 104.8).abs() < 1e-4);
assert_eq!(glyph_offset("CLUSTER", 0, 0.6), 0.0);
assert!((glyph_offset("CLUSTER", 3, 0.6) - 1.8).abs() < 1e-4);
assert!((glyph_offset("ÉA", 2, 0.6) - 0.6).abs() < 1e-4); // byte index 2 = 1 char before
```

- [ ] **Step 2:** Run `cargo test ui::tracked_text`. Expected: FAIL.
- [ ] **Step 3:** Implement the element.
- [ ] **Step 4:** Run `cargo test ui::tracked_text`. Expected: PASS.
- [ ] **Step 5 (gate, manual, macOS):**
  1. Temporarily render `section_label("CLUSTER")`, `"NAMESPACE"` and `"DEPLOYMENTS"` at sidebar positions (x = 16 px, sidebar bg).
  2. Take 2× screenshots of both windows (Cmd-Shift-4 then Space; the terminal may lack Screen Recording permission for `screencapture`). Overlay them or measure the glyph x positions.
  3. Pass: every glyph is within 1 device pixel. **If it fails, stop and report to the owner before Task 7. Do not build screens on top of it.**
- [ ] **Step 6:** Commit: `feat: tracked text element for Tailwind letter-spacing`.

### Task 7: Shared widgets

**Files:**
- Create: `src/ui/button.rs`, `input.rs`, `badge.rs`, `skeleton.rs`, `dropdown.rs`, `modal.rs`, `drawer.rs`, `collapsible.rs`, `icons.rs`
- Modify: `src/main.rs` (asset source)

**Interfaces** (each reads `pal(cx)`; values come from the React file named after each item):
- **Button** (`button.rs`, from `c70bd05:src/components/common/Button.tsx`):
  - `#[derive(IntoElement)] pub struct Button`; `Button::new(id, Variant)`.
  - Builders: `.size(BtnSize)`, `.icon(IconName)` (16 px, `mr-1`, or `mr-2` via `.icon_gap(8.)` for the Search button), `.label(impl Into<SharedString>)`, `.disabled(bool)`, `.tooltip(&'static str)`, `.on_click(impl Fn(&ClickEvent, &mut Window, &mut App) + 'static)`.
  - `enum Variant { Primary, Secondary, Ghost }`, `enum BtnSize { Sm, Md, Lg, Icon }`.
  - Disabled: opacity 0.5, no hover, no click, no tooltip.
- **Inputs** (`input.rs`): `search_input(state: &Entity<InputState>, cx: &App) -> Input` (Search icon prefix, h 36, radius 6, bg3, border) and `text_input(..)` without the icon. Disable autocorrect/spellcheck where gpui-component exposes it.
- **Badges** (`badge.rs`, from `Badge.tsx`):
  - `status_badge(status: &str, ready: Option<&str>, cx: &App)`
  - `level_badge(level: &str, cx: &App)`: min width 52, white text; ERROR is red-500 in light and red-600 in dark, WARN amber-500, INFO blue-500, DEBUG purple-500.
  - `label_badge(text, cx)`
  - `dot(color: Hsla, size_px: f32)`
- **Skeletons** (`skeleton.rs`, from `Skeleton.tsx`): `table_skeleton(rows: usize, cx)` and `log_skeleton(rows: usize, cx)`. Pulse animation: 2 s, opacity 1 → 0.5 → 1.
- **Dropdown** (`dropdown.rs`, from `Dropdown.tsx`; the spike version is the reference behavior):
  - `pub struct DropdownState`:
    - `new(placeholder: &str, search_placeholder: Option<&str>, window, cx)`; `Some` makes it searchable.
    - `set_options(Vec<(String, String)>, cx)` with (value, label) pairs; `set_value(Option<String>, cx)`; `set_disabled(bool, cx)`; `value() -> Option<&str>`.
    - Implements `Render` and `EventEmitter<DropdownEvent>`.
  - `pub enum DropdownEvent { Change(String) }`.
  - Behavior:
    - Trigger: h 36, chevron rotates 180° when open.
    - Popup: 4 px below, `max-h 300`, `shadow-md`.
    - Search box focused on open.
    - Selected option shows Check + accent color; other options get a 16 px spacer.
    - "No results found" when the filter matches nothing.
    - A click outside closes the popup and clears the search; ESC closes it.
- **Modal** (`modal.rs`, from `Modal.tsx`): `pub fn modal(id, title, size: ModalSize, persistent: bool, on_close: Rc<dyn Fn(&mut Window, &mut App)>, body: AnyElement, window, cx) -> AnyElement`. Implemented as a deferred window-anchored overlay. The backdrop is black at 0.5 (light) or 0.7 (dark), and backdrop clicks are ignored when `persistent`. Body max height 85 % of the viewport.
- **Drawer** (`drawer.rs`, from `Drawer.tsx`):
  - `pub fn drawer(id, title, open: bool, on_close, body: AnyElement, window, cx) -> AnyElement`.
  - Width 480, bg2, left border. Backdrop black at 0.2 (light) or 0.4 (dark).
  - Slides from `translate-x-full` and fades the backdrop over 300 ms ease-out, both opening and closing (keep it rendered until the close animation ends).
  - Title truncated, X ghost icon button.
- **Collapsible** (`collapsible.rs`): `collapsible_header(id, title: &str, open: bool, on_toggle) -> impl IntoElement`. Chevron rotated −90° when closed, tracked uppercase xs label, hover → text2.
- **Icons** (`icons.rs`): `icon_assets!(pub AppIcons, [...])` composed with `gpui_kit::assets::Assets` into an `AssetSource` that also serves `assets/icon.png` and `assets/icon_256x256.png`. Icons: Layers, Search, Moon, Sun, Settings, ChevronDown, Check, ArrowLeft, Download, Copy, X, AlertTriangle (Lucide `triangle-alert`), RefreshCw, Package, PanelLeftClose, PanelLeftOpen.

- [ ] **Step 1:** Write failing gpui UI tests with `#[gpui_kit::test]` (see `~/.cargo/registry/src/*/gpui-kit-0.9.1/TESTING.md`; add `.test_support()` IDs on interactive parts):
  - `dropdown_opens_selects_and_emits_change`: click the trigger, click the option "b" → a `Change("b")` event, popup closed.
  - `dropdown_click_outside_closes_and_clears_search`
  - `dropdown_search_filters_options_case_insensitive`: "PROD" matches "prod-backend"; "zzz" → "No results found" is present.
  - `button_disabled_ignores_click`
  - `persistent_modal_ignores_backdrop_click` and `modal_backdrop_click_closes`
- [ ] **Step 2:** Run `cargo test ui::`. Expected: FAIL.
- [ ] **Step 3:** Implement the widgets, then switch `main.rs` to the new asset source.
- [ ] **Step 4:** Run `cargo test ui::`. Expected: all pass.
- [ ] **Step 5 (manual, macOS):** Add a temporary gallery (`PODLOGS_GALLERY=1`) that shows every widget in both themes. Compare against the Tauri app's equivalents: Pods header buttons, the namespace dropdown open, the log detail modal, the settings drawer. Remove the gallery before committing.
- [ ] **Step 6:** Commit: `feat: shared widgets (button, input, badges, skeleton, dropdown, modal, drawer)`.

### Task 8: Store entity (state, navigation, fetching, errors, persistence)

**Files:**
- Create: `src/app/store.rs`

**Interfaces:**
- Consumes: `Api` (Task 2), `Query`/`Outcome`/`CacheEmpty` (Task 5), `PersistedState`/`persist::save`/`ThemePref` (Task 3), `TimeRange` (Task 3).
- Produces:
  - `pub enum View { Deployments, Pods, Logs, Search }` and `pub enum DrawerKind { Pod(String), Deployment(String), Settings }`.
  - `pub struct Store` with these public fields (read-only by convention):
    - `theme: ThemePref`, `sidebar_collapsed: bool`, `refresh_interval_ms: u32`
    - `view: View`, `drawer: Option<DrawerKind>`, `log_viewer_pod: Option<String>`
    - `context: String`, `namespace: String`, `deployment: Option<String>`
    - `log_container: Option<String>`, `log_time_range: TimeRange`
    - `error: Option<String>`, `countdown_s: u32`, `os_dark: bool`
    - queries: `clusters`, `namespaces`, `deployments`, `pods`, `pod_details`, `deployment_details`, `pod_logs`
  - Constructor and tick:
    - `Store::new(api: Arc<dyn Api>, persisted: PersistedState, persist_path: Option<PathBuf>, cx: &mut Context<Self>) -> Self`. Starts a `cx.spawn` loop that calls `tick` every 1 s using `cx.background_executor().timer`. Time always comes from `cx.background_executor().now()`.
    - `tick(&mut self, cx)`: count down `countdown_s` (floor 0), recompute active keys, start due fetches.
  - Actions (each ends with `cx.notify()`; persisted fields are saved):
    - `set_context(&mut self, ctx: String, cx)`: namespace = remembered value or "default", deployment = None, starts a transition.
    - `set_namespace(ns, cx)`: deployment = None, remembers the namespace per context, starts a transition.
    - `toggle_deployment_and_show_pods(name, cx)`: sidebar click.
    - `set_deployment(Option<String>, cx)`
    - `set_view(View, cx)`: clears `log_viewer_pod`.
    - `open_log_viewer(pod, cx)`: view Logs, container None, time range M30.
    - `close_log_viewer(cx)`: view Pods.
    - `select_pod(Option<String>, cx)` and `select_deployment_info(Option<String>, cx)`: open/close the drawer.
    - `open_settings(cx)`, `close_drawer(cx)`
    - `set_theme(ThemePref, cx)`; `toggle_theme(cx)`: Dark → Light, anything else → Dark; `is_dark(&self) -> bool` (`Dark`, or `System` with `os_dark`).
    - `set_os_dark(bool, cx)`, `toggle_sidebar(cx)`
    - `set_refresh_interval(ms, cx)`: resets the countdown.
    - `set_log_container(String, cx)`, `set_log_time_range(TimeRange, cx)`
    - `dismiss_error(cx)`
    - `retry_connection(cx)`: clears the error, `clear()`s clusters/namespaces/deployments/pods, then ticks immediately.
  - Behavior:
    - **Active keys:**
      - clusters: always.
      - namespaces: context non-empty.
      - deployments and pods: context + namespace non-empty; pods keyed with the deployment.
      - pod_details: drawer `Pod(name)`, or view Logs with `log_viewer_pod`.
      - deployment_details: drawer `Deployment(name)`.
      - pod_logs: view Logs, a pod, and `log_container` is `Some`. The key holds `TimeRange.seconds()`.
    - **Container default:** once pod_details for the log pod loads and `log_container` is None, use the first container. With zero containers it stays None.
    - **Clusters loaded:** if `context` is empty, pick the cluster with `is_current`, else the first.
    - **Namespaces loaded:** if `namespace` is not in the list, use "default" if present, else the first.
    - **Transition** (context/namespace change):
      - Errors are ignored for 1 s and the current error is cleared.
      - pod_logs and pod_details are canceled.
      - View Logs or Pods → Deployments.
      - Pod drawer → closed.
    - **Error reporting:** after `finish` returns `Outcome::Err`, set `error` only if not transitioning and `query.data()` is None or `cache_empty()`.
    - **pod_logs retry:** retried once after 1 s on error (React Query `retry: 1`); all other queries never retry.
    - **Countdown:** reset to `refresh_interval_ms / 1000` when a pods fetch finishes, whatever the outcome.

- [ ] **Step 1:** Write failing tests in `#[gpui_kit::test]` style (`TestAppContext`, `FakeApi`, `cx.executor().advance_clock`, `run_until_parked`):
  - `startup_picks_current_context_and_fetches_namespaces`
  - `namespace_fallback_to_default_then_first`
  - `store_reports_missing_kubeconfig_on_startup`: clusters → `Err("No kubeconfig found")` → `error == Some("No kubeconfig found")`.
  - `refetch_error_with_cache_is_silent`: pods ok, then a later pods poll errs → `error` None.
  - `errors_ignored_during_transition`
  - `late_pods_after_namespace_switch_are_dropped`: gate pods for ns A, switch to B, release A → `pods.data()` never shows A's list and `error` stays None.
  - `slow_fetch_does_not_overlap_and_resets_countdown`: gate pods 12 s at a 5 s interval → 1 call; release → countdown == 5.
  - `namespace_change_returns_to_deployments_and_closes_pod_drawer`
  - `log_container_defaults_to_first_container`
  - `pod_logs_inactive_without_container`: zero containers → 0 PodLogs calls after 20 s.
  - `pod_logs_retries_once_then_reports`: 2 calls, then `error` set.
  - `retry_connection_refetches_core_queries`
  - `toggle_theme_cycle`: System → Dark → Light → Dark.
  - `persists_changes_to_disk`: a temp path; after `set_namespace`, `persist::load` shows it plus `last_namespace_by_cluster`.
- [ ] **Step 2:** Run `cargo test app::store`. Expected: FAIL.
- [ ] **Step 3:** Implement `Store`. Fetch pattern: `let fut = api.pods(..); cx.spawn(async move |this, cx| { let r = fut.await; this.update(cx, |s, cx| s.on_pods(key, gen, r, cx)) })`.
- [ ] **Step 4:** Run `cargo test app::store`. Expected: all 14 pass.
- [ ] **Step 5:** Commit: `feat: Store entity with query polling, transitions and persistence`.

### Task 9: App shell — root view, sidebar (incl. collapse), status bar, error dialog, menus

**Files:**
- Create: `src/app/root.rs`, `sidebar.rs`, `status_bar.rs`, `menus.rs`
- Modify: `src/app/error_dialog.rs` (add the dialog view), `src/main.rs`

**Interfaces:**
- Consumes: `Store` (Task 8), widgets (Task 7), `tracked`/`section_label` (Task 6).
- Produces:
  - `pub struct PodLogsApp` (root view): `new(store: Entity<Store>, window, cx)`. Observes the store; on store change it calls `set_palette(palette(store.is_dark()))`. Observes window appearance → `store.set_os_dark`. Owns the child view entities. Renders: [menubar on non-macOS] + sidebar + (main view by `store.view`) + status bar + drawers + Connection Error dialog.
  - Actions `actions!(podlogs, [Quit, About, Hide, HideOthers, ShowAll, Minimize, Zoom, CloseWindow, CloseOverlay])`. Escape → `CloseOverlay` closes, in order: an open dropdown, the log detail modal, the drawer. The error dialog ignores Escape.
  - `menus::install(cx)`:
    - macOS: `cx.set_menus` (PodLogs/Edit/Window as in `c70bd05:src-tauri/src/lib.rs`; Edit items dispatch gpui-component input actions; Services via `SystemMenuType::Services`).
    - Others: `menus::menu_bar(window, cx) -> Entity<AppMenuBar>` rendered at the top of the root.
    - About: use a native about panel if gpui-pre exposes one (search `about` in `gpui-pre-0.3.12/src`); otherwise a `modal` (size sm) with the 64 px `icon_256x256.png`, "PodLogs" and `Version {CARGO_PKG_VERSION}`.
  - `pub struct SidebarView`: `new(store, window, cx)`. Owns the cluster `DropdownState`, the namespace `DropdownState` (searchable, "Search namespaces...") and the deployment filter `InputState`.
    - Expanded layout and states per `c70bd05:src/components/layout/Sidebar.tsx`. Labels via `section_label`.
    - New `PanelLeftClose` ghost icon button left of the theme toggle.
    - Collapsed 60 px rail per the same file, with a `PanelLeftOpen` ghost button above the theme toggle. Nav tooltips show the item label.
  - `status_bar(store: &Store, cx: &App) -> impl IntoElement`, per `StatusBar.tsx`. "Showing N pod(s)" counts `pods.data()` on the Pods view only. "Refresh in Ns" is hidden while `pods.is_fetching()`.
  - `error_dialog::connection_error(raw: &str, on_dismiss, on_retry, window, cx) -> AnyElement`: a persistent `modal` (md), body per `ErrorDialog.tsx`, text from `describe_connection_error`.
- `main.rs`: builds the tokio runtime (multi-thread, 2 workers, `enable_all`), `KubeApi`, loads `persist::default_path()`, creates the `Store`, opens the window with `PodLogsApp`, and calls `menus::install`.

- [ ] **Step 1:** Write failing UI tests (FakeApi-backed Store):
  - `sidebar_click_deployment_selects_and_shows_pods_then_toggles_off`
  - `sidebar_filter_hides_non_matching_and_shows_empty_text`: "No deployments found".
  - `collapse_toggle_switches_rail_and_persists`
  - `status_bar_countdown_hidden_while_fetching`
  - `error_dialog_shows_mapped_message_and_retry_refetches`
  - `escape_closes_drawer_but_not_error_dialog`
- [ ] **Step 2:** Run `cargo test app::`. Expected: the new tests FAIL.
- [ ] **Step 3:** Implement. Until Tasks 10–13 land, main views are `div()` placeholders.
- [ ] **Step 4:** Run `cargo test`. Expected: all pass.
- [ ] **Step 5 (manual, macOS, real cluster):**
  1. Run `cargo run` beside the Tauri app, both themes.
  2. Check sidebar, status bar and error dialog (rename `~/.kube/config` temporarily to trigger it, then restore). Check menus and About.
  3. Fix every visible difference in color, spacing or position.
- [ ] **Step 6:** Commit: `feat: app shell with sidebar, status bar, error dialog and menus`.

### Task 10: Deployments view, Deployment drawer, Settings drawer

**Files:**
- Create: `src/app/deployments.rs`, `deployment_details.rs`, `settings.rs`
- Modify: `src/app/root.rs` (route the view, the drawers, and the settings buttons)

**Interfaces:**
- Produces:
  - `DeploymentsView::new(store, window, cx)`: owns its search `InputState`.
    - Per `DeploymentList.tsx`: column widths Replicas/Available/Ready/Status 128 px each (`w-32`), Actions 144 px; table headers via `tracked`.
    - Row click → `set_deployment(Some)` + `set_view(Pods)`. The Info button stops propagation → `select_deployment_info`.
    - Skeleton while `deployments.is_loading()`; empty state with Layers 48 px.
  - `DeploymentDetailsView::new(store, window, cx)`: Annotations collapsed and Conditions expanded by default, per `DeploymentDetails.tsx`. The drawer title is the deployment name. Annotation and condition texts are selectable (gpui-component selectable text, `text::…selectable(true)`).
  - `SettingsView::new(store, window, cx)`: Theme dropdown (Dark/Light/System) and Refresh Interval dropdown (5/10/15/20/30 seconds), helper texts per `Settings.tsx`.
- [ ] **Step 1:** Write failing UI tests:
  - `deployment_row_click_navigates_to_pods`
  - `deployment_info_opens_drawer_without_navigating`
  - `settings_interval_change_updates_store_and_countdown`
  - `deployment_health_text_and_colors`: Healthy/Degraded/Unhealthy labels present for 3/3, 3/1, 3/0.
- [ ] **Step 2:** Run `cargo test -- app::deployments app::settings`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run `cargo test`. Expected: all pass.
- [ ] **Step 5 (manual, macOS):** Side-by-side parity in both themes: list, hover, skeleton (throttle with a slow context), empty search, drawer open/close animation, settings. Fix the differences.
- [ ] **Step 6:** Commit: `feat: deployments view, deployment drawer and settings`.

### Task 11: Pods view and Pod drawer

**Files:**
- Create: `src/app/pods.rs`, `pod_details.rs`
- Modify: `src/app/root.rs`

**Interfaces:**
- Produces:
  - `PodsView::new(store, window, cx)`, per `PodList.tsx`:
    - Header "Pods" + the deployment name in accent; search w 256 + settings.
    - Columns: Name (flex, mono), Status, Ready (center), Restarts (center; red-500 light / red-400 dark when > 0), Age (right), IP (right, mono, `-` if empty), Actions (right, Logs primary + Info secondary, size sm, gap 8).
    - Logs/Info are disabled for Terminating/ContainerCreating/Evicted.
    - Column widths are fixed at the widest real content (spec §7). Measure the "Actions" and "IP" content with `window.text_system()` once per render if fixed values clip.
  - `PodDetailsView::new(store, window, cx)`, per `PodDetails.tsx`:
    - Container cards with env vars collapsed by default; `<secret>` is shown for `source == "Secret"`.
    - Conditions collapsed.
    - Image, env values and labels are selectable text.
- [ ] **Step 1:** Write failing UI tests:
  - `pods_search_filters_case_insensitive`
  - `logs_button_opens_log_viewer_for_pod`
  - `disabled_actions_for_terminating_pod`
  - `secret_env_values_masked`
  - `restarts_colored_only_when_positive`: assert the color via the snapshot style if exposed; otherwise unit-test `restarts_color(n, dark)` in `pods.rs`.
- [ ] **Step 2:** Run `cargo test -- app::pods app::pod_details`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run `cargo test`. Expected: all pass.
- [ ] **Step 5 (manual, macOS):** Side-by-side parity in both themes, including CrashLoopBackOff/Pending/Terminating badges on a test namespace, and a long pod name at a 1000 px window width.
- [ ] **Step 6:** Commit: `feat: pods view and pod details drawer`.

### Task 12: Log viewer, Log detail modal, Export

**Files:**
- Create: `src/app/log_viewer.rs`, `log_detail_modal.rs`
- Modify: `src/app/root.rs`

**Interfaces:**
- Produces:
  - `LogViewerView::new(store, window, cx)`, per `LogViewer.tsx`:
    - Owns: the Container and Time `DropdownState`s (widths 160/176), the filter `InputState`, `mode: Mode { Json, Raw }`, a `VirtualListScrollHandle`, a `selected: Option<LogEntry>`, and a `FilterCache`.
    - Header with Back (ghost sm, ArrowLeft) → `close_log_viewer`.
    - JSON column headers Timestamp 176 / Level 64 / Message via `tracked`.
    - Rows via `v_virtual_list`: Raw 24, JSON 36, ERROR-in-JSON 52 (`line_clamp(2)`, `items_start`); tints red-500/0.05 and amber-500/0.05; hover bg3/0.5; mono 14.
    - Auto-scroll when within 100 px of the bottom; on new data, scroll to the last row with `ScrollStrategy::Bottom`.
    - Footer: Live/Syncing, `"{thousands(n)} lines"`, `"Auto-scroll: ON|OFF"`, Export (secondary sm, Download).
    - `log_skeleton(15)` while `pod_logs.is_loading()`.
  - `pub struct FilterCache` with `rows(&mut self, logs: &[LogEntry], filter: &str) -> &[usize]`: case-insensitive match on `message` or `raw`. It recomputes only when `(filter, logs.len(), logs-ptr)` changed.
  - `pub fn row_height(mode: Mode, level: Option<&str>) -> f32` (24/36/52).
  - `pub fn export_txt(rows: &[&LogEntry]) -> String` (raw joined by `\n`) and file name `"{pod}-{container}-logs.txt"`. The save goes through `cx.prompt_for_new_path(home, Some(name))` + `std::fs::write`; a cancelled dialog writes nothing.
  - `LogDetailModal` (entity, holds `copied: Option<u8>` with a 2 s reset), per `LogDetailModal.tsx`:
    - Size lg (700).
    - Metadata in 2×2 rows.
    - Message (bg3, colored by level, max-h 200), Raw (bg2 + border, xs, max-h 150), JSON formatted (`serde_json::to_string_pretty` of the substring from the first `{`; only when `is_json`).
    - Copy → `cx.write_to_clipboard`, "Copied" with a green Check.
    - All texts selectable.
- [ ] **Step 1:** Write failing tests:
  - `filter_cache_recomputes_only_on_input_change`: count recomputes with an instrumented counter; 3 calls with the same inputs → 1 recompute.
  - `row_heights_ignore_content_length`: a 10k-char INFO message → 36; a multi-line WARN → 36; ERROR → 52; Raw → 24.
  - `export_txt_joins_raw_lines`
  - UI test `clicking_row_opens_detail_modal_and_escape_closes`
  - UI test `json_raw_toggle_switches_header`
- [ ] **Step 2:** Run `cargo test -- app::log_viewer app::log_detail_modal`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run `cargo test`. Expected: all pass.
- [ ] **Step 5 (manual, macOS):**
  1. Side-by-side on a chatty pod over 48 h: scroll speed, auto-scroll on/off at the bottom, filter typing latency (no visible lag at ≥100k lines), modal, copy, export to file.
  2. Container switch on a multi-container pod.
- [ ] **Step 6:** Commit: `feat: log viewer with virtualized rows, detail modal and export`.

### Task 13: Log search

**Files:**
- Create: `src/app/search_state.rs`, `src/app/log_search.rs`
- Modify: `src/app/root.rs`

**Interfaces:**
- Produces from `search_state.rs`, the pure port of `useLogSearch` in `c70bd05:src/hooks/useLogs.ts`:
  - `#[derive(Clone, PartialEq, Debug)] pub struct Snapshot { context, namespace, deployment, keyword: String, log_level: String, time_range: TimeRange }` and `pub struct Scope { context, namespace, deployment: Option<String> }`.
  - `pub enum SearchStatus { Idle, Pending(Snapshot), Success(Snapshot, LogSearchResponse), Error(Snapshot, String) }`.
  - `pub struct SearchState` with:
    - `start(&mut self, snap: Snapshot, scope: &Scope) -> Option<u64>`: refused when pending, when context/namespace/deployment is empty, or when out of scope.
    - `finish(&mut self, id: u64, r: Result<LogSearchResponse, String>, scope: &Scope)`: ignored if the id is stale or out of scope.
    - `reset_scope(&mut self)`: bumps the id, clears pending, goes Idle.
    - `visible(&self, scope: &Scope) -> &SearchStatus`: Idle if the snapshot is out of scope.
    - `retry_snapshot(&self) -> Option<Snapshot>`
  - `pub fn view_model(status: &SearchStatus, form: &Snapshot) -> SearchVm`, with `SearchVm { filters_changed, applied_line: Option<String>, all_failed, partial, total_matches, unique_pods }`. Rules from `LogSearch.tsx`. `applied_line` format: `Applied filters: Keyword "x"` or `All keywords`, then ` · {level or Any level} · {time label}`.
- Produces from `log_search.rs`: `LogSearchView::new(store, window, cx)`, per `LogSearch.tsx`:
  - Form: 4 equal columns, gap 16.
  - Fields: Deployment (searchable dropdown bound to `store.deployment`), Keyword `text_input`, Level dropdown (Any, ERROR, WARN, INFO, DEBUG), Time dropdown (default H1).
  - Enter in Keyword submits.
  - Search button (primary md, Search icon, gap 8), disabled when `!context || !namespace || deployment.is_none() || pending`.
  - Amber "Filters changed. Click Search to apply."
  - States: partial alert with collapsible failure list + Retry; error/all-failed; no containers; no results (partial wording variant); results.
  - Results table: border, radius 8, fixed layout, header bg2. Timestamp 96 (`format_short_timestamp`); Pod column resizable 100–500 (default 208) via a drag handle 4 px wide with hover accent/0.5 and the tooltip "Drag to resize"; Level 80; Message with `highlight_match` (amber-500/0.3 mark, text1, px 2, radius 2), errors `line_clamp(2)`.
  - Pod name in cyan-500 light / cyan-400 dark, container below in xs muted.
  - Tooltips: the pod name shows `"{pod} / {container}"`, the container name shows itself.
  - Footer pod chips `...{last 6 chars}`, each with a tooltip of the full pod name, + Export CSV.
  - Row click → `LogDetailModal`. The scope changes `reset_scope` and close the modal.
  - `pub fn export_csv(results: &[LogSearchResult]) -> String`: lines `"{timestamp}\t{pod}\t{level or -}\t{message}"` using the raw `timestamp` string. File name `"{snapshot.deployment}-log-search.csv"`.
- [ ] **Step 1:** Write failing tests in `search_state.rs`, porting the cases of `c70bd05:src/components/logs/LogSearch.test.tsx`:
  - `submits_identical_params_and_empty_keyword_each_time`
  - `refuses_duplicate_start_while_pending`
  - `editing_form_sets_filters_changed_but_keeps_applied_snapshot`
  - `error_shown_and_retry_uses_previous_snapshot_without_auto_retry`
  - `ignores_late_error_after_scope_change`
  - `timeout_error_shown_and_late_success_ignored`
  - `partial_failures_with_matches`
  - `partial_without_matches_differs_from_complete_empty`
  - `all_failed_is_error`
  - `zero_targets_state`
  - `export_csv_format`

  Then UI tests in `log_search.rs`: `enter_in_keyword_submits` and `search_disabled_without_deployment`.
- [ ] **Step 2:** Run `cargo test -- app::search_state app::log_search`. Expected: FAIL.
- [ ] **Step 3:** Implement. The search runs through `store`'s `api.search_logs` with the snapshot values; `log_level` is `None` for "", and `since_seconds = time_range.seconds()`.
- [ ] **Step 4:** Run `cargo test`. Expected: all pass.
- [ ] **Step 5 (manual, macOS):** Side-by-side on a real deployment: highlight, resize, partial failure (scale a pod to a crashing image), export.
- [ ] **Step 6:** Commit: `feat: log search with request guard, highlight, resize and CSV export`.

### Task 14: Packaging and CI

**Files:**
- Create: `icons/icon.ico` (from `icons/icon_256x256.png`, sizes 16/32/48/256)
- Modify: `Cargo.toml` (`[package.metadata.packager]`), `.github/workflows/release.yml`

**Interfaces:**
- Packager config, keys checked against `cargo packager --help` / cargo-packager docs:
  - `product-name = "PodLogs"`, `identifier = "dev.podlogs.pass"`.
  - Icons: the existing PNGs, `icon.icns` and `icon.ico`.
  - macOS minimum: `10.15`, or gpui's minimum if higher; note the result in the commit.
  - Formats per OS: macOS `dmg`, Linux `deb` + `appimage`, Windows `nsis`.
  - The OFL texts are included as resources.
- `release.yml`:
  - `check-version`: version from `cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version'`; tag logic unchanged.
  - New `test` job (ubuntu-22.04): `rustup show`, apt `libxkbcommon-x11-dev libwayland-dev libx11-xcb-dev libfontconfig-dev libvulkan1` (add what the linker reports), `cargo test --locked`. All build jobs `needs: [check-version, test]`.
  - `build-macos` (aarch64 + x86_64), `build-linux` (ubuntu-22.04 + ubuntu-22.04-arm): `cargo install cargo-packager --locked`, `cargo packager --release`. Keep the rename to `PodLogs_${VERSION}_<os>_<arch>.<ext>` and the upload steps.
  - `build-windows` (windows-latest, `continue-on-error: true`): NSIS → `PodLogs_${VERSION}_windows_x64-setup.exe`.
  - `release`: `needs: [build-macos, build-linux]` plus `if: always() && needs.build-macos.result == 'success' && needs.build-linux.result == 'success'`. Attach the Windows artifact when present. Release notes say Linux/Windows are untested and Windows is unsigned (SmartScreen warning).
- [ ] **Step 1:** Run `cargo packager --release` locally on macOS. Expected: a `.dmg` exists under `target/`. The mounted app launches, has the right icon and name, and About shows `2.0.0`.
- [ ] **Step 2:** Push the branch and run the workflow via `workflow_dispatch`. Expected: the test, macOS and Linux jobs are green; Windows is green or failing without blocking. **Do not merge to `main`** (merging releases).
- [ ] **Step 3:** Commit: `build: cargo-packager config and gpui release workflow`.

### Task 15: Docs

**Files:**
- Modify: `CLAUDE.md`, `README.md`

- [ ] **Step 1:** Rewrite `CLAUDE.md`:
  - Commands: `cargo run`, `cargo test`, `cargo packager --release`.
  - Architecture: Store/Query/Api, views, widgets, tracked text.
  - Pinning rules: toolchain 1.99.1, gpui-kit =0.9.1, upgrades as separate tasks.
  - Keep the "Pod Status Detection" and "Navigation Flow" sections; drop the React/Tauri sections.
- [ ] **Step 2:** Update `README.md`:
  - Install/build instructions; platforms (Windows new, Linux/Windows untested).
  - Remove the Cmd/Ctrl+K claim and every npm instruction.
- [ ] **Step 3:** Run `cargo test` and confirm `grep -rn "npm\|tauri" CLAUDE.md README.md` prints nothing meaningful.
- [ ] **Step 4:** Commit: `docs: update for full-Rust gpui app`.

### Task 16: Release gates (owner + builder, macOS)

**Files:** none (findings go into fixes in the owning task's files)

- [ ] **Step 1: Re-sync.**
  1. Run `git log --oneline c70bd05..origin/main -- src src-tauri` and `git diff c70bd05 origin/main --stat`.
  2. Port every UI or behavior change into the matching task's files, add it to spec §6, and commit `feat: port <change> from main`.
  3. Expected: nothing left un-ported.
- [ ] **Step 2: Full parity pass** with the owner, light and dark, side by side with Tauri v1.0.2: every §6 row. Log each difference, fix it, and re-check.
- [ ] **Step 3: RAM gate**, per spec §8 — total physical footprint, Tauri (app + WebKit processes) vs gpui (`footprint <pid>` / `vmmap --summary`):
  - (a) Pods view idle for 2 min.
  - (b) Log Viewer for 2 min on a pod with ≥3,000 lines.

  Expected: gpui ≤ Tauri in both. If not, profile (Instruments) and fix before release.
- [ ] **Step 4:** Commit any fixes. Hand the branch to the owner for merge, which triggers the release.
