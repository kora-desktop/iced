//! A partial redraw of a layer with rounded corners paints its damage and
//! nothing else, as a square layer's does.
use crate::Renderer;
use crate::core::renderer::Quad;
use crate::core::{Color, Font, Pixels, Point, Rectangle, Renderer as _, Size};
use crate::graphics::Viewport;

const WIDTH: u32 = 40;
const HEIGHT: u32 = 40;

fn screen() -> Rectangle {
    Rectangle::with_size(Size::new(WIDTH as f32, HEIGHT as f32))
}

fn quad(renderer: &mut Renderer, bounds: Rectangle, color: Color) {
    renderer.fill_quad(
        Quad {
            bounds,
            ..Quad::default()
        },
        color,
    );
}

fn draw(renderer: &mut Renderer, pixels: &mut tiny_skia::Pixmap, damage: Rectangle) {
    let viewport = Viewport::with_physical_size(Size::new(WIDTH, HEIGHT), 1.0);
    let mut mask = tiny_skia::Mask::new(WIDTH, HEIGHT).unwrap();
    renderer.draw(
        &mut pixels.as_mut(),
        &mut mask,
        &viewport,
        &[damage],
        Color::TRANSPARENT,
    );
}

#[test]
fn a_redraw_inside_a_rounded_layer_keeps_what_it_did_not_damage() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
    renderer.reset(screen());
    // A card filling a rounded window, and a mark on it near the top.
    renderer.with_layer_rounded(screen(), 8.0.into(), |renderer| {
        quad(renderer, screen(), Color::WHITE);
        quad(
            renderer,
            Rectangle::new(Point::new(10.0, 10.0), Size::new(10.0, 10.0)),
            Color::BLACK,
        );
    });
    let mut pixels = tiny_skia::Pixmap::new(WIDTH, HEIGHT).unwrap();
    draw(&mut renderer, &mut pixels, screen());
    assert_eq!(pixels.pixel(15, 15).unwrap().red(), 0, "the mark is drawn");

    // Something changed near the bottom only: the card is painted there again.
    draw(
        &mut renderer,
        &mut pixels,
        Rectangle::new(Point::new(0.0, 30.0), Size::new(WIDTH as f32, 10.0)),
    );

    assert_eq!(
        pixels.pixel(15, 15).unwrap().red(),
        0,
        "the card is not painted over the mark, outside the damage"
    );
    assert_eq!(pixels.pixel(20, 35).unwrap().red(), 255, "the damage is");
}
