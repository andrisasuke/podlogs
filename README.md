# PodLogs

A modern, lightweight Kubernetes pod log viewer desktop application built with Tauri.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)
![Tauri](https://img.shields.io/badge/Tauri-2.x-blue)
![React](https://img.shields.io/badge/React-19.2-61DAFB)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![Vibe Coded](https://img.shields.io/badge/vibe%20coded-100%25-blueviolet)

> **Note:** This entire application was created through 100% vibe coding with [Claude Code](https://claude.ai/code) - from architecture design to implementation, debugging, and documentation.

## Overview

PodLogs provides a fast, native desktop experience for viewing and searching Kubernetes pod logs. Built with Rust and React, it offers real-time log streaming, multi-pod search, and a clean interface optimized for debugging containerized applications.

## Features

### Core Features
- **Multi-Cluster Support** - Switch between Kubernetes contexts from your kubeconfig
- **Real-Time Log Streaming** - Live log updates with auto-scroll and countdown timer
- **Deployment Log Search** - Search logs across all pods in a deployment simultaneously
- **Log Level Filtering** - Filter by ERROR, WARN, INFO, DEBUG levels
- **Time Range Selection** - View logs from last 5 minutes to 24 hours
- **Pod Details** - View container info, environment variables, labels, and conditions
- **JSON/Raw View Modes** - Toggle between parsed JSON and raw log output
- **Export Logs** - Download logs as text files
- **Copy to Clipboard** - Copy raw logs or formatted JSON from log detail modal

### User Experience
- **Smart Namespace Memory** - Automatically remembers last visited namespace per cluster
- **Searchable Dropdowns** - Search through namespaces and deployments with keyboard
- **Resizable Columns** - Drag to resize pod column in log search results
- **Sticky Table Headers** - Headers stay visible when scrolling through results
- **Dark/Light Theme** - Solarized Light and Slate Dark themes
- **Virtualized Lists** - Smooth scrolling with thousands of log entries
- **Intuitive Navigation** - Deployments → Pods → Logs with proper back navigation

### Pod Status Detection
Comprehensive pod status recognition with color-coded indicators:
- **Running** (green) - Pod is healthy and all containers ready
- **Running** (amber) - Pod running but not all containers ready (e.g., 0/1)
- **Pending / ContainerCreating** (amber) - Pod is starting up
- **Terminating** (purple) - Pod is being deleted
- **Failed / CrashLoopBackOff / ImagePullBackOff / ErrImagePull** (red) - Pod has errors
- **Evicted / UnexpectedAdmissionError / ContainerStatusUnknown** (red) - Pod issues

## Screenshots

<p align="center">
  <img src="design/img-1.png" alt="Screenshot 1" width="100%">
</p>

<p align="center">
  <img src="design/img-2.png" alt="Screenshot 2" width="100%">
</p>

<p align="center">
  <img src="design/img-3.png" alt="Screenshot 3" width="100%">
</p>

## Installation

### Download

Download the latest release for your platform from the [Releases](../../releases) page.

| Platform | Architecture | Download |
|----------|--------------|----------|
| macOS | Apple Silicon (M1/M2/M3/M4) | `PodLogs_x.x.x_macos_aarch64.dmg` |
| macOS | Intel (x64) | `PodLogs_x.x.x_macos_x64.dmg` |
| Linux | x64 (amd64) | `PodLogs_x.x.x_linux_amd64.deb` |
| Linux | ARM64 | `PodLogs_x.x.x_linux_arm64.deb` |

AppImage builds are also available for Linux.

### macOS Installation

1. Download the `.dmg` file for your architecture
2. Open the DMG and drag PodLogs to Applications
3. On first launch, right-click and select "Open" to bypass Gatekeeper

### Linux Installation (Debian/Ubuntu)

```bash
sudo dpkg -i PodLogs_x.x.x_linux_amd64.deb
```

### Prerequisites

- A valid `~/.kube/config` file with cluster access
- Network connectivity to your Kubernetes clusters

## Development

### Requirements

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install) 1.70+
- [Tauri CLI](https://tauri.app/v1/guides/getting-started/prerequisites)

### Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/podlogs.git
cd podlogs

# Install dependencies
npm install

# Start development server
npm run tauri dev
```

### Build

```bash
# Build for current platform
npm run tauri build

# Build for specific targets
npm run tauri build -- --target aarch64-apple-darwin    # macOS Apple Silicon
npm run tauri build -- --target x86_64-apple-darwin     # macOS Intel
npm run tauri build -- --target universal-apple-darwin  # macOS Universal
```

### Project Structure

```
podlogs/
├── src/                      # React frontend
│   ├── components/           # UI components
│   │   ├── common/           # Shared components (Button, Modal, Badge, Dropdown)
│   │   ├── layout/           # Layout components (Sidebar, StatusBar, MainLayout)
│   │   ├── pods/             # Pod list and details
│   │   ├── deployments/      # Deployment list
│   │   └── logs/             # Log viewer, search, and detail modal
│   ├── hooks/                # React hooks (useK8s, useLogs, useTheme)
│   ├── stores/               # Zustand stores (cluster, ui, error)
│   ├── lib/                  # Utilities and Tauri bridge
│   └── types/                # TypeScript definitions
├── src-tauri/                # Rust backend
│   ├── src/
│   │   ├── k8s/              # Kubernetes operations
│   │   ├── models/           # Data structures
│   │   └── error.rs          # Error handling
│   └── icons/                # Application icons
└── design/                   # Screenshots and design assets
```

## Tech Stack

### Frontend
- **React 19.2** - UI framework with latest features (`useEffectEvent`, `Activity`)
- **TypeScript 5.7** - Type safety
- **Tailwind CSS 3.4** - Styling
- **Zustand 5** - UI state management with persistence
- **TanStack Query 5** - Server state, caching, and background refetching
- **TanStack Virtual 3** - Virtualized log rendering for performance

### Backend
- **Tauri 2.x** - Desktop application framework
- **Rust 1.70+** - Backend runtime
- **kube-rs** - Kubernetes API client
- **tokio** - Async runtime

## Configuration

PodLogs reads your Kubernetes configuration from the standard location:

- **macOS/Linux**: `~/.kube/config`
- **Windows**: `%USERPROFILE%\.kube\config`

All contexts defined in your kubeconfig will be available in the cluster selector.

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Esc` | Close modal/drawer |
| `Cmd/Ctrl + K` | Focus search |

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting a pull request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Tauri](https://tauri.app/) - Desktop application framework
- [kube-rs](https://kube.rs/) - Kubernetes client for Rust
- [Lucide](https://lucide.dev/) - Icon library
