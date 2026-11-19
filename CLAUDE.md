# PodLogs

PodLogs is a native Rust desktop Kubernetes pod log viewer built with GPUI. The repository is a single Rust binary crate; the UI and Kubernetes client run in the same process.

## Development

Use the pinned Rust toolchain and locked dependencies:

```sh
cargo run
cargo test --locked
cargo build --locked
```

`cargo run` starts the desktop app and normally needs a working kubeconfig and graphical session. Packaging requires `cargo-packager` 0.11.8 (`cargo install cargo-packager --version 0.11.8 --locked`) and a release binary built first with `cargo build --release --locked`; `cargo packager --release` packages that binary. See README for platform-specific commands and Linux system libraries.

Rust 1.99.1 is pinned in `rust-toolchain.toml`; `gpui-kit` is exactly pinned to `=0.9.1` in `Cargo.toml`. Treat upgrades to either pin as a separate task because they affect the compiler and native UI dependency stack.

## Architecture

- `src/app/store.rs` owns application and selection state, actions, persistence updates, and the periodic tick. It starts due requests and ignores responses from outdated scopes.
- `src/app/query.rs` is the per-resource cache and polling state machine. Query keys identify context, namespace, and resource; a fetch for a key does not overlap itself, and generation changes discard stale completions. The poll interval is measured from fetch start or activation; if a request is still running when polling is due, another fetch waits until it finishes.
- `src/api.rs` defines the `Api` boundary and `KubeApi` implementation. Kubernetes operations run through the Tokio runtime; ordinary calls have a 30-second timeout, while cluster discovery has no timeout.
- `src/k8s/` contains kube-rs operations, with response models in `src/models/`. `src/k8s/pods.rs` fetches referenced ConfigMaps; `src/k8s/pod_env.rs` expands the fetched data into environment declarations.
- `src/app/` contains GPUI screen entities and app behavior. `src/ui/` contains reusable widgets and theme/text primitives. `src/ui/tracked_text.rs` draws tracked text for letter-spaced labels; use the shared `tracked` element where that typography is needed.
- `src/persist.rs` loads and saves UI preferences and cluster/namespace selection under the platform configuration directory at `podlogs/state.json`.

Data flows from a view action to `Store`, which updates selection or query state; the tick asks `Api` to fetch due Kubernetes data; results update the keyed query cache and notify observing views. Views render cached data and shared widgets. Log viewer and search tables use virtual lists so rendering work is limited to visible rows.

When changing a query, preserve its key scope and generation behavior so data or errors from an old context/namespace cannot appear after a selection change. Keep cached data on background refresh errors; initial connection errors are surfaced through the app error dialog. Persistence stores preferences and selected context/namespace, not live Kubernetes resources.

## Navigation Flow

1. Startup opens Deployments. When namespaces load, the saved namespace is used when available; otherwise `default` is preferred, then the first available namespace.
2. Selecting a deployment opens Pods filtered to that deployment. Clearing the deployment shows all pods in the namespace.
3. Opening a pod's Logs view shows that pod's logs; Back returns to Pods. Changing context or namespace clears the deployment. If the current view is Pods or Logs, it returns to Deployments; Log Search stays open.
4. Log Search is a separate sidebar view. Settings and resource details open in drawers or dialogs over the current view.

## Pod Status Detection

`src/k8s/pods.rs` determines pod status in this order: deletion timestamp (`Terminating`), container waiting/termination reasons, pod status reason (including `Evicted` and `UnexpectedAdmissionError`), then pod phase. `src/status.rs` maps status and readiness to display badge variants. Supported displayed conditions include Running, Pending, Succeeded, Failed, Unknown, CrashLoopBackOff, ImagePullBackOff, ErrImagePull, ContainerCreating, Terminating, Evicted, UnexpectedAdmissionError, and ContainerStatusUnknown.

## Pod Environment Values

Pod details expand `envFrom.configMapRef` into current ConfigMap data, applying each declared prefix, and resolve `configMapKeyRef` values while retaining the declared environment variable name as an alias. These are values from the current Kubernetes configuration and can differ from the environment of an already-running container. Secret values remain masked; Secret `envFrom` is shown as a declaration, not expanded secret data. Missing or inaccessible ConfigMaps are represented as unresolved environment entries/issues.
