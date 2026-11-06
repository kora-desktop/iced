//! A device's errors are counted and logged, never panicked on. These run on
//! wgpu's noop backend, which needs no GPU.
use crate::core::renderer::{Quad, Renderer as _};
use crate::core::{Color, Font, Pixels, Rectangle, Size};
use crate::graphics::{Shell, Viewport};
use crate::{Engine, Health, Renderer};

use std::sync::Arc;

const SIDE: u32 = 8;

fn noop_engine() -> Engine {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::NOOP,
        backend_options: wgpu::BackendOptions {
            noop: wgpu::NoopBackendOptions { enable: true },
            ..wgpu::BackendOptions::default()
        },
        ..wgpu::InstanceDescriptor::default()
    });

    let adapter = futures::executor::block_on(
        instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
    )
    .expect("the noop adapter");

    let (device, queue) =
        futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("a noop device");

    Engine::new(
        &adapter,
        device,
        queue,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        None,
        Shell::headless(),
    )
}

fn out_of_memory() -> wgpu::Error {
    wgpu::Error::OutOfMemory {
        source: Box::new(std::io::Error::other("injected")),
    }
}

fn validation_error() -> wgpu::Error {
    wgpu::Error::Validation {
        source: Box::new(std::io::Error::other("injected")),
        description: "injected".to_owned(),
    }
}

fn draw_a_frame(renderer: &mut Renderer) -> Vec<u8> {
    let screen = Rectangle::with_size(Size::new(SIDE as f32, SIDE as f32));

    renderer.reset(screen);
    renderer.fill_quad(
        Quad {
            bounds: screen,
            ..Quad::default()
        },
        Color::from_rgb(1.0, 0.0, 0.0),
    );

    renderer.screenshot(
        &Viewport::with_physical_size(Size::new(SIDE, SIDE), 1.0),
        Color::TRANSPARENT,
    )
}

#[test]
fn an_error_wgpu_raises_is_counted_instead_of_panicking() {
    let engine = noop_engine();

    // Mapping a buffer both ways is invalid without a feature we never ask for.
    let _buffer = engine.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::MAP_WRITE,
        mapped_at_creation: false,
    });

    assert_eq!(engine.health().error_count(), 1);
}

#[test]
fn running_out_of_memory_keeps_the_renderer_drawing() {
    let mut renderer = Renderer::new(noop_engine(), Font::DEFAULT, Pixels(16.0));
    let health = Arc::clone(renderer.health());

    let _ = draw_a_frame(&mut renderer);
    health.record(out_of_memory());
    renderer.release_memory();
    let pixels = draw_a_frame(&mut renderer);

    assert_eq!(health.out_of_memory_count(), 1);
    assert_eq!(pixels.len(), (SIDE * SIDE * 4) as usize);
    assert_eq!(health.error_count(), 0, "the frame after should be valid");
    assert!(!health.is_lost());
}

#[test]
fn out_of_memory_never_panics_even_in_tests() {
    let health = Health::default();
    health.panic_on_errors();

    for _ in 0..100 {
        health.record(out_of_memory());
    }

    assert_eq!(health.out_of_memory_count(), 100);
}

#[test]
#[should_panic(expected = "wgpu error")]
fn tests_still_panic_on_validation_errors() {
    let health = Health::default();
    health.panic_on_errors();

    health.record(validation_error());
}

#[test]
fn only_a_real_loss_marks_the_device_lost() {
    let health = Health::default();

    health.record_loss(wgpu::DeviceLostReason::Destroyed, "dropped");
    assert!(!health.is_lost());

    health.record_loss(wgpu::DeviceLostReason::Unknown, "reset");
    assert!(health.is_lost());
}

#[test]
fn dropping_the_device_is_not_a_loss() {
    let engine = noop_engine();
    let health = Arc::clone(engine.health());

    drop(engine);

    assert!(!health.is_lost());
}
