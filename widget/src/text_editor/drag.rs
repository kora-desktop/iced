//! Dragging selected text.
//!
//! A press on the selection does not place the caret at once: held and moved
//! a few pixels, it carries the selection. Let go over the text, the
//! selection moves there as one edit ([`Edit::MoveSelection`]). Carried out
//! of the area the editor is shown in, it leaves as a Wayland drag that other
//! applications can take ([`Shell::start_dnd`]); dropped back on the editor,
//! it moves as if it had never left. Let go without moving, the press is the
//! click it would have been.
//!
//! [`Gesture`] is the state of one such drag, kept apart from any widget so
//! that a widget drawing its own editor can drive it the same way.
//!
//! [`Edit::MoveSelection`]: crate::text_editor::Edit::MoveSelection
//! [`Shell::start_dnd`]: crate::core::Shell::start_dnd
use crate::core::Point;
use crate::core::dnd::{self, DndAction};
use crate::core::text::editor::Selection;

/// How far, in logical pixels, a press on the selection must move before it
/// carries the text.
pub const THRESHOLD: f32 = 6.0;

/// The MIME types any selection offers when it is dragged out, last of all:
/// the text itself.
pub const TEXT_MIME_TYPES: [&str; 3] = ["text/plain;charset=utf-8", "text/plain", "UTF8_STRING"];

/// What [`Gesture`] asks of the widget driving it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// Nothing to do.
    None,
    /// The press on the selection was let go where it was: click there.
    Click(Point),
    /// Move the selection to this point.
    Drop(Point),
    /// Start the Wayland drag that carries the selection out.
    DragOut,
    /// The drag ended without a drop on the editor.
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Phase {
    #[default]
    Idle,
    /// Held on the selection, not moved far enough yet.
    Pressed { from: Point },
    /// Carried inside the editor's area.
    Carried { at: Point },
    /// Out as a Wayland drag; `at` is where it is over the editor, if it is.
    Out { at: Option<Point> },
}

/// One drag of selected text.
///
/// Points are the caller's to choose, as long as it uses one space: the
/// text editor uses the text's own, as [`Action::Click`] does.
///
/// [`Action::Click`]: crate::text_editor::Action::Click
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Gesture {
    phase: Phase,
}

impl Gesture {
    /// A press at `at`. Returns whether the gesture takes it: only a single
    /// press on the selection is taken, and the caller must not click then.
    pub fn press(&mut self, at: Point, on_selection: bool) -> bool {
        if matches!(self.phase, Phase::Out { .. }) {
            return false;
        }

        self.phase = if on_selection {
            Phase::Pressed { from: at }
        } else {
            Phase::Idle
        };

        on_selection
    }

    /// The pointer moved to `at` while held; `inside` is whether that is
    /// still in the editor's area.
    pub fn motion(&mut self, at: Point, inside: bool) -> Outcome {
        match self.phase {
            Phase::Pressed { from } if at.distance(from) < THRESHOLD => Outcome::None,
            Phase::Pressed { .. } | Phase::Carried { .. } if !inside => {
                self.phase = Phase::Out { at: None };
                Outcome::DragOut
            }
            Phase::Pressed { .. } | Phase::Carried { .. } => {
                self.phase = Phase::Carried { at };
                Outcome::None
            }
            Phase::Idle | Phase::Out { .. } => Outcome::None,
        }
    }

    /// The button was let go at `at`; `over_text` is whether that is over
    /// the editor's text.
    pub fn release(&mut self, at: Point, over_text: bool) -> Outcome {
        let phase = std::mem::take(&mut self.phase);

        match phase {
            Phase::Pressed { from } => Outcome::Click(from),
            Phase::Carried { .. } if over_text => Outcome::Drop(at),
            Phase::Carried { .. } => Outcome::Cancel,
            // The compositor holds the pointer until the drag ends.
            Phase::Out { .. } => {
                self.phase = phase;
                Outcome::None
            }
            Phase::Idle => Outcome::None,
        }
    }

    /// Escape, or the editor lost the pointer: the text stays where it is.
    pub fn cancel(&mut self) -> Outcome {
        match std::mem::take(&mut self.phase) {
            Phase::Pressed { .. } | Phase::Carried { .. } => Outcome::Cancel,
            phase @ Phase::Out { .. } => {
                self.phase = phase;
                Outcome::None
            }
            Phase::Idle => Outcome::None,
        }
    }

