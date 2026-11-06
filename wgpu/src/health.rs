//! Keep a device's errors from taking the app down.
//!
//! wgpu panics on any error nobody captured. Apps built with `panic = "abort"`
//! then dump core when the GPU runs out of memory, so every device the
//! [`Engine`](crate::Engine) gets reports here instead.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// The exit status of an app whose GPU device was lost.
pub const DEVICE_LOST_EXIT_CODE: i32 = 75;

/// What went wrong on a device since it was created.
#[derive(Debug, Default)]
pub struct Health {
    out_of_memory: AtomicU64,
    errors: AtomicU64,
    lost: AtomicBool,
    strict: AtomicBool,
}

impl Health {
    /// Takes over `device`'s uncaptured errors and its loss.
    pub fn watch(device: &wgpu::Device) -> Arc<Self> {
        let health = Arc::new(Self::default());

        let errors = Arc::clone(&health);
        device.on_uncaptured_error(Arc::new(move |error| errors.record(error)));

        let lost = Arc::clone(&health);
        device.set_device_lost_callback(move |reason, message| {
            lost.record_loss(reason, &message);
        });

        health
    }

    /// Counts and logs an error no error scope captured, without panicking.
    pub fn record(&self, error: wgpu::Error) {
        match &error {
            wgpu::Error::OutOfMemory { source } => {
                let count = self.out_of_memory.fetch_add(1, Ordering::AcqRel) + 1;

                if worth_logging(count) {
                    log::warn!("GPU out of memory ({count} so far), skipping the frame: {source}");
                }
            }
            wgpu::Error::Validation { .. } | wgpu::Error::Internal { .. } => {
                if self.strict.load(Ordering::Acquire) {
                    panic!("wgpu error: {error}");
                }

                let count = self.errors.fetch_add(1, Ordering::AcqRel) + 1;

                if worth_logging(count) {
                    log::error!("wgpu error ({count} so far): {error}");
                }
            }
        }
    }

    /// Panics on validation and internal errors again, as wgpu does by default.
    ///
    /// Tests want those bugs loud. Running out of memory never panics.
    pub fn panic_on_errors(&self) {
        self.strict.store(true, Ordering::Release);
    }

    /// Records that the device is gone. Dropping it reports it destroyed, which is no loss.
    pub fn record_loss(&self, reason: wgpu::DeviceLostReason, message: &str) {
        if reason == wgpu::DeviceLostReason::Destroyed {
            return;
        }

        log::error!("GPU device lost: {message}");
        self.lost.store(true, Ordering::Release);
    }

    /// How many times the device has run out of memory.
    pub fn out_of_memory_count(&self) -> u64 {
        self.out_of_memory.load(Ordering::Acquire)
    }

    /// How many validation and internal errors the device has reported.
    pub fn error_count(&self) -> u64 {
        self.errors.load(Ordering::Acquire)
    }

    /// Whether the device was lost. Nothing drawn with it reaches the screen again.
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }

    /// Exits when the device was lost, so the session starts the app again.
    ///
    /// A new device would need new renderers and surfaces for every window.
    /// An exit is what a session restarts; an abort leaves a core dump behind.
    pub fn exit_if_lost(&self) {
        if self.is_lost() {
            log::error!(
                "GPU device lost, exiting with status {DEVICE_LOST_EXIT_CODE} so the app can be restarted"
            );
            std::process::exit(DEVICE_LOST_EXIT_CODE);
        }
    }
}

/// An error repeats every frame until its cause goes away: log the first few,
/// then every power of two.
fn worth_logging(count: u64) -> bool {
    count <= 3 || count.is_power_of_two()
}
