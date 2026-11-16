# Packaging and CI evidence

Status: packaging and workflow implementation is locally reviewable; CI and manual installation checks remain owner follow-ups.

## Toolchain and packaging

- Rust toolchain: pinned by `rust-toolchain.toml` (`1.99.1`, minimal profile).
- cargo-packager: `0.11.8`, installed with `cargo install cargo-packager --version 0.11.8 --locked`.
- cargo-packager configuration fields and supported formats were checked against the 0.11.8 source in the Cargo registry and its CLI help.
- Runner labels were checked against GitHub's current hosted-runner documentation: `macos-15` is arm64, `macos-15-intel` is x64, and `ubuntu-22.04-arm` is arm64. `macos-14` is deprecated, so CI uses the current macOS 15 labels.
- A `workflow_dispatch` on the migration branch runs checks/builds but cannot publish a release; publication is gated to `main` and the existing unused-tag check.
- ICO source: `assets/icon_256x256.png`; generated with 16, 32, 48, and 256 pixel entries.
- macOS minimum: 12.0 Monterey. The locked GPUI dependency sources `gpui-pre-apple 0.3.12` and `gpui-fast-apple 0.1.14` each set `-mmacosx-version-min=12.0` in their build scripts. The app bundle minimum and CI `MACOSX_DEPLOYMENT_TARGET` use the same value.
- Packaged font license resources: `assets/fonts/OFL-JetBrainsMono.txt` and `assets/fonts/OFL-IBMPlexSans.txt`.

## Verification

- `cargo packager --version` reports `cargo-packager 0.11.8`; its CLI help and 0.11.8 source confirm the config keys, format names, and package arguments used here.
- `actionlint 1.7.12 .github/workflows/release.yml` passes.
- `cargo test --locked` passes: 124 passed, 1 ignored.
- `cargo build --locked` passes.
- `MACOSX_DEPLOYMENT_TARGET=12.0 cargo build --release --locked` passes on macOS arm64. The packager created `target/release/PodLogs.app` with both OFL license texts. Its plist reports display name `PodLogs`, identifier `dev.podlogs.pass`, version `2.0.0`, and `LSMinimumSystemVersion` `12.0`; the arm64 Mach-O reports `LC_BUILD_VERSION minos 12.0`.
- Metadata leaves formats at cargo-packager's platform default: macOS App + DMG, Windows NSIS, and Linux DEB + AppImage + Pacman. CI selects only the requested formats explicitly (`dmg`, `deb,appimage`, or `nsis`).
- `MACOSX_DEPLOYMENT_TARGET=12.0 cargo packager --release` confirms the platform default selects App + DMG. Its initial Finder AppleScript attempt was denied (`Not authorized to send Apple events to Finder`, error `-1743`); the supported `CI=true` mode adds `create-dmg --skip-jenkins`, which avoids Finder automation and produced the final package. No app was launched and no DMG was opened or manually mounted.
- DMG: `target/release/PodLogs_2.0.0_aarch64.dmg`, 7,473,750 bytes; SHA-256 `0232155001506dbe9f59ce8f8666090bb2f578968cdb10e84847dc5787b10902`. `hdiutil imageinfo` confirms a checksummed, compressed UDIF read-only image.
- Built app binary: `target/release/PodLogs.app/Contents/MacOS/podlogs`, 13,375,056 bytes; SHA-256 `83afb08eba6f41fce09c88d76f8c75a8c8836cf23941a453d59104c29d2c09ac`.
- Pending: CI execution for Linux amd64/arm64, macOS Apple Silicon/Intel, and Windows x64.
- Pending owner checks: manually install and test the macOS DMG. Linux and Windows CI have not run, and neither platform was executed or manually tested on this macOS host. Windows unsigned installer may trigger SmartScreen; macOS builds are unsigned and not notarized, so Gatekeeper may require right-click > Open.
