/// TODO kb docs, inlays-vs-inlay hints.
pub mod inlay_hints;

use std::any::TypeId;

use gpui::{App, Context, HighlightStyle, Hsla, Rgba, Task};
use multi_buffer::Anchor;
use text::Rope;

use crate::{Editor, hover_links::InlayHighlight};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InlayId {
    EditPrediction(usize),
    DebuggerValue(usize),
    // LSP
    Hint(usize),
    Color(usize),
}

impl InlayId {
    pub fn id(&self) -> usize {
        match self {
            Self::EditPrediction(id) => *id,
            Self::DebuggerValue(id) => *id,
            Self::Hint(id) => *id,
            Self::Color(id) => *id,
        }
    }
}

/// A splice to send into the `inlay_map` for updating the visible inlays on the screen.
/// "Visible" inlays may not be displayed in the buffer right away, but those are ready to be displayed on further buffer scroll, pane item activations, etc. right away without additional LSP queries or settings changes.
/// The data in the cache is never used directly for displaying inlays on the screen, to avoid races with updates from LSP queries and sync overhead.
/// Splice is picked to help avoid extra hint flickering and "jumps" on the screen.
#[derive(Debug, Default)]
pub struct InlaySplice {
    pub to_remove: Vec<InlayId>,
    pub to_insert: Vec<Inlay>,
}

#[derive(Debug, Clone)]
pub struct Inlay {
    pub id: InlayId,
    pub position: Anchor,
    pub text: Rope,
    pub color: Option<Hsla>,
}

impl Inlay {
    pub fn hint(id: usize, position: Anchor, hint: &project::InlayHint) -> Self {
        let mut text = hint.text();
        if hint.padding_right && text.reversed_chars_at(text.len()).next() != Some(' ') {
            text.push(" ");
        }
        if hint.padding_left && text.chars_at(0).next() != Some(' ') {
            text.push_front(" ");
        }
        Self {
            id: InlayId::Hint(id),
            position,
            text,
            color: None,
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn mock_hint(id: usize, position: Anchor, text: impl Into<Rope>) -> Self {
        Self {
            id: InlayId::Hint(id),
            position,
            text: text.into(),
            color: None,
        }
    }

    pub fn color(id: usize, position: Anchor, color: Rgba) -> Self {
        Self {
            id: InlayId::Color(id),
            position,
            text: Rope::from("◼"),
            color: Some(Hsla::from(color)),
        }
    }

    pub fn edit_prediction<T: Into<Rope>>(id: usize, position: Anchor, text: T) -> Self {
        Self {
            id: InlayId::EditPrediction(id),
            position,
            text: text.into(),
            color: None,
        }
    }

    pub fn debugger<T: Into<Rope>>(id: usize, position: Anchor, text: T) -> Self {
        Self {
            id: InlayId::DebuggerValue(id),
            position,
            text: text.into(),
            color: None,
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn get_color(&self) -> Option<Hsla> {
        self.color
    }
}

pub struct InlineValueCache {
    pub enabled: bool,
    pub inlays: Vec<InlayId>,
    pub refresh_task: Task<Option<()>>,
}

impl InlineValueCache {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            inlays: Vec::new(),
            refresh_task: Task::ready(None),
        }
    }
}

impl Editor {
    pub fn splice_inlays(
        &self,
        to_remove: &[InlayId],
        to_insert: Vec<Inlay>,
        cx: &mut Context<Self>,
    ) {
        self.display_map.update(cx, |display_map, cx| {
            display_map.splice_inlays(to_remove, to_insert, cx)
        });
        cx.notify();
    }

    pub(crate) fn highlight_inlays<T: 'static>(
        &mut self,
        highlights: Vec<InlayHighlight>,
        style: HighlightStyle,
        cx: &mut Context<Self>,
    ) {
        self.display_map.update(cx, |map, _| {
            map.highlight_inlays(TypeId::of::<T>(), highlights, style)
        });
        cx.notify();
    }

    pub fn inline_values_enabled(&self) -> bool {
        self.inline_value_cache.enabled
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn inline_value_inlays(&self, cx: &App) -> Vec<Inlay> {
        self.display_map
            .read(cx)
            .current_inlays()
            .filter(|inlay| matches!(inlay.id, InlayId::DebuggerValue(_)))
            .cloned()
            .collect()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn all_inlays(&self, cx: &App) -> Vec<Inlay> {
        self.display_map
            .read(cx)
            .current_inlays()
            .cloned()
            .collect()
    }
}
