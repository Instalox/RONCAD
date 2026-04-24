# RonCad Tool Manifesto

**Thesis.** Tools decide whether RonCad feels premium or merely capable. We don't win by shipping more — we win by making the ones we ship feel inevitable.

**North star.** Fusion precision. Shapr3D directness. Onshape clarity. ZBrush responsiveness. No one of these alone is the answer; the blend is.

## Principles

### Feel
1. **Preview is the product.** The commit is a footnote.
2. **Cursor-local beats panel-heavy.** Sidebar holds lists and diagnostics. The canvas holds the tool.
3. **Type anytime.** Numeric input is ambient, never modal. No clicking into a field first.
4. **Fluid over ceremonious.** A tool that responds in the same frame feels truer than one that validates first.
5. **Guided, not gated.** Tools suggest; they don't trap.

### Interaction
6. **Zero dead clicks.** Activation = preview starts. Never "now click somewhere to begin."
7. **Escape is a ladder, not a cliff.** First press clears substate / typing. Second exits the tool.
8. **Selection is a verb.** Two lines selected → parallel/perpendicular/equal is already on offer. Closed profile → extrude. Centerline + geometry → symmetry.
9. **One good default.** The common case takes zero configuration. Options stay hidden until asked.
10. **Everything re-editable.** If it committed, one click reopens its driving values.

### Visual
11. **Quiet canvas, loud tool.** Dark calm ground; bright, unambiguous active state.
12. **Four weights, strictly ordered: selection > hover > preview > problem.** No neon. Amber/red only when something is actually wrong.
13. **Handles are big.** Legacy CAD hit-targets are a bug, not a tradition.
14. **References are visible.** What the tool is snapping to, cutting from, or driven by — always drawn during the gesture.

## Definition of a "done" tool

A tool ships only when it meets all seven:

- Preview is live on activation
- Mini-HUD shows what matters *right now* and nothing else
- Typing works without focusing a field
- Escape behavior is layered
- Commit → repeat (if natural) or → Select (if one-shot)
- Result is re-editable by clicking it
- Every reference it used is visually cited during the gesture

If you can't check all seven, the tool isn't done — it's a feature flag in disguise.

## Decision filter

When a design choice is contested, ask in order:

1. Does it reduce clicks in the common case?
2. Does it make the preview more honest?
3. Does it let the user type?
4. Does it stay re-editable after commit?

If all four are no, reject the choice. If one is yes, debate. If two or more are yes, ship it.

## What we don't do

- Ribbon toolbars.
- Modal dialogs for anything a HUD could own.
- Mystery constraints. Every solved constraint is drawn; every failed one is named.
- "Click here to begin" empty states.
- Tools that exist only because the architecture made them easier than the right tool.
- More settings to compensate for a weak default.

## The five that must be world-class first

**select · line · trim · dimension · extrude**

Everything else inherits their quality bar. If any of the five is merely good, the app is merely good. No new tool gets built until these five pass the seven-point check.
