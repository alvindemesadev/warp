# Warp - High-Speed File Mover

## 1. Overview
Warp is a lightweight desktop application designed to bypass the slow UI of traditional file explorers by leveraging the raw power of the command line (Robocopy for Windows, Rsync for macOS/Linux).

## 2. Technical Stack (Ultra-Lightweight)
- **Backend**: Rust (via Tauri 2.0)
    - *Why*: Zero-overhead, memory-safe, and compiles to a tiny native binary.
- **Frontend**: Svelte 5 + Vite
    - *Why*: Svelte is a compiler, not a framework. It has no virtual DOM, resulting in the smallest possible JS bundle and fastest execution.
- **CSS**: Tailwind CSS v4
    - *Why*: The new lightning-fast engine with zero-config and minimal CSS output.
- **Communication**: Tauri "Commands" (Direct Rust-to-JS bridge)

## 3. Core Scenarios & Processes
- **Standard Copy**: Multi-threaded copy using `/MT:32`.
- **Move (Cut & Paste)**: Verified move using `/MOVE`.
- **Conflict Management**: Pre-set rules (Skip, Overwrite, Resume).
- **Resumable Transfers**: Native support for restarting interrupted jobs.
- **Admin Elevation**: Auto-prompting for restricted directories.

## 4. UI/UX Design
- **Minimalist Interface**: Two drop zones (Source & Destination).
- **Mode Toggle**: Easy switching between Copy/Move/Sync.
- **Progress Monitoring**: Visual bars with an optional "Live CLI Feed".
- **Theme**: Dark mode with high-contrast neon accents.

## 5. Development Roadmap
- [x] Initialize Tauri + Svelte 5 project
- [x] Implement Rust backend for CLI process spawning
- [x] Build Drag-and-Drop frontend interface
- [x] Integrate Progress parsing from CLI stdout
- [x] Add Error Handling and Summary reports
