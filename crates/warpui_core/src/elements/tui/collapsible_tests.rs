use std::cell::Cell;
use std::rc::Rc;

use super::{tui_collapsible, tui_collapsible_with_header_spans, TuiCollapsibleHeaderSpan};
use crate::elements::tui::test_support::with_paint_context;
use crate::elements::tui::{
    Modifier, TuiBuffer, TuiBufferExt, TuiConstraint, TuiElement, TuiEvent, TuiEventContext,
    TuiLayoutContext, TuiPoint, TuiRect, TuiSize, TuiStyle, TuiText,
};
use crate::elements::MouseStateHandle;
use crate::event::ModifiersState;
use crate::{App, EntityId, EntityIdMap};

#[test]
fn only_a_header_click_invokes_on_toggle() {
    App::test((), |app| async move {
        app.read(|app_ctx| {
            let hits = Rc::new(Cell::new(0u32));
            let counter = hits.clone();
            let mut collapsible = tui_collapsible(
                false,
                "Thinking...",
                TuiStyle::default(),
                TuiStyle::default(),
                MouseStateHandle::default(),
                TuiText::new("reasoning").finish(),
                move |_, _| counter.set(counter.get() + 1),
            );
            let mut rendered_views = EntityIdMap::default();
            let mut ctx = TuiLayoutContext {
                rendered_views: &mut rendered_views,
            };
            let area = TuiRect::new(0, 0, 20, 4);
            collapsible.layout(TuiConstraint::loose(TuiSize::new(20, 4)), &mut ctx, app_ctx);
            // A click is a press-then-release pair; the hoverable's arming
            // notify needs an origin view to attribute the redraw to.
            let mut click = |y| {
                let mut event_ctx = TuiEventContext::default();
                event_ctx.set_origin_view(Some(EntityId::new()));
                let down = TuiEvent::LeftMouseDown {
                    position: TuiPoint::new(2, y),
                    modifiers: ModifiersState::default(),
                    click_count: 1,
                    is_first_mouse: false,
                };
                let pressed =
                    collapsible.dispatch_event(&down, area, &mut event_ctx, &mut ctx, app_ctx);
                let up = TuiEvent::LeftMouseUp {
                    position: TuiPoint::new(2, y),
                    modifiers: ModifiersState::default(),
                };
                let released =
                    collapsible.dispatch_event(&up, area, &mut event_ctx, &mut ctx, app_ctx);
                pressed && released
            };

            // Row 0 is the header: the click toggles. Row 1 is the body: the
            // header's handler covers only its own slot, so it goes unhandled.
            assert!(click(0));
            assert_eq!(hits.get(), 1);
            assert!(!click(1));
            assert_eq!(hits.get(), 1);
        });
    });
}

#[test]
fn hovered_header_swaps_styles_per_span_without_bleeding_past_the_text() {
    App::test((), |app| async move {
        app.read(|app_ctx| {
            let underlined = TuiStyle::default().add_modifier(Modifier::UNDERLINED);
            let render_header = |hovered| {
                let mouse_state = MouseStateHandle::default();
                mouse_state.lock().unwrap().is_hovered = hovered;
                // A glyph span with no hover style beside a label span that
                // underlines on hover; the chevron keeps the resting style.
                let mut collapsible = tui_collapsible_with_header_spans(
                    false,
                    [
                        TuiCollapsibleHeaderSpan::new("☰ ", TuiStyle::default()),
                        TuiCollapsibleHeaderSpan::new("Tasks 3", TuiStyle::default())
                            .with_hover_style(underlined),
                    ],
                    TuiStyle::default(),
                    TuiStyle::default(),
                    mouse_state,
                    TuiText::new("body").finish(),
                    |_, _| {},
                );
                let mut rendered_views = EntityIdMap::default();
                let mut ctx = TuiLayoutContext {
                    rendered_views: &mut rendered_views,
                };
                let area = TuiRect::new(0, 0, 20, 2);
                collapsible.layout(TuiConstraint::loose(TuiSize::new(20, 2)), &mut ctx, app_ctx);
                let mut buffer = TuiBuffer::empty(area);
                with_paint_context(|paint_ctx| collapsible.render(area, &mut buffer, paint_ctx));
                buffer
            };

            // Hovered: `☰ Tasks 3 ▾` underlines exactly the label's cells —
            // not the glyph, the chevron, or the trailing cells past the
            // text. The label's start column is located from the buffer since
            // the glyph's cell width varies by rendering backend.
            let buffer = render_header(true);
            assert_eq!(buffer.to_lines()[0].trim_end(), "☰ Tasks 3 ▾");
            let label_start = (0..20u16)
                .find(|&x| buffer[(x, 0)].symbol() == "T")
                .expect("the header row contains the label");
            let underlined: Vec<u16> = (0..20u16)
                .filter(|&x| buffer[(x, 0)].modifier.contains(Modifier::UNDERLINED))
                .collect();
            // "Tasks 3" spans seven cells.
            let label_cells: Vec<u16> = (label_start..label_start + 7).collect();
            assert_eq!(underlined, label_cells);

            // Not hovered: no underline anywhere on the header row.
            let buffer = render_header(false);
            assert!((0..20u16).all(|x| !buffer[(x, 0)].modifier.contains(Modifier::UNDERLINED)));
        });
    });
}
