//! An image wider than the atlas is drawn in fragments; turned, they must land
//! where the whole image turns to. These draw on a GPU, so they are ignored by
//! default: `cargo test -p iced_wgpu --features image -- --ignored image_turn_tests`.
use crate::Renderer;
use crate::core::image::{Handle, Image, Renderer as _};
use crate::core::renderer::Headless;
use crate::core::{Color, Font, Pixels, Point, Radians, Rectangle, Renderer as _, Size};
use crate::graphics::Viewport;

use std::f32::consts::{FRAC_PI_2, PI};

const SCREEN: u32 = 2000;
/// Wider than the 2048 atlas, so it is cut in two fragments.
const WIDTH: u32 = 3000;
const HEIGHT: u32 = 1000;
/// Drawn 1800 by 600 in the middle of the screen: over half its size, so it
/// is uploaded whole rather than halved.
const SCALE: f32 = 0.6;

/// Red on its left half and blue on its right.
fn halves() -> Handle {
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for _ in 0..HEIGHT {
        for x in 0..WIDTH {
            pixels.extend(if x < WIDTH / 2 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            });
        }
    }
    Handle::from_rgba(WIDTH, HEIGHT, pixels)
}

/// The screen's pixels with the image drawn turned `rotation` clockwise, once
/// it is uploaded: a large image goes up off the frame that first draws it.
fn turned(rotation: f32) -> Vec<u8> {
    let mut renderer = futures::executor::block_on(<Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        None,
    ))
    .expect("a GPU adapter");
    let screen = Rectangle::with_size(Size::new(SCREEN as f32, SCREEN as f32));
    let size = Size::new(WIDTH as f32 * SCALE, HEIGHT as f32 * SCALE);
    let middle = SCREEN as f32 / 2.0;
    let bounds = Rectangle::new(
        Point::new(middle - size.width / 2.0, middle - size.height / 2.0),
        size,
    );
    let handle = halves();
    let mut pixels = Vec::new();
    for _ in 0..100 {
        renderer.reset(screen);
        let mut image = Image::new(handle.clone());
        image.rotation = Radians(rotation);
        renderer.draw_image(image, bounds, screen);
        pixels = renderer.screenshot(
            &Viewport::with_physical_size(Size::new(SCREEN, SCREEN), 1.0),
            Color::TRANSPARENT,
        );
        if at(&pixels, SCREEN / 2, SCREEN / 2) != '.' {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    pixels
}

/// The colour at `(x, y)`: `'r'`, `'b'`, nothing drawn `'.'`, or `'?'`.
fn at(pixels: &[u8], x: u32, y: u32) -> char {
    let i = ((y * SCREEN + x) * 4) as usize;
    match (pixels[i], pixels[i + 2], pixels[i + 3]) {
        (_, _, a) if a < 8 => '.',
        (r, b, _) if r > 200 && b < 40 => 'r',
        (r, b, _) if b > 200 && r < 40 => 'b',
        _ => '?',
    }
}

/// Every pixel of the turned image, two in from its edges, is `want(x, y)`.
fn assert_drawn(pixels: &[u8], area: (u32, u32, u32, u32), want: impl Fn(u32, u32) -> char) {
    let (x0, y0, x1, y1) = area;
    for y in y0 + 2..y1 - 2 {
        for x in x0 + 2..x1 - 2 {
            let got = at(pixels, x, y);
            let want = want(x, y);
            // The seam between the halves is blended.
            if got == '?' && (want == 'r' || want == 'b') {
                continue;
            }
            assert_eq!(got, want, "at ({x}, {y})");
        }
    }
}

#[test]
#[ignore = "needs a GPU adapter"]
fn a_wide_image_turned_a_quarter_draws_whole() {
    // Standing 600 by 1800: its left half on top.
    let pixels = turned(FRAC_PI_2);
    assert_drawn(&pixels, (700, 100, 1300, 1900), |_, y| {
        if y < 1000 { 'r' } else { 'b' }
    });
    assert_eq!(at(&pixels, 400, 1000), '.', "nothing beside it");
}

#[test]
#[ignore = "needs a GPU adapter"]
fn a_wide_image_turned_three_quarters_draws_whole() {
    let pixels = turned(3.0 * FRAC_PI_2);
    assert_drawn(&pixels, (700, 100, 1300, 1900), |_, y| {
        if y < 1000 { 'b' } else { 'r' }
    });
}

#[test]
#[ignore = "needs a GPU adapter"]
fn a_wide_image_turned_a_half_draws_whole() {
    let pixels = turned(PI);
    assert_drawn(&pixels, (100, 700, 1900, 1300), |x, _| {
        if x < 1000 { 'b' } else { 'r' }
    });
}

/// Drawn small, a turned image must still cover every pixel of its box. The
/// bounds are rounded to whole pixels, so an image's scale differs a little
/// across its axes; a laptop showed a 3840 by 2160 picture, held whole and
/// turned a quarter, with an empty row where its two fragments meet.
#[test]
#[ignore = "needs a GPU adapter"]
fn a_turned_image_drawn_small_leaves_no_gap() {
    let mut renderer = futures::executor::block_on(<Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        None,
    ))
    .expect("a GPU adapter");
    let mut gaps = Vec::new();

    // In fragments and whole, wide and tall, and one whose fragments meet
    // on a pixel's centre when drawn an odd number of pixels wide.
    for (width, height) in [
        (3840, 2160),
        (2160, 3840),
        (4096, 2048),
        (3000, 1000),
        (1600, 900),
    ] {
        let handle = opaque(width, height);
        for scale in [1.0, 1.5] {
            // Held at its own size, as any zoom past half draws it.
            let own = Size::new(width as f32 / scale, height as f32 / scale);
            let _ = draw(
                &mut renderer,
                &handle,
                Rectangle::with_size(own),
                0.0,
                scale,
            );

            for step in 0..48 {
                // Fit and around it: from a fifth of the screen to most of it.
                let long = (SCREEN as f32 / 5.0 + step as f32 * 13.7) / scale;
                let size = if width >= height {
                    Size::new(long, long * height as f32 / width as f32)
                } else {
                    Size::new(long * width as f32 / height as f32, long)
                };
                let middle = SCREEN as f32 / scale / 2.0;
                // Off the pixel grid, as a centred picture usually is.
                let bounds = Rectangle::new(
                    Point::new(
                        middle - size.width / 2.0 + step as f32 * 0.13,
                        middle - size.height / 2.0 + step as f32 * 0.29,
                    ),
                    size,
                );

                for turns in 0..4 {
                    let rotation = turns as f32 * FRAC_PI_2;
                    let pixels = draw(&mut renderer, &handle, bounds, rotation, scale);
                    if let Some(gap) = first_gap(&pixels, bounds * scale, turns % 2 == 1) {
                        gaps.push(format!(
                            "{width}x{height} at {:.1}x{:.1}px, scale {scale}, {turns} turns: {gap}",
                            size.width * scale,
                            size.height * scale,
                        ));
                    }
                }
            }
        }
    }

    assert!(gaps.is_empty(), "{} gaps:\n{}", gaps.len(), gaps.join("\n"));
}

