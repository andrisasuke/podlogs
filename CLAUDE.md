# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PodLogs is a Kubernetes pod log viewer desktop application built with Tauri 2.x. It uses a Rust backend for K8s API communication and a React/TypeScript frontend.

## Development Commands

```bash
# Install dependencies
npm install

# Start development (Vite + Tauri hot reload)
npm run tauri dev

# Build frontend only (faster iteration for UI work)
npm run build

# Build production app
npm run tauri build

# Check Rust compilation
cd src-tauri && cargo check

# Clean rebuild (needed after icon changes)
cd src-tauri && cargo clean && cd .. && npm run tauri build
```

## Architecture

### Tech Stack
- **Framework**: Tauri 2.x (Rust backend + system webview)
- **Backend**: Rust with kube-rs for K8s API
- **Frontend**: React 19.2 + TypeScript 5.7 + Vite 6 + Tailwind CSS 3.4
- **State**: Zustand 5 (UI state with persistence) + TanStack Query 5 (server state)

### Key Directories
```
src/                    # React frontend
├── components/         # UI components
│   ├── common/         # Button, Modal, Badge, Dropdown, Input, Drawer
│   ├── layout/         # Sidebar, StatusBar, MainLayout
│   ├── pods/           # PodList, PodDetails
│   ├── deployments/    # DeploymentList
│   └── logs/           # LogViewer, LogSearch, LogDetailModal
├── hooks/              # useK8s.ts, useLogs.ts, useTheme.ts
├── stores/             # clusterStore, uiStore, errorStore
├── lib/                # tauri.ts (Tauri bridge), formatters.ts, constants.ts
└── types/              # kubernetes.ts, logs.ts

src-tauri/              # Rust backend
├── src/
│   ├── k8s/            # K8s operations (client.rs, clusters.rs, pods.rs, logs.rs)
│   ├── models/         # Rust structs matching frontend types
│   └── error.rs        # Error types with Tauri serialization
└── icons/              # App icons (icon.icns for macOS)

design/                 # Screenshots and design assets
```

### Data Flow
1. React component calls hook (e.g., `usePods()`)
2. Hook uses React Query to call `src/lib/tauri.ts` wrapper
3. Tauri bridge invokes Rust command via IPC
4. Rust uses kube-rs to call K8s API
5. Response flows back through each layer

### Tauri Commands (Rust → Frontend)
Commands are defined in `src-tauri/src/k8s/*.rs` and registered in `src-tauri/src/lib.rs`:
- `get_clusters` - List K8s contexts from kubeconfig
- `get_namespaces(context)` - List namespaces
- `get_deployments(context, namespace)` - List deployments
- `get_pods(context, namespace, deployment?)` - List pods
- `get_pod_details(context, namespace, pod_name)` - Pod details with containers/env vars
- `get_pod_logs(context, namespace, pod_name, container?, since_seconds?)` - Pod logs
- `search_deployment_logs(context, namespace, deployment, keyword?, log_level?)` - Search logs

### State Management

**clusterStore** (`src/stores/clusterStore.ts`):
- `context` - Current K8s context/cluster
- `namespace` - Current namespace
- `deployment` - Selected deployment (null = show all)
- `lastNamespaceByCluster` - Remembers last namespace per cluster (persisted)

**uiStore** (`src/stores/uiStore.ts`):
- `currentView` - 'deployments' | 'pods' | 'logs' | 'search'
- `theme` - 'light' | 'dark' | 'system'
- `logViewerPod` - Pod currently viewing logs for
- Persists: theme, sidebarCollapsed

**errorStore** (`src/stores/errorStore.ts`):
- `error` - Current connection error
- `isTransitioning` - Flag to ignore errors during namespace/context switches

### Error Handling
- Global error store captures K8s connection errors
- `useK8s` hooks report errors only on initial load (not background refetches)
- `ConnectionError` component shows user-friendly error dialog
- 30-second timeout on K8s API calls (frontend + backend)
- Transition flag prevents spurious errors during namespace/context switches

### Theme
- Light theme: Solarized Light colors (`#FDF6E3` background)
- Dark theme: Slate colors (`#0F172A` background)
- CSS variables defined in `src/index.css`

## Important Patterns

### Navigation Flow
1. User starts at Deployments view (default)
2. Click deployment → Pods view (shows pods filtered by deployment)
3. Click pod's Log button → Log viewer
4. Click Back → Returns to Pods view (same deployment)
5. Namespace/context change → Returns to Deployments view (deployment cleared)

### Pod Status Detection
Rust backend (`src-tauri/src/k8s/pods.rs`) detects statuses in order:
1. Check `deletion_timestamp` → "Terminating"
2. Check container statuses (waiting/terminated reasons)
3. Check pod status reason (Evicted, UnexpectedAdmissionError)
4. Fallback to pod phase

Supported statuses: Running, Pending, Succeeded, Failed, Unknown, CrashLoopBackOff, ImagePullBackOff, ErrImagePull, ContainerCreating, Terminating, Evicted, UnexpectedAdmissionError, ContainerStatusUnknown

### Searchable Dropdown
Use `searchable` prop on Dropdown component:
```tsx
<Dropdown
  options={options}
  value={value}
  onChange={onChange}
  searchable
  searchPlaceholder="Search..."
/>
```
- Auto-disables autocapitalize, autocorrect, spellcheck
- Filters options as user types

### Adding a new K8s query hook
1. Add Tauri command in `src-tauri/src/k8s/` and register in `lib.rs`
2. Add TypeScript wrapper in `src/lib/tauri.ts`
3. Create hook in `src/hooks/useK8s.ts` using `useQuery` with error handling

### Modal behavior
- Regular modals: dismissable via ESC, backdrop click, X button
- Error dialog: `persistent` prop - only dismissable via buttons (not ESC/backdrop)

### Icons (macOS)
- Source: `design/icon_256x256.png`
- Icons must fill ~82% of canvas for proper dock appearance
- Regenerate with: create iconset → `iconutil -c icns`
