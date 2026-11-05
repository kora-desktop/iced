//! Rich text driven by the selection callbacks counts in bytes across its
//! whole text, newlines included: a press on the second line reports an offset
//! past the first, and a range lights only the line it falls on.
//!
//! They need a renderer that measures text, which this crate's tests don't build by
//! default, so they return early without one. Run them with:
//!
//! ```sh
//! ICED_TEST_BACKEND=tiny-skia cargo test -p iced_widget --test rich_text_selection_lines \
//!     --features iced_renderer/tiny-skia,iced_renderer/wayland,iced_renderer/x11
//! ```
//!
//! For wgpu, set `ICED_TEST_BACKEND=wgpu` and add `iced_renderer/wgpu,iced_renderer/wgpu-bare`.
#![cfg(not(feature = "wgpu"))]
use iced_test::Simulator;
use iced_widget::core::text::{LineHeight, Span};
use iced_widget::core::{Color, Element, Event, Point, Settings, Theme, mouse};
use iced_widget::{rich_text, span};

type Renderer = iced_renderer::Renderer;
type Ui<'a> = Simulator<'a, Message, Theme, Renderer>;

/// Whether the renderer measures text; the null one this crate builds by default doesn't.
fn measures_text() -> bool {
    std::any::TypeId::of::<Renderer>() != std::any::TypeId::of::<()>()
}

const FIRST: &str = "first line\n";
const TEXT: &str = "first line\nsecond line";
const LINE: f32 = 20.0;
const MARK: Color = Color::from_rgb(1.0, 0.0, 0.0);

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Start(usize),
    Drag(usize),
    End,
}

fn text<'a>(selection: Option<(usize, usize)>) -> Ui<'a> {
    let spans: [Span<'a, ()>; 1] = [span(TEXT)];
    let text: Element<'a, Message, Theme, Renderer> = rich_text(spans)
        .line_height(LineHeight::Absolute(LINE.into()))
        .selection(selection)
        .selection_color(MARK)
        .on_selection_start(Message::Start)
        .on_selection_drag(Message::Drag)
        .on_selection_end(|| Message::End)
        .into();

    Simulator::with_size(Settings::default(), (300.0, 3.0 * LINE), text).scale_factor(1.0)
}

/// The press offset at `x` on the given line.
fn pressed(line: usize, x: f32) -> Option<usize> {
    let mut ui = text(None);
    let at = Point::new(x, line as f32 * LINE + LINE / 2.0);

    ui.point_at(at);
    let _ = ui.simulate([
        Event::Mouse(mouse::Event::CursorMoved { position: at }),
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
    ]);

    ui.into_messages().find_map(|message| match message {
        Message::Start(offset) => Some(offset),
        _ => None,
    })
}

/// Which lines the highlight of `range` is drawn on.
fn lit_lines(range: (usize, usize)) -> Vec<usize> {
    let mut ui = text(Some(range));
    let snapshot = ui.snapshot(&Theme::Light).expect("snapshot");
    let shot = snapshot.screenshot();
    let width = shot.size.width as usize;

    let mut lines: Vec<usize> = shot
        .rgba
        .chunks_exact(4)
        .enumerate()
        .filter(|(_, pixel)| pixel[0] > 200 && pixel[1] < 60 && pixel[2] < 60)
        .map(|(at, _)| ((at / width) as f32 / LINE) as usize)
        .collect();

    lines.dedup();
    lines
}

#[test]
fn a_press_on_the_second_line_counts_the_first() {
    if !measures_text() {
        return;
    }

    assert_eq!(pressed(0, 0.5), Some(0));
    assert_eq!(pressed(1, 0.5), Some(FIRST.len()));
    assert_eq!(pressed(1, 290.0), Some(TEXT.len()));
}

#[test]
fn a_range_lights_only_the_line_it_falls_on() {
    if !measures_text() {
        return;
    }

    assert_eq!(lit_lines((0, "first".len())), [0]);
    assert_eq!(lit_lines((FIRST.len(), FIRST.len() + "second".len())), [1]);
    assert_eq!(lit_lines((3, FIRST.len() + 3)), [0, 1]);
}
