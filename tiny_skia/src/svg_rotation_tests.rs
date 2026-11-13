//! An SVG with a rotation is drawn turned on its center, as wgpu draws it.
use crate::Renderer;
use crate::core::svg::{Handle, Renderer as _, Svg};
use crate::core::{Color, Font, Pixels, Point, Radians, Rectangle, Renderer as _, Size};
use crate::graphics::Viewport;

const WIDTH: u32 = 40;
const HEIGHT: u32 = 40;

/// A 20px square, black in its top half.
const TOP_HALF: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 20 20"><rect width="20" height="10" fill="black"/></svg>"#;

fn screen() -> Rectangle {
    Rectangle::with_size(Size::new(WIDTH as f32, HEIGHT as f32))
}

/// The square drawn in the middle of the screen, turned by `radians`.
fn drawn(radians: f32) -> tiny_skia::Pixmap {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
    renderer.reset(screen());
    renderer.draw_svg(
        Svg::new(Handle::from_memory(TOP_HALF)).rotation(Radians(radians)),
        Rectangle::new(Point::new(10.0, 10.0), Size::new(20.0, 20.0)),
        screen(),
    );
    let viewport = Viewport::with_physical_size(Size::new(WIDTH, HEIGHT), 1.0);
    let mut pixels = tiny_skia::Pixmap::new(WIDTH, HEIGHT).unwrap();
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

fn inked(pixels: &tiny_skia::Pixmap, x: u32, y: u32) -> bool {
    pixels.pixel(x, y).unwrap().alpha() > 200
}

#[test]
fn an_upright_svg_is_drawn_as_it_is() {
    let pixels = drawn(0.0);
    assert!(inked(&pixels, 20, 13), "the top half is black");
    assert!(!inked(&pixels, 20, 27), "the bottom half is clear");
}

#[test]
fn a_half_turn_puts_the_top_at_the_bottom() {
    let pixels = drawn(std::f32::consts::PI);
    assert!(!inked(&pixels, 20, 13), "the top half is clear");
    assert!(inked(&pixels, 20, 27), "the bottom half is black");
}

#[test]
fn a_quarter_turn_puts_the_top_on_the_right() {
    // Clockwise on screen, as raster images and the wgpu pipeline turn.
    let pixels = drawn(std::f32::consts::FRAC_PI_2);
    assert!(inked(&pixels, 27, 20), "the right half is black");
    assert!(!inked(&pixels, 13, 20), "the left half is clear");
}
