# TuiHoverable content-sized hit area — Tech Spec

Branch: `harry/fix-hoverable-hit-box`.

## Context

TUI event dispatch is area-passing: a parent hands each child a slot rect via `TuiElement::dispatch_event(event, area, ...)`, and the child hit-tests against that rect. Under `CrossAxisAlignment::Start` (the default) and `Stretch`, `TuiFlex` gives every child its full cross-axis extent (`Center`/`End` instead place the child's measured extent within the slot — see `child_rect_for`, crates/warpui_core/src/elements/tui/flex.rs (213-243)). So a collapsible thinking header ("Thought for 4s ▾") in a default-aligned flex column received a full-width row — hover and click activated over the blank space right of the chevron, not just the text.

The GUI framework doesn't have this bug because its `Hoverable` hit-tests against its child's own laid-out size (`Hoverable::is_mouse_over_element`, crates/warpui_core/src/elements/gui/hoverable.rs (405-431)). Area-passing stays; the fix ports only the "hit target = content footprint" idea into `TuiHoverable`.

## Alternative considered: GUI-style cached geometry dispatch

The GUI's `Element::dispatch_event` is geometry-free: events are broadcast through the element tree, each element uses paint-time cached geometry to decide whether it is under the pointer, and `Scene` answers clip/occlusion questions (`visible_rect`, `is_covered`). That model solves GUI-specific overlap: an element can be offered the event even when another element painted above it, so it needs a retained scene to determine whether it is covered.

A direct TUI port would require more than removing the `area` parameter from `TuiElement::dispatch_event`:

- `render` would need to become mutable or gain a separate geometry-recording pass so mouse-aware elements can cache screen-space origins and hit rects.
- A new paint context would need to carry a transform and clip stack. `TuiClipped` renders children into a scratch buffer rooted at `(0, 0)` and then copies a viewport window into the real buffer, so cached child origins are only correct if the paint context records screen-space coordinates separately from buffer coordinates. Rows clipped above the viewport require signed intermediate coordinates before clip intersection; saturating arithmetic would produce incorrect hit areas.
- Full GUI parity would also require a TUI scene or hit map for z-order and occlusion. That is useful only once TUI supports arbitrary overlap; a command palette, modal, tooltip, or autocomplete menu can be handled compositionally by the overlay-owning parent dispatching in reverse paint order and preventing covered siblings from seeing mouse events.
- Dispatch would become tied to the last painted geometry. That mirrors the GUI, but it introduces the same missing/stale-geometry failure mode currently avoided by passing the root area and child slots during dispatch.

The port is feasible but broad: `TuiElement`, the presenter/runtime dispatch path, every TUI element implementation, and most TUI element tests would change together, with the risky work concentrated in `TuiClipped`, `TuiViewportedList`, and `TuiChildView`. The current fix keeps the cell-grid model explicit: parents own placement, clipping, and overlay routing; leaves own only the hit target within the rect they are handed. If TUI grows pervasive arbitrary overlap, a retained hit scene should be reconsidered from those requirements rather than copied preemptively from the GUI.

## Changes

In crates/warpui_core/src/elements/tui/hoverable.rs:

- `laid_out: Option<TuiSize>` (line 49) — the child's size recorded during `layout` (lines 101-110).
- `hit_area(area)` (lines 79-97) — clips the parent-passed slot to the laid-out footprint, anchored at the slot's origin; falls back to the whole area before first layout:

```rust
fn hit_area(&self, area: TuiRect) -> TuiRect {
    match self.laid_out {
        Some(size) => TuiRect::new(
            area.x,
            area.y,
            area.width.min(size.width),
            area.height.min(size.height),
        ),
        None => area,
    }
}
```

- `dispatch_event` (lines 124-178) — hover transitions and click containment both use `hit_area`; the child still receives the full `area`.

This is parent-agnostic: any element wrapped in a `TuiHoverable` gets a content-sized hit target with no call-site changes (`tui_collapsible` needed none). Elements that fill their slot (input box, `Stretch`ed banners) have `laid_out == slot`, so the clip is a no-op and full-slot targets are preserved.

Alongside the leaf change, the two slot-computing parents deduplicate their placement geometry so `render`, `cursor_position`, and `dispatch_event` share one calculation and paint and hit-test geometry cannot drift:

- `TuiFlex::child_slots`, crates/warpui_core/src/elements/tui/flex.rs (150-168) — yields each child's main-axis slot from the last layout pass.
- `VisibleElement::slot`, crates/warpui_core/src/elements/tui/viewported_list.rs (110-120) — yields each visible item's clipped slot within the viewport area.

### Contract: parents own placement offsets

`hit_area` keeps `area`'s top-left corner: a parent that places a child at an offset within its slot must pass the placed rect down as `area` — the offset is not recoverable in the leaf. `TuiFlex` already satisfies this for `CrossAxisAlignment::Center`/`End`: `child_rect_for` (crates/warpui_core/src/elements/tui/flex.rs (213-243)) feeds the same placed rect to `render`, `cursor_position`, and `dispatch_event`, which also makes the clip a harmless no-op under those alignments. Any future parent that offsets children within their slots must follow the same rule.

## Testing and validation

- `hit_testing_is_bounded_to_the_child_laid_out_size`, crates/warpui_core/src/elements/tui/hoverable_tests.rs:190 — hover and click register inside the text but not in the slot's trailing blank space.
- `only_a_header_click_invokes_on_toggle`, crates/warpui_core/src/elements/tui/collapsible_tests.rs:14 — extended so clicking right of the label + chevron does not toggle.
- Run with `cargo nextest run -p warpui_core --features tui -E 'test(tui::hoverable) or test(tui::collapsible) or test(tui::flex) or test(tui::viewported_list)'` (the TUI module is gated behind the `tui` feature).

## Follow-ups

- Add reverse-paint-order overlay tests when TUI gains a stack, command palette, tooltip, or autocomplete surface, so area-passing covers occlusion intentionally instead of relying on current non-overlap.
