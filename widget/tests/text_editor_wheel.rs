//! A text editor scrolls a high-resolution wheel's fractions of a notch as far
//! as the whole notch they add up to, not a line each.
//!
//! Skipped with the `wgpu` feature: it builds the renderer without a backend,
//! so the simulator can't make one.
#![cfg(not(feature = "wgpu"))]
use iced_test::Simulator;
use iced_widget::core::{Element, Event, Point, Theme, mouse};
use iced_widget::text_editor::{Action, Content, TextEditor};

type Renderer = iced_renderer::Renderer;

/// The lines the editor asks to scroll for `wheel`, a notch down per 1.0.
fn scrolled(wheel: &[f32]) -> i32 {
    let content = Content::<Renderer>::with_text(&"line\n".repeat(100));
    let editor: Element<'_, Action, Theme, Renderer> = TextEditor::new(&content)
        .height(100)
        .on_action(|action| action)
        .into();

    let mut ui = Simulator::new(editor);
    ui.point_at(Point::new(20.0, 20.0));

    let _ = ui.simulate(wheel.iter().map(|lines| {
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: -lines },
        })
    }));

    ui.into_messages()
        .map(|action| match action {
            Action::Scroll { lines } => lines,
            other => panic!("only scrolls: {other:?}"),
        })
        .sum()
}

#[test]
fn a_notch_scrolls_four_lines() {
    assert_eq!(scrolled(&[1.0]), 4);
}

#[test]
fn eight_eighths_of_a_notch_scroll_as_far_as_a_notch() {
    assert_eq!(scrolled(&[0.125; 8]), 4);
}

#[test]
fn a_wheel_up_scrolls_back() {
    assert_eq!(scrolled(&[-0.125; 8]), -4);
}