    /// The Wayland drag the gesture started is over the editor at `at`, or
    /// left it (`None`). Returns whether the editor should accept it.
    pub fn hover(&mut self, at: Option<Point>) -> bool {
        if let Phase::Out { at: over } = &mut self.phase {
            *over = at;
            at.is_some()
        } else {
            false
        }
    }

    /// The Wayland drag was dropped on this window: on the editor, the
    /// selection moves there.
    pub fn dropped(&mut self) -> Outcome {
        match self.phase {
            Phase::Out { at: Some(at) } => {
                self.phase = Phase::Idle;
                Outcome::Drop(at)
            }
            _ => Outcome::None,
        }
    }

    /// An event for the source of the Wayland drag: once it ended, the
    /// gesture is over.
    pub fn source_event(&mut self, event: dnd::SourceEvent) -> Outcome {
        match (self.phase, event) {
            (
                Phase::Out { .. },
                dnd::SourceEvent::Cancelled
                | dnd::SourceEvent::DropPerformed
                | dnd::SourceEvent::Finished,
            ) => {
                self.phase = Phase::Idle;
                Outcome::Cancel
            }
            _ => Outcome::None,
        }
    }

    /// Whether a press on the selection is held or carried.
    pub fn is_active(&self) -> bool {
        self.phase != Phase::Idle
    }

    /// Whether the selection is out as a Wayland drag.
    pub fn is_out(&self) -> bool {
        matches!(self.phase, Phase::Out { .. })
    }

    /// Where the carried text would land, to draw a caret there.
    pub fn drop_point(&self) -> Option<Point> {
        match self.phase {
            Phase::Carried { at } | Phase::Out { at: Some(at) } => Some(at),
            _ => None,
        }
    }
}

/// Whether `point` is on the selected text.
pub fn is_on(selection: &Selection, point: Point) -> bool {
    match selection {
        Selection::Range(regions) => regions.iter().any(|region| region.contains(point)),
        Selection::Caret(_) => false,
    }
}

/// What a dragged-out `text` offers: `formats` first, in the order given,
/// then the text itself.
pub fn offer(text: &str, formats: Vec<(String, Vec<u8>)>) -> (Vec<String>, Vec<Vec<u8>>) {
    formats
        .into_iter()
        .chain(
            TEXT_MIME_TYPES
                .iter()
                .map(|mime| ((*mime).to_owned(), text.as_bytes().to_vec())),
        )
        .unzip()
}

