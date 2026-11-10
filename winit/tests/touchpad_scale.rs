//! A touchpad swipe moves a scrollable by the distance the fingers travelled, in
//! logical pixels, whatever the window's scale factor. winit reports the swipe in
//! physical pixels.
use iced_test::Simulator;
use iced_widget::core::{Element, Event, Length, Point, Theme};
use iced_widget::{column, container, scrollable, text};
use iced_winit::conversion;
use winit::dpi::LogicalPosition;
use winit::event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent};
use winit::keyboard::ModifiersState;

type Renderer = iced_renderer::Renderer;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Scrolled(f32);

/// 50 rows of 40px in a 200px viewport: 1800px of scroll range.
fn list() -> Element<'static, Scrolled, Theme, Renderer> {
    let rows = (0..50).map(|i| {
        container(text(format!("row {i}")))
            .height(40)
            .width(Length::Fill)
            .into()
    });

    scrollable(column(rows))
        .height(200)
        .on_scroll(|viewport| Scrolled(viewport.absolute_offset().y))
        .into()
}

/// What winit sends for a swipe of `finger` logical pixels upwards at `scale`.
fn swipe(finger: f64, scale: f64) -> WindowEvent {
    WindowEvent::MouseWheel {
        device_id: DeviceId::dummy(),
        delta: MouseScrollDelta::PixelDelta(LogicalPosition::new(0.0, -finger).to_physical(scale)),
        phase: TouchPhase::Moved,
    }
}

fn scrolled_by(finger: f64, steps: usize, scale: f32) -> f32 {
    let mut ui = Simulator::new(list());
    ui.point_at(Point::new(50.0, 50.0));

    let events: Vec<Event> = (0..steps)
        .map(|_| {
            conversion::window_event(
                swipe(finger, f64::from(scale)),
                scale,
                ModifiersState::empty(),
            )
            .expect("a wheel event")
        })
        .collect();
    let _ = ui.simulate(events);

    ui.into_messages()
        .map(|Scrolled(y)| y)
        .last()
        .unwrap_or(0.0)
}

#[test]
fn a_swipe_scrolls_by_the_finger_distance_at_any_scale() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let moved = scrolled_by(4.0, 8, scale);

        assert!(
            (moved - 32.0).abs() < 1e-3,
            "at scale {scale} eight 4px swipes moved the content {moved}px"
        );
    }
}
