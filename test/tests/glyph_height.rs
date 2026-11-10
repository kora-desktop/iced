//! A glyph is drawn as tall as its outline at any scale: a font's own hinting
//! program must not stretch it to the pixel grid. Geist's raises its x-height to
//! the next whole pixel and scales the glyph with it, so at 14px on a 1.5 scale
//! its capitals came out 16px tall instead of 14.9.
//!
//! Its own test binary: the text system is built once per process.
use std::borrow::Cow;

use iced_test::core::renderer::Headless;
use iced_test::core::text::{Alignment, Ellipsis, LineHeight, Shaping, Wrapping};
use iced_test::core::text::{Paragraph as _, Renderer as _, Text};
use iced_test::core::{Color, Font, Pixels, Point, Rectangle, Renderer as _, Size, alignment};
use iced_test::renderer::Renderer;
use iced_test::renderer::graphics::text;

const GEIST: &[u8] = include_bytes!("../../examples/letter_spacing/fonts/Geist-Regular.ttf");
/// Geist's cap height and x-height, in ems (`OS/2`: 710 and 530 of 1000).
const CAP_HEIGHT: f32 = 0.71;
const X_HEIGHT: f32 = 0.53;
const SIZE: f32 = 14.0;
const BOX: Size<u32> = Size::new(60, 60);

/// The ink height of `glyph`, in physical pixels: each row counts as much as
/// its strongest pixel covers, so a partly covered edge row counts in part.
fn ink_height(backend: &str, glyph: &str, scale: f32) -> f32 {
    assert!(
        text::set_system_fonts(false),
        "the host's fonts are off for every capture"
    );
    text::font_system()
        .write()
        .expect("Write font system")
        .load_font(Cow::Borrowed(GEIST));

    let font = Font::with_name("Geist");
    let mut renderer = block_on(<Renderer as Headless>::new(
        font,
        Pixels(SIZE),
        Some(backend),
    ))
    .unwrap_or_else(|| panic!("the {backend} renderer is available"));
    assert_eq!(renderer.name(), backend, "asked for {backend}");

    let logical = Size::new(BOX.width as f32, BOX.height as f32);
    let paragraph = <Renderer as iced_test::core::text::Renderer>::Paragraph::with_text(Text {
        content: glyph,
        bounds: logical,
        size: Pixels(SIZE),
        line_height: LineHeight::default(),
        font,
        align_x: Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
        ellipsis: Ellipsis::None,
        hint_factor: None,
        letter_spacing: None,
    });

    let bounds = Rectangle::with_size(logical);
    renderer.reset(bounds);
    renderer.fill_paragraph(&paragraph, Point::new(10.0, 10.0), Color::BLACK, bounds);

    let physical = Size::new(
        (BOX.width as f32 * scale) as u32,
        (BOX.height as f32 * scale) as u32,
    );
    let rgba = renderer.screenshot(physical, scale, Color::TRANSPARENT);
    let rows: Vec<u32> = rgba
        .chunks(physical.width as usize * 4)
        .map(|row| row.chunks(4).map(|pixel| u32::from(pixel[3])).sum())
        .filter(|ink| *ink > 0)
        .collect();
    let mut sorted = rows.clone();
    sorted.sort_unstable();
    let full = sorted[sorted.len() / 2] as f32;
    rows.iter().map(|ink| *ink as f32 / full).sum()
}

fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    iced_test::futures::futures::executor::block_on(future)
}

#[test]
fn a_glyph_is_as_tall_as_its_outline_on_either_renderer() {
    for backend in ["wgpu", "tiny-skia"] {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let px = SIZE * scale;
            for (glyph, em) in [("I", CAP_HEIGHT), ("ı", X_HEIGHT)] {
                let drawn = ink_height(backend, glyph, scale);
                let outline = em * px;
                assert!(
                    (drawn - outline).abs() < 0.25,
                    "{backend} at {scale}x drew {glyph} {drawn:.2}px tall; its outline is {outline:.2}px"
                );
            }
        }
    }
}
