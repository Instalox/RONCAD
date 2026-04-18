# Mini CAD Architecture Blueprint

## Product goal

Build a **small-scope CAD desktop app for accurate 3D printing geometry**, inspired by Autodesk Fusion’s workflow patterns but intentionally much simpler.

The product should prioritize:

* precise dimensions
* clean sketching and solid creation workflow
* dark, modern UI
* maintainable Rust code
* modular features that integrate without cross-cutting mess
* small source files with strong boundaries

## Core product definition

This is **not** a general-purpose CAD suite.

It should focus on:

1. 2D sketch creation on a work plane
2. dimensional constraints for accuracy
3. simple solid generation from sketches
4. measurement and inspection tools
5. export for 3D printing

It should avoid, at least initially:

* advanced surfacing
* assemblies
* simulation
* generative design
* cloud collaboration
* scripting language support
* history-heavy feature complexity beyond a small parametric timeline

## Recommended technical direction

### App model

Use a **single desktop Rust application** with a layered architecture:

* **UI layer**
* **application/state layer**
* **domain/modeling layer**
* **rendering layer**
* **platform/infrastructure layer**

### Recommended stack

* **Windowing + app shell:** `eframe` / `egui`
* **GPU rendering:** `wgpu`
* **Math:** `glam`
* **Serialization:** `serde`
* **IDs / handles:** `slotmap` or typed arena handles
* **Error handling:** `thiserror`, `anyhow` in app boundaries only
* **Logging/tracing:** `tracing`, `tracing-subscriber`
* **File format:** project JSON + binary mesh cache if needed
* **Geometry kernel (initial):** custom lightweight geometric core

### Why this stack

