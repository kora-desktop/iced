//! `max_lines` shows at most that many lines and ellipsizes the last one, as CSS
//! `-webkit-line-clamp` does; shorter text keeps its own height.
//!
//! They need a renderer that measures text, which this crate's tests don't build by
//! default, so they return early without one. Run them with:
//!
//! ```sh
//! ICED_TEST_BACKEND=tiny-skia cargo test -p iced_widget --test text_max_lines \
//!     --features iced_renderer/tiny-skia,iced_renderer/wayland,iced_renderer/x11
//! ```
//!
//! For wgpu, set `ICED_TEST_BACKEND=wgpu` and add `iced_renderer/wgpu,iced_renderer/wgpu-bare`.
#![cfg(not(feature = "wgpu"))]
use iced_test::Simulator;
use iced_widget::core::text::{Ellipsis, Wrapping};
use iced_widget::core::{Color, Element, Length, Pixels, Settings, Theme};
use iced_widget::{column, container, text};

type Renderer = iced_renderer::Renderer;

/// Whether the renderer measures text; the null one this crate builds by default doesn't.
fn measures_text() -> bool {
    std::any::TypeId::of::<Renderer>() != std::any::TypeId::of::<()>()
}

const LINE: f32 = 24.0;
const WIDTH: f32 = 120.0;
const LONG: &str = "One two three four five six seven eight nine ten eleven twelve thirteen";
const SHORT: &str = "One two";

fn line<'a>(content: &'a str) -> text::Text<'a, Theme, Renderer> {
    text(content)
        .size(16)
        .line_height(Pixels(LINE))
        .color(Color::BLACK)
}

fn simulate(element: Element<'_, (), Theme, Renderer>) -> Simulator<'_, (), Theme, Renderer> {
    let boxed = container(element).width(WIDTH);
    Simulator::with_size(Settings::default(), (WIDTH, 400.0), boxed).scale_factor(1.0)
}

fn height(content: &str, text: text::Text<'_, Theme, Renderer>) -> f32 {
    let mut ui = simulate(text.into());
    ui.find(content)
        .expect("the text is laid out")
        .bounds()
        .height
}

fn pixels(text: text::Text<'_, Theme, Renderer>) -> Vec<u8> {
    let mut ui = simulate(text.into());
    let snapshot = ui.snapshot(&Theme::Light).expect("snapshot");
    snapshot.screenshot().rgba.to_vec()
}

#[test]
fn long_text_stops_at_its_line_limit() {
    if !measures_text() {
        return;
    }

    assert!(
        height(LONG, line(LONG)) > 2.0 * LINE,
        "the text needs more lines"
    );
    assert_eq!(height(LONG, line(LONG).max_lines(2)), 2.0 * LINE);
    assert_eq!(height(LONG, line(LONG).max_lines(1)), LINE);
}

#[test]
fn shorter_text_keeps_its_own_height() {
    if !measures_text() {
        return;
    }

    assert_eq!(height(SHORT, line(SHORT).max_lines(3)), LINE);
}

#[test]
fn the_last_line_ends_in_an_ellipsis() {
    if !measures_text() {
        return;
    }

    let two_lines = Length::Fixed(2.0 * LINE);
    let clamped = pixels(line(LONG).max_lines(2));

    assert_eq!(
        clamped,
        pixels(line(LONG).height(two_lines).ellipsis(Ellipsis::End)),
        "ends like a two-line box ellipsized at its end",
    );
    assert_ne!(
        clamped,
        pixels(line(LONG).height(two_lines)),
        "differs from the same box merely cut off",
    );
}

#[test]
fn an_ellipsis_set_on_the_text_wins() {
    if !measures_text() {
        return;
    }

    assert_ne!(
        pixels(line(LONG).max_lines(2)),
        pixels(line(LONG).max_lines(2).ellipsis(Ellipsis::Middle)),
    );
}

#[test]
fn unwrapped_text_shows_one_line() {
    if !measures_text() {
        return;
    }

    assert_eq!(
        height(LONG, line(LONG).wrapping(Wrapping::None).max_lines(3)),
        LINE
    );
}

#[test]
fn a_column_below_starts_where_the_last_line_ends() {
    if !measures_text() {
        return;
    }

    let mut ui = simulate(column![line(LONG).max_lines(2), line(SHORT)].into());

    assert_eq!(
        ui.find(SHORT).expect("the second text").bounds().y,
        2.0 * LINE
    );
}