/// The actions a dragged-out selection allows: the text is copied, never
/// taken from the editor.
pub const ACTIONS: DndAction = DndAction::COPY;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Rectangle;

    const FROM: Point = Point::new(10.0, 10.0);

    fn carried() -> Gesture {
        let mut gesture = Gesture::default();
        assert!(gesture.press(FROM, true));
        assert_eq!(gesture.motion(Point::new(40.0, 10.0), true), Outcome::None);
        gesture
    }

    #[test]
    fn a_press_off_the_selection_is_not_taken() {
        let mut gesture = Gesture::default();
        assert!(!gesture.press(FROM, false));
        assert_eq!(gesture.motion(Point::new(90.0, 10.0), true), Outcome::None);
        assert_eq!(gesture.release(Point::new(90.0, 10.0), true), Outcome::None);
        assert!(!gesture.is_active());
    }

    #[test]
    fn a_press_let_go_where_it_was_clicks_there() {
        let mut gesture = Gesture::default();
        assert!(gesture.press(FROM, true));
        assert_eq!(gesture.motion(Point::new(12.0, 11.0), true), Outcome::None);
        assert_eq!(
            gesture.drop_point(),
            None,
            "not carried within the threshold"
        );
        assert_eq!(
            gesture.release(Point::new(12.0, 11.0), true),
            Outcome::Click(FROM)
        );
        assert!(!gesture.is_active());
    }

    #[test]
    fn carried_and_let_go_over_the_text_moves_it_there() {
        let mut gesture = carried();
        assert_eq!(gesture.drop_point(), Some(Point::new(40.0, 10.0)));
        assert_eq!(
            gesture.release(Point::new(50.0, 30.0), true),
            Outcome::Drop(Point::new(50.0, 30.0))
        );
        assert!(!gesture.is_active());
    }

    #[test]
    fn let_go_off_the_text_or_escaped_leaves_it() {
        let mut gesture = carried();
        assert_eq!(
            gesture.release(Point::new(50.0, 30.0), false),
            Outcome::Cancel
        );

        let mut gesture = carried();
        assert_eq!(gesture.cancel(), Outcome::Cancel);
        assert_eq!(gesture.release(Point::new(50.0, 30.0), true), Outcome::None);
    }

    #[test]
    fn carried_out_of_the_area_it_leaves_once() {
        let mut gesture = carried();
        assert_eq!(
            gesture.motion(Point::new(-5.0, 10.0), false),
            Outcome::DragOut
        );
        assert!(gesture.is_out());
        assert_eq!(
            gesture.motion(Point::new(-50.0, 10.0), false),
            Outcome::None
        );
        assert_eq!(
            gesture.release(Point::new(-50.0, 10.0), false),
            Outcome::None
        );
        assert!(
            gesture.is_out(),
            "the compositor ends a drag, not the release"
        );
        assert!(!gesture.press(FROM, true), "no new drag while one is out");
        assert_eq!(gesture.cancel(), Outcome::None);
        assert!(gesture.is_out());
    }

    #[test]
    fn a_fast_pull_out_leaves_without_being_carried_first() {
        let mut gesture = Gesture::default();
        assert!(gesture.press(FROM, true));
        assert_eq!(
            gesture.motion(Point::new(-100.0, 10.0), false),
            Outcome::DragOut
        );
    }

    #[test]
    fn a_drag_out_ends_with_its_source() {
        for event in [
            dnd::SourceEvent::Cancelled,
            dnd::SourceEvent::DropPerformed,
            dnd::SourceEvent::Finished,
        ] {
            let mut gesture = carried();
            let _ = gesture.motion(Point::new(-5.0, 0.0), false);
            assert_eq!(
                gesture.source_event(dnd::SourceEvent::SendRequested),
                Outcome::None
            );
            assert_eq!(gesture.source_event(event), Outcome::Cancel);
            assert!(!gesture.is_active());
        }
        let mut idle = Gesture::default();
        assert_eq!(
            idle.source_event(dnd::SourceEvent::Finished),
            Outcome::None,
            "another widget's drag is not ours"
        );
    }

    #[test]
    fn a_drag_out_dropped_back_on_the_editor_moves_the_text() {
        let mut gesture = carried();
        let _ = gesture.motion(Point::new(-5.0, 0.0), false);
        assert!(gesture.hover(Some(Point::new(30.0, 30.0))));
        assert_eq!(gesture.drop_point(), Some(Point::new(30.0, 30.0)));
        assert_eq!(gesture.dropped(), Outcome::Drop(Point::new(30.0, 30.0)));
        assert!(!gesture.is_active());
    }

    #[test]
    fn a_drag_out_dropped_off_the_editor_is_not_taken() {
        let mut gesture = carried();
        let _ = gesture.motion(Point::new(-5.0, 0.0), false);
        assert!(gesture.hover(Some(Point::new(30.0, 30.0))));
        assert!(!gesture.hover(None));
        assert_eq!(gesture.dropped(), Outcome::None);
        assert!(gesture.is_out());

        let mut idle = Gesture::default();
        assert!(!idle.hover(Some(FROM)), "another app's drag is not taken");
    }

    #[test]
    fn the_selection_is_hit_by_its_regions() {
        let selection = Selection::Range(vec![
            Rectangle::new(Point::new(20.0, 0.0), crate::core::Size::new(80.0, 20.0)),
            Rectangle::new(Point::ORIGIN, crate::core::Size::new(40.0, 20.0))
                + crate::core::Vector::new(0.0, 20.0),
        ]);
        assert!(is_on(&selection, Point::new(30.0, 10.0)));
        assert!(is_on(&selection, Point::new(10.0, 30.0)));
        assert!(!is_on(&selection, Point::new(10.0, 10.0)));
        assert!(!is_on(&Selection::Caret(FROM), FROM));
    }

    #[test]
    fn a_dragged_out_selection_offers_the_app_formats_then_text() {
        let (mime_types, data) = offer(
            "héllo",
            vec![("application/x-example+json".to_owned(), b"{}".to_vec())],
        );
        assert_eq!(
            mime_types,
            [
                "application/x-example+json",
                "text/plain;charset=utf-8",
                "text/plain",
                "UTF8_STRING"
            ]
        );
        assert_eq!(data[0], b"{}");
        assert!(data[1..].iter().all(|d| d == "héllo".as_bytes()));
    }
}
