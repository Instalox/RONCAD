# Spec: The First Five

Concrete UX spec for the five tools that must ship world-class before any new tool is built: **select · line · trim · dimension · extrude**. Each measured against the seven-point done-check from `tool-manifesto.md`.

---

## 1. Select

**What.** The default tool. Rarely left.

**Hover.** Geometry under cursor gets the preselection weight (lighter than selection). Overlapping stacks cycle with `Tab` — never with a click-menu. A small cursor-local label names the topmost item.

**Click.** Selects. Adds with `Shift`. Subtracts with `Shift+Alt`. *Alt alone is not toggle — too easy to hit accidentally.*

**Drag (empty).** Box-select. Left-to-right = fully contained; right-to-left = crossing. *Direction-based, not modifier-based.*

**Drag (on geometry).** Moves it if unconstrained. Constrained geometry shakes subtly and names the constraint pinning it. *No silent failure.*

**Selection-is-a-verb.** When the selection set changes, a compact chip appears cursor-adjacent: `Extrude · Dimension · Parallel`. Clicking activates that tool with the current selection. `Esc` dismisses.

**Re-edit.** Clicking a committed feature reopens its HUD in place.

**Done-check.** Preview ✓ · HUD ✓ · Typing — N/A · Escape ✓ · Re-edit ✓ · References ✓.

---

## 2. Line

**What.** Draw a line segment. Chainable.

**Activation.** Cursor gains a crosshair; a rubber-band ghost follows from the nearest snap. *First motion is a preview, not a dead state.*

**First click.** Anchors start. Ghost stretches to cursor.

**HUD (at anchor, follows cursor).**

- `Length: 42.500` (mouse-driven)
- `Angle: 28.3°` (mouse-driven)

Typing any digit begins editing Length; `Tab` jumps to Angle. With Length locked, typing modifies Angle. *No "click into field" step.*

**Snaps.** Endpoint, midpoint, intersection, perpendicular-foot, tangent, grid, axis. Active snap is labeled cursor-adjacent.

**Auto-constraints.** Horizontal / vertical fire silently within ~2° of axis-align. A soft glyph near the segment marks the constraint — clickable to remove if unwanted.

**Second click.** Commits. Chain mode: next segment auto-starts from that endpoint. `Enter` or `Esc` breaks the chain.

**Escape ladder.** (1) Clear typed value. (2) Break chain. (3) Exit to Select.

**Re-edit.** Click the line → HUD reopens with editable length/angle.

**Done-check.** All seven ✓.

---

## 3. Trim

**What.** Remove a segment bounded by intersections.

**Hover.** *This is where trim usually fails.* Under cursor, the exact removable chunk — between the two nearest intersections — turns amber and dashed. Not the whole line. Only the chunk.

**Click.** Commits. Remaining endpoints inherit the old line's constraints where geometrically defined.

**Drag.** Paint-trim: sweep the cursor across chunks, each previewing amber, all committed on release. *Stolen from Fusion; kills the fourteen-click trim session.*

**HUD.** Tiny cursor label: `Trim: segment · 3 intersections`. Silent over empty space.

**Edge case — no bounding intersection.** Whole segment turns amber with a hint: `Will delete entire segment (no bounds)`. Still committable; no surprise.

**Escape ladder.** (1) Cancel paint in progress. (2) Exit to Select.

**Re-edit.** N/A by design — undo owns this. *Don't invent a trim-history.*

**Done-check.** Preview ✓ · HUD ✓ · Typing — N/A · Escape ✓ · Re-edit — N/A by design · References ✓.

---

## 4. Dimension

**What.** One tool for every dimension kind. Intent inferred from selection.

**Inference table.**

| Selection | Dimension |
|---|---|
| 1 segment | length |
| 2 parallel lines / 2 points | distance |
| 2 non-parallel lines | angle |
| line + point | point-to-line |
| 1 circle / arc | radius (toggle to diameter) |
| 2 circles | center-to-center |

*The tool never asks "what kind." It guesses from the selection and shows a type pill the user can flip if wrong.*

**Gesture.** With nothing selected: prompt `Pick geometry`. With valid selection: ghost dimension is already drawn; cursor places the text offset. Click commits the offset; typing at any point overrides the value and makes the dimension driving.

**HUD.** Value field (mouse-driven until typed) · type pill · driving/driven toggle.

**Driving vs driven.** Default driving. If adding would over-define, pill auto-flips to driven with a one-line note: `Over-defined — set to reference`. *Never block; always explain.*

**Edit in place.** Double-click a committed dimension → value field opens in the canvas. No modal.

**Escape ladder.** (1) Clear typed value. (2) Clear selection, stay in tool. (3) Exit.

**Done-check.** All seven ✓.

---

## 5. Extrude

**What.** The bridge to 3D. Sets the bar for every feature tool after it.

**Entry.** Closed profile selected → translucent ghost body appears immediately at a sensible default depth. Axis-arrow handle emerges from the profile normal.

**Handle.** Drag the arrow → depth changes live. HUD shows the number.

**HUD (cursor-adjacent).**

- Distance field (draggable or typed)
- Operation chip: **New · Add · Cut** — click cycles, `1/2/3` hotkeys
- Direction chip: **One · Symmetric · Two-sided**
- End condition (disclosed): **Blind · To-next · Through-all · To-face**

*Chips for the common cases; the long tail lives behind one disclosure.*

**Preview.** Ghost body updates every frame. Profile perimeter and new side faces drawn with preview weight. Collisions with existing bodies flash amber for `New`, green for `Add`/`Cut` — before commit, not after.

**Commit.** `Enter` or checkmark in HUD. Tool exits to Select — extrude is one-shot, not chainable.

**Re-edit.** Click the feature in canvas or timeline → same HUD reopens; handle and ghost restored. *This is the unlock. Features feel like live objects, not frozen history.*

**Escape ladder.** (1) Clear typed distance. (2) Cancel, return to sketch with profile still selected.

**Done-check.** All seven ✓.

---

## Out of scope

Construction planes, offset faces, shell, fillet/chamfer, patterns, measure, section. They come after the five pass real use.