`egui` is immediate-mode and especially well-suited to highly interactive tools, while `wgpu` gives a safe cross-platform Rust graphics API for native GPU rendering. That combination fits a compact CAD app with custom viewport behavior better than a heavier retained widget architecture. `egui` explicitly positions itself as immediate mode and highly interactive, and `wgpu` is a safe portable Rust graphics layer targeting Vulkan, Metal, D3D12, and OpenGL/WebGPU backends. ([docs.rs](https://docs.rs/egui/latest/egui/?utm_source=chatgpt.com))

## Architectural principles

1. **Domain-first**: geometry logic must not depend on UI.
2. **Typed everything**: avoid raw strings and primitive obsession.
3. **Feature modules own behavior**: each tool is a plugin-like module.
4. **Commands mutate state**: tools produce commands, not direct scattered mutations.
5. **Viewport is a subsystem**: rendering and interaction are separate concerns.
6. **Project model is authoritative**: UI reflects state, never becomes the source of truth.
7. **Small files, strong boundaries**: every source file under 500 LOC.
8. **No hidden coupling**: shared interfaces live in stable core modules.
9. **Deterministic geometry pipeline**: same model state should always rebuild the same output.
10. **Progressive complexity**: start direct-modeling friendly, add limited parametrics later.

## Best UI choice for this product

Use **egui for panels, toolbars, inspectors, dialogs, command palette, and timeline UI**, and use a **custom wgpu viewport** for the CAD scene.

This is the right tradeoff because:

* CAD interaction is viewport-heavy
* custom overlays, snapping markers, selection outlines, and gizmos benefit from direct rendering control
* immediate-mode UI keeps surrounding tools simple and easy to refactor
* the app stays entirely in Rust

## High-level system layout

```text
mini-cad/
  crates/
    app/
    core/
    geometry/
    rendering/
    ui/
    tools/
    project_io/
```

## Crate responsibilities

### `core`

Shared primitives and abstractions.

* IDs
* units
* commands
* events
* app traits
* selection abstractions
* typed errors

### `geometry`

Authoritative model and geometric operations.

* sketch entities
* constraints
* work planes
* extrusions
* booleans later
* measurements
* topology-lite data model

### `rendering`

Everything GPU and viewport related.

* camera
* grid renderer
* sketch renderer
* solid renderer
* picking buffers
* highlight overlays
* gizmos

### `ui`

Panels and desktop shell.

* left tool shelf
* top toolbar
* right inspector
* bottom status bar
* project tree
* dimensions panel
* timeline strip

### `tools`

Interactive editing features.

* line tool
* rectangle tool
* circle tool
* dimension tool
* select tool
* pan/orbit tool
* extrude tool

### `project_io`

Save/load/export.

* project serialization
* STL export
* 3MF export later
* versioned file schema

### `app`

Composition root.

* startup
* dependency wiring
* main app state
* command dispatch
* undo/redo orchestration

## Domain model

The domain should use explicit typed structures.

### Recommended core entities

* `Project`
* `DocumentSettings`
* `Workspace`
* `Workplane`
* `Sketch`
* `SketchEntity`
* `Constraint`
* `Dimension`
* `Body`
* `Feature`
* `Selection`
* `Camera`

### Sketch entities

Start with:

* point
* line segment
* rectangle primitive
* circle
* arc later
* construction line

### Constraint types

Start with:

* coincident
* horizontal
* vertical
* parallel
* perpendicular
* equal length
* distance
* radius/diameter

### Solid feature types

Start with:

* extrude
* cut extrude
* fillet later
* chamfer later
* revolve later

## Recommended modeling strategy

Do **not** begin with a full B-rep kernel.

Start with a staged approach:

1. **Constraint-driven 2D sketch model**
2. **Generate polygonal profiles from closed sketch loops**
3. **Extrude profiles into watertight triangle meshes**
4. **Export STL**
5. Later, optionally replace internals with a stronger geometry kernel if necessary

This keeps scope realistic and lets you ship a useful 3D-printing design tool quickly.

## Parametric strategy

Use a **limited feature timeline**, not a giant dependency graph at first.

Each feature should:

* consume stable references to prior entities
* rebuild deterministically
* expose editable parameters
* provide preview state during editing

Feature examples:

* `CreateSketch`
* `AddLine`
* `AddCircle`
* `AddConstraint`
* `ExtrudeProfile`

A rebuild pipeline should look like:

```text
Project State -> Evaluate Sketches -> Solve Constraints -> Build Profiles -> Build Mesh Bodies -> Render
```

## Interaction architecture

Interactive tools should follow a common contract.

```rust
pub trait Tool {
    fn id(&self) -> ToolId;
    fn on_activate(&mut self, ctx: &mut ToolContext);
    fn on_pointer_event(&mut self, ctx: &mut ToolContext, event: PointerEvent);
    fn on_keyboard_event(&mut self, ctx: &mut ToolContext, event: KeyboardEvent);
    fn draw_overlay(&self, ctx: &mut OverlayContext);
    fn build_commands(&mut self, ctx: &ToolContext) -> Vec<AppCommand>;
    fn on_deactivate(&mut self, ctx: &mut ToolContext);
}
```

This makes each feature a drop-in module instead of scattered special cases.

## Command architecture

Use a command/event split.

### Commands

Intent to mutate state.
Examples:

* `CreateSketch`
* `AddSketchEntity`
* `ApplyConstraint`
* `UpdateDimension`
* `CreateExtrude`
* `DeleteSelection`

### Events

Derived notifications.
Examples:

* `SelectionChanged`
* `DocumentDirtyChanged`
* `FeatureRebuilt`
* `MeshInvalidated`

### Why this matters

It keeps:

* undo/redo centralized
* tool logic thin
* UI decoupled from domain mutation

## Undo/redo model

Use a transactional command stack.

```text
Tool interaction -> build one or more commands -> command transaction -> apply -> push inverse ops -> emit events
```

Rules:

* mouse drag previews should not spam permanent undo entries
* commit only on confirm/release/finalize
* geometry rebuild can happen continuously, but history commit should be intentional

## Viewport subsystem

The viewport should be treated like its own engine.

### Responsibilities

* camera navigation
* grid rendering
* sketch entity rendering
* body mesh rendering
* hover and selection visualization
* snapping visualization
* transform/extrude gizmos
* hit testing / picking

### Camera modes

Start with:

* orbit
* pan
* zoom
* orthographic top/front/right
* perspective toggle optional

For 3D printing geometry, orthographic views should be first-class.

## Picking strategy

Use layered picking:

1. coarse CPU bounds test
2. optional GPU ID buffer for precise entity picking
3. priority resolution based on active tool and mode

Pickable targets:

* sketch vertex
* sketch edge
* profile region
* body face
* body edge later

## Snapping strategy

Snapping is critical for UX quality.

Start with:

* endpoint
* midpoint
* center
* intersection
* grid
* horizontal/vertical inference

The snap engine should be a standalone module with no UI dependencies.

## Units and precision

Use explicit unit types everywhere.

Recommended approach:

* internal canonical unit: millimeters
* user-visible units configurable later
* avoid raw `f32` across the domain
* use `f64` for geometry calculations
* only convert to `f32` at render boundaries when needed

Suggested types:

* `LengthMm(f64)`
* `AngleRad(f64)`
* `Vec2d`
* `Vec3d`

## File and module policy

Every file:

* under 500 lines
* one primary responsibility
* a short header comment describing purpose
* tests close to the module where possible

Example file header style:

```rust
//! Handles sketch line entity creation and editing.
//! Keeps line geometry rules isolated from UI and rendering.
```

## Suggested workspace structure

```text
crates/
  app/
    src/
      main.rs
      app.rs
      bootstrap.rs
      routes.rs

  core/
    src/
      lib.rs
      ids.rs
      units.rs
      command.rs
      event.rs
      selection.rs
      error.rs
      transaction.rs

  geometry/
    src/
      lib.rs
      project.rs
      workspace.rs
      workplane.rs
      sketch.rs
      sketch_entity.rs
      constraint.rs
      dimension.rs
      profile.rs
      body.rs
      feature.rs
      mesh.rs
      solver.rs
      evaluate.rs

  rendering/
    src/
      lib.rs
      renderer.rs
      camera.rs
      grid.rs
      sketch_renderer.rs
      body_renderer.rs
      overlays.rs
      picking.rs
      materials.rs
      viewport.rs

  ui/
    src/
      lib.rs
      theme.rs
      shell.rs
      toolbar.rs
      tool_shelf.rs
      inspector.rs
      project_tree.rs
      status_bar.rs
      timeline.rs
      dialogs.rs

  tools/
    src/
      lib.rs
      tool.rs
      context.rs
      manager.rs
      select_tool.rs
      line_tool.rs
      rectangle_tool.rs
      circle_tool.rs
      dimension_tool.rs
      extrude_tool.rs
      navigation_tool.rs
      snapping.rs

  project_io/
    src/
      lib.rs
      schema.rs
      save.rs
      load.rs
      stl_export.rs
      migration.rs
```

## App state design

Avoid one giant mutable app struct with everything in it.

Prefer:

* persistent document state
* transient tool state
* transient UI state
* cached render state

### Example split

* `DocumentState`: project model, feature tree, selection
* `UiState`: open panels, focused inputs, theme, dialog state
* `ToolRuntimeState`: active tool, hover state, in-progress gesture, snap candidates
* `RenderCache`: tessellated sketch buffers, body meshes, pick buffers

## Theme and UX direction

### Visual direction

* dark-first theme
* low-noise UI chrome
* high-contrast active tool state
* restrained accent color
* minimal beveling
* soft separators
* generous spacing in inspectors
* dense but legible numeric inputs

### UX principles

* left: tools
* center: viewport
* right: properties/inspector
* top: file/edit/view/tool tabs or compact toolbar
* bottom: coordinates, snap feedback, dimensions, hints

### UX behavior

* dimensions should be editable inline
* tool hints should appear in status bar
* hover should never feel ambiguous
* selections should show clear hierarchy
* destructive changes should be obvious
* numeric precision entry should be fast from keyboard

## Initial feature roadmap

### Milestone 1: skeleton app

* window
* dark theme
* docked panels
* viewport with grid
* camera controls
* command system skeleton
* project save/load skeleton

### Milestone 2: 2D sketching

* create sketch on XY plane
* line tool
* rectangle tool
* circle tool
* selection
* delete
* snapping
* dimensions display

### Milestone 3: constraints

* horizontal/vertical
* coincident
* distance
* radius
* simple solver pass
* editable dimensions

### Milestone 4: profile generation + extrude

* detect closed loops
* generate profile regions
* extrude to mesh
* basic shaded viewport
* STL export

### Milestone 5: inspection and polish

* measure tool
* object tree
* undo/redo polish
* better selection outlines
* keyboard shortcuts
* file versioning

## What to avoid early

* a plugin system loaded from dynamic libraries
* ECS for the core CAD document
* deep trait abstraction before patterns stabilize
* advanced CAD kernels before shipping the first usable version
* over-engineered dependency injection
* generalized node graph architecture

## Where an external CAD kernel may fit later

If the product eventually needs stronger topological modeling, you can evaluate Rust CAD kernels such as Truck or Fornjot. Both exist, but both are still better treated as future options rather than foundations for a small first version. Truck explicitly presents itself as a Rust CAD kernel, and Fornjot describes itself as an early-stage experimental b-rep CAD kernel. ([github.com](https://github.com/ricosjp/truck?utm_source=chatgpt.com))

## Recommended first implementation choice

If you want the cleanest path to a working version:

* use `egui` + `eframe` for app shell and tool panels
* use `wgpu` for custom viewport rendering
* keep geometry custom and minimal
* implement 2D sketch + extrusion to mesh before any advanced CAD features
* structure the repo as a Rust workspace with separate crates by responsibility

## Non-negotiable engineering rules

1. No source file over 500 lines.
2. Every file starts with a 1-2 line purpose comment.
3. UI code never owns geometry truth.
4. Geometry code never depends on UI crates.
5. Every feature implements the common tool/command contracts.
6. State mutations happen through commands or transactions only.
7. Cache invalidation must be explicit.
8. Add tests for geometry, constraints, and file migrations.
9. Prefer composition over inheritance-like abstractions.
10. Refuse scope creep until extrusion and STL export are excellent.

## My recommendation

Build this as a **precision-first desktop CAD app for 3D-printable parts**, not a miniature Fusion clone.

That means the identity should be:

* fast sketching
* exact dimensions
* clean extrusion workflow
* reliable export
* elegant dark UI
* highly modular Rust architecture

That focus will make the product feel intentional instead of incomplete.

## Next design step

The next concrete step should be to define:

1. the exact user workflow
2. the app state model
3. the command types
4. the first folder/crate scaffold
5. the MVP screen layout
