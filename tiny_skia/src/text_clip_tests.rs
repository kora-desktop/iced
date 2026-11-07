//! Text drawn with `fill_text` stays inside its clip bounds, even when those sit
//! inside the layer, as a pick list's value inside a scrollable does.
use crate::Renderer;
use crate::core::text::{Alignment, Ellipsis, LineHeight, Renderer as _, Shaping, Text, Wrapping};
use crate::core::{Color, Font, Pixels, Point, Rectangle, Renderer as _, Size, alignment};
use crate::graphics::Viewport;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 72;

/// About 190px of text at 20px, wider than the screen.
const WIDE: &str = "WWWWWWWWWW";

fn renderer() -> Renderer {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
    renderer.reset(screen());
    renderer
}

fn screen() -> Rectangle {
    Rectangle::with_size(Size::new(WIDTH as f32, HEIGHT as f32))
}

/// One line of `content`, whatever `width` says.
fn text(
    renderer: &mut Renderer,
    content: &str,
    position: Point,
    width: f32,
    clip_bounds: Rectangle,
) {
    renderer.fill_text(
        Text {
            content: content.to_owned(),
            bounds: Size::new(width, 24.0),
            size: Pixels(20.0),
            line_height: LineHeight::default(),
            font: Font::DEFAULT,
            align_x: Alignment::Left,
            align_y: alignment::Vertical::Top,
            shaping: Shaping::default(),
            wrapping: Wrapping::None,
            ellipsis: Ellipsis::default(),
            hint_factor: None,
            letter_spacing: None,
        },
        position,
        Color::BLACK,
        clip_bounds,
    );
}

fn render(renderer: &mut Renderer) -> tiny_skia::Pixmap {
    let mut pixels = tiny_skia::Pixmap::new(WIDTH, HEIGHT).unwrap();
    let viewport = Viewport::with_physical_size(Size::new(WIDTH, HEIGHT), 1.0);
    let mut mask = tiny_skia::Mask::new(WIDTH, HEIGHT).unwrap();
    renderer.draw(
        &mut pixels.as_mut(),
        &mut mask,
        &viewport,
        &[screen()],
        Color::TRANSPARENT,
    );
    pixels
}

/// Whether any pixel in columns `from..to` was drawn.
fn inked(pixels: &tiny_skia::Pixmap, from: u32, to: u32) -> bool {
    (0..HEIGHT).any(|y| (from..to).any(|x| pixels.pixel(x, y).unwrap().alpha() > 0))
}

#[test]
fn text_past_a_clip_inside_its_layer_is_clipped() {
    let mut renderer = renderer();
    renderer.with_layer(
        Rectangle::new(Point::ORIGIN, Size::new(140.0, HEIGHT as f32)),
        |renderer| {
            text(
                renderer,
                WIDE,
                Point::new(0.0, 24.0),
                WIDTH as f32,
                Rectangle::new(Point::ORIGIN, Size::new(50.0, HEIGHT as f32)),
            );
        },
    );
    let pixels = render(&mut renderer);

    assert!(inked(&pixels, 0, 50), "the text inside the clip is drawn");
    assert!(!inked(&pixels, 50, WIDTH), "the text past the clip is not");
}

#[test]
fn text_past_its_own_bounds_is_clipped() {
    let mut renderer = renderer();
    text(
        &mut renderer,
        WIDE,
        Point::new(20.0, 24.0),
        15.0,
        Rectangle::new(Point::ORIGIN, Size::new(60.0, HEIGHT as f32)),
    );
    let pixels = render(&mut renderer);

    assert!(
        inked(&pixels, 40, 60),
        "the overflow inside the clip is drawn"
    );
    assert!(
        !inked(&pixels, 60, WIDTH),
        "the overflow past the clip is not"
    );
}

#[test]
fn text_inside_its_clip_is_drawn_whole() {
    let draw = |clip_bounds: Rectangle| {
        let mut renderer = renderer();
        text(
            &mut renderer,
            "Wi",
            Point::new(4.0, 24.0),
            40.0,
            clip_bounds,
        );
        render(&mut renderer)
    };

    assert_eq!(
        draw(Rectangle::new(
            Point::new(2.0, 0.0),
            Size::new(56.0, HEIGHT as f32)
        ))
        .data(),
        draw(Rectangle::INFINITE).data(),
    );
}
