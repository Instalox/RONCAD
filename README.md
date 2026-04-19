# roncad

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.82%2B-orange.svg)](https://www.rust-lang.org/)
[![workspace status](https://img.shields.io/badge/workspace-7%20crates-lightgrey)](https://doc.rust-lang.org/book/ch14-00-more-about-crates.html)

A Rust workspace project for a precision-first desktop CAD app focused on 3D-printable geometry.

## Overview

This workspace brings together crates for graphics rendering, user interfaces, geometry handling, project I/O, and tooling, built with egui and eframe. The goal is to create a small-scope CAD desktop app for accurate 3D printing geometry, inspired by Autodesk Fusion’s workflow patterns but intentionally much simpler.

## Crates

- **app** – Application entry point and main loop
- **core** – Core data structures and logic (units, IDs, commands, events)
- **geometry** – Geometric primitives and operations (sketches, constraints, profiles, bodies)
- **rendering** – Rendering backends and draw pipelines (wgpu viewport, camera, overlays)
- **ui** – UI components and state (egui panels, toolbars, inspectors)
- **tools** – Interactive editing features (line, rectangle, circle, dimension, extrude tools)
- **project_io** – Serialization and file I/O for project data (save/load, STL export)

## Getting Started

### Prerequisites

- Rust toolchain (version 1.82 or later)
- A C compiler (for wgpu dependencies)
- Vulkan, Metal, D3D12, or OpenGL graphics driver

### Build

```bash
git clone https://github.com/Instalox/RONCAD.git
cd roncad
cargo build --release
```

### Run

```bash
cargo run --release --bin roncad-app
```

## Features

- Real-time rendering with egui and wgpu
- Cross-platform windowing via eframe
- Dark, modern UI
- Constraint-driven 2D sketching
- Simple solid generation (extrude)
- STL export for 3D printing
- Modular, maintainable Rust architecture
- Small source files with strong boundaries (<500 LOC)
- Extensible tool system via command pattern
- Deterministic geometry pipeline

## What's Planned

Based on the [Mini CAD Architecture Blueprint](plan.md):
- Precise 2D sketching and solid creation workflow
- Dimensional constraints for accuracy
- Measurement and inspection tools
- Viewport with grid, camera controls, and gizmos
- Undo/redo through command transactions
- File format: project JSON + binary mesh cache
- Future: 3MF export, fillet/chamfer, revolve

## Documentation

- [Architecture Blueprint](plan.md) – Detailed design and implementation plan
- [Crate Responsibilities](plan.md#crate-responsibilities) – Breakdown of each crate's role
- [Domain Model](plan.md#domain-model) – Core entities and data structures
- [Interaction Architecture](plan.md#interaction-architecture) – Tool and command contracts
- [Viewport Subsystem](plan.md#viewport-subsystem) – Rendering and picking details

## Contributing

We welcome contributions! Please read our contributing guidelines (coming soon) before submitting pull requests.

## License

Licensed under either of:
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

## Acknowledgments

This project is inspired by the workflow patterns of modern CAD systems and built with:
- [egui](https://github.com/emilk/egui) – Immediate mode GUI
- [eframe](https://github.com/emilk/egui/tree/master/eframe) – egui integration for native/web
- [wgpu](https://github.com/gfx-rs/wgpu) – Safe, portable GPU abstraction in Rust
- [glam](https://github.com/bitshifter/glam.rs) – Simple linear algebra
- [slotmap](https://github.com/mountain-pass/slotmap) – Stable key-value storage
- [serde](https://github.com/serde-rs/serde) – Serialization framework

---
*Work in progress – expect frequent changes as we refine the design and implementation.*