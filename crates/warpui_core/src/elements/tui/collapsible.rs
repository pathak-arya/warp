//! [`tui_collapsible`]: a disclosure section — a clickable text header with a
//! chevron over a body that shows only when expanded.
//!
//! This is a plain composition of existing primitives: a [`TuiFlex`] column
//! whose first child is the header (a [`TuiText`] of the label followed by a
//! chevron reflecting the state, wrapped in a [`TuiHoverable`] for the click
//! and hover tracking) and whose second child — present only when expanded —
//! is the body. State is owned by the caller: `collapsed` and the hover state
//! on `mouse_state` are read at composition time and `on_toggle` fires on a
//! header click, leaving the caller to flip its own state and re-render.

use super::{TuiElement, TuiEventContext, TuiFlex, TuiHoverable, TuiStyle, TuiText};
use crate::elements::MouseStateHandle;
use crate::AppContext;

/// Disclosure glyph shown when the section is collapsed.
const CHEVRON_COLLAPSED: &str = "▸";
/// Disclosure glyph shown when the section is expanded.
const CHEVRON_EXPANDED: &str = "▾";

/// One styled run of a collapsible header, with the style swapped in while
/// the header is hovered. Spans let a header mix hover treatments — e.g.
/// underline only the label while a decorative leading glyph (whose own
/// strokes would clash with an underline) keeps its resting style.
pub struct TuiCollapsibleHeaderSpan {
    text: String,
    style: TuiStyle,
    hover_style: TuiStyle,
}

impl TuiCollapsibleHeaderSpan {
    /// A span rendered with `style` whether or not the header is hovered.
    pub fn new(text: impl Into<String>, style: TuiStyle) -> Self {
        Self {
            text: text.into(),
            style,
            hover_style: style,
        }
    }

    /// Renders the span with `hover_style` while the header is hovered.
    pub fn with_hover_style(mut self, hover_style: TuiStyle) -> Self {
        self.hover_style = hover_style;
        self
    }
}

/// Composes a collapsible section from one uniformly-styled header label:
/// the label and chevron are styled with `header_hover_style` while
/// `mouse_state` reports the header hovered and `header_style` otherwise.
/// See [`tui_collapsible_with_header_spans`] for the general form and the
/// remaining parameter semantics.
pub fn tui_collapsible(
    collapsed: bool,
    label: impl Into<String>,
    header_style: TuiStyle,
    header_hover_style: TuiStyle,
    mouse_state: MouseStateHandle,
    body: Box<dyn TuiElement>,
    on_toggle: impl FnMut(&mut TuiEventContext, &AppContext) + 'static,
) -> Box<dyn TuiElement> {
    tui_collapsible_with_header_spans(
        collapsed,
        [TuiCollapsibleHeaderSpan::new(label, header_style).with_hover_style(header_hover_style)],
        header_style,
        header_hover_style,
        mouse_state,
        body,
        on_toggle,
    )
}

/// Composes a collapsible section: a clickable header of styled `spans`
/// (suffixed with a state chevron styled by `chevron_style` /
/// `chevron_hover_style`) over `body`, which is included only when
/// `collapsed` is `false`. `on_toggle` runs when the header is clicked.
/// While `mouse_state` reports the header hovered, each span and the chevron
/// swap to their hover styles; hover transitions are recorded on
/// `mouse_state`, which the caller owns so it survives re-renders.
///
/// Styling is per span, with no base style painted across the header row, so
/// hover decorations (e.g. an underline) never bleed past the header text
/// into the row's trailing cells.
pub fn tui_collapsible_with_header_spans(
    collapsed: bool,
    spans: impl IntoIterator<Item = TuiCollapsibleHeaderSpan>,
    chevron_style: TuiStyle,
    chevron_hover_style: TuiStyle,
    mouse_state: MouseStateHandle,
    body: Box<dyn TuiElement>,
    on_toggle: impl FnMut(&mut TuiEventContext, &AppContext) + 'static,
) -> Box<dyn TuiElement> {
    let chevron = if collapsed {
        CHEVRON_COLLAPSED
    } else {
        CHEVRON_EXPANDED
    };
    let hovered = mouse_state
        .lock()
        .map(|state| state.is_hovered())
        .unwrap_or(false);
    let mut header_spans: Vec<(String, TuiStyle)> = spans
        .into_iter()
        .map(|span| {
            let style = if hovered {
                span.hover_style
            } else {
                span.style
            };
            (span.text, style)
        })
        .collect();
    header_spans.push((
        format!(" {chevron}"),
        if hovered {
            chevron_hover_style
        } else {
            chevron_style
        },
    ));
    let header = TuiHoverable::new(
        mouse_state,
        TuiText::from_spans(header_spans).truncate().finish(),
    )
    .on_click(on_toggle);

    let mut column = TuiFlex::column().child(header.finish());
    if !collapsed {
        column = column.child(body);
    }
    column.finish()
}

#[cfg(test)]
#[path = "collapsible_tests.rs"]
mod tests;