/// An opaque white image.
fn opaque(width: u32, height: u32) -> Handle {
    Handle::from_rgba(width, height, vec![255; (width * height * 4) as usize])
}

/// The screen's pixels with `handle` drawn over `bounds` at `scale`, once it
/// is uploaded.
fn draw(
    renderer: &mut Renderer,
    handle: &Handle,
    bounds: Rectangle,
    rotation: f32,
    scale: f32,
) -> Vec<u8> {
    let screen = Rectangle::with_size(Size::new(SCREEN as f32 / scale, SCREEN as f32 / scale));
    let viewport = Viewport::with_physical_size(Size::new(SCREEN, SCREEN), scale);
    let mut pixels = Vec::new();
    for _ in 0..100 {
        renderer.reset(screen);
        let mut image = Image::new(handle.clone());
        image.rotation = Radians(rotation);
        renderer.draw_image(image, bounds, screen);
        pixels = renderer.screenshot(&viewport, Color::TRANSPARENT);
        let middle = bounds.center();
        let (x, y) = ((middle.x * scale) as u32, (middle.y * scale) as u32);
        if pixels[((y * SCREEN + x) * 4 + 3) as usize] > 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    pixels
}

/// The first pixel inside the turned box of `bounds`, in screen pixels and
/// one in from its edges, that the image does not cover.
fn first_gap(pixels: &[u8], bounds: Rectangle, quarter: bool) -> Option<String> {
    let bounds = bounds.round();
    let center = bounds.center();
    let (width, height) = if quarter {
        (bounds.height, bounds.width)
    } else {
        (bounds.width, bounds.height)
    };
    let (left, top) = (center.x - width / 2.0, center.y - height / 2.0);
    let (x0, y0) = (left.ceil() as u32 + 1, top.ceil() as u32 + 1);
    let (x1, y1) = (
        (left + width).floor() as u32 - 1,
        (top + height).floor() as u32 - 1,
    );

    for y in y0..y1 {
        for x in x0..x1 {
            let alpha = pixels[((y * SCREEN + x) * 4 + 3) as usize];
            if alpha < 255 {
                return Some(format!(
                    "alpha {alpha} at ({}, {}) of its {width}x{height} box",
                    x as f32 - left,
                    y as f32 - top,
                ));
            }
        }
    }
    None
}
