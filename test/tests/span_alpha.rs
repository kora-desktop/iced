//! A rich-text span's own colour is what is drawn, on either renderer: the colour the
//! paragraph is filled with does not scale it again.
//!
//! Its own test binary: the text system is built once per process.
use iced_test::core::renderer::Headless;
use iced_test::core::text::{Alignment, Ellipsis, LineHeight, Shaping, Wrapping};
use iced_test::core::text::{Paragraph as _, Renderer as _, Span, Text};
use iced_test::core::{Color, Font, Pixels, Point, Rectangle, Renderer as _, Size, alignment};
use iced_test::renderer::Renderer;
use iced_test::renderer::graphics::text;

const SIZE: u32 = 120;

/// The strongest alpha of a paragraph of one big glyph, drawn with `span` colour inside
/// a paragraph filled with `fill`, over nothing.
fn peak_alpha(backend: &str, span: Color, fill: Color) -> u8 {
    assert!(
        text::set_system_fonts(false),
        "the host's fonts are off for every capture"
    );

    let font = Font::with_name("Fira Sans");
    let mut renderer = block_on(<Renderer as Headless>::new(
        font,
        Pixels(16.0),
        Some(backend),
    ))
    .unwrap_or_else(|| panic!("the {backend} renderer is available"));

    assert_eq!(renderer.name(), backend, "asked for {backend}");

    let spans = [Span::<(), Font>::new("M").size(100.0).color(span)];
    let paragraph = <Renderer as iced_test::core::text::Renderer>::Paragraph::with_spans(Text {
        content: &spans[..],
        bounds: Size::new(SIZE as f32, SIZE as f32),
        size: Pixels(100.0),
        line_height: LineHeight::default(),
        font,
        align_x: Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: Shaping::Basic,
        wrapping: Wrapping::None,
        ellipsis: Ellipsis::None,
        hint_factor: None,
        letter_spacing: None,
    });

    let bounds = Rectangle::with_size(Size::new(SIZE as f32, SIZE as f32));
    renderer.reset(bounds);
    renderer.fill_paragraph(&paragraph, Point::ORIGIN, fill, bounds);

    let rgba = renderer.screenshot(Size::new(SIZE, SIZE), 1.0, Color::TRANSPARENT);
    rgba.as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| pixel[3])
        .max()
        .unwrap()
}

fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    iced_test::futures::futures::executor::block_on(future)
}

#[test]
fn a_span_colour_is_not_scaled_by_the_paragraph_colour_on_either_renderer() {
    let span = Color::from_rgba(0.0, 0.0, 0.0, 0.5);
    let fill = Color::from_rgba(0.0, 0.0, 0.0, 0.5);

    let wgpu = peak_alpha("wgpu", span, fill);
    let tiny_skia = peak_alpha("tiny-skia", span, fill);

    // Half of 255, once; not a quarter.
    assert!(wgpu.abs_diff(128) <= 2, "wgpu drew alpha {wgpu}");
    assert!(
        tiny_skia.abs_diff(wgpu) <= 2,
        "tiny-skia drew alpha {tiny_skia}, wgpu {wgpu}"
    );
}
