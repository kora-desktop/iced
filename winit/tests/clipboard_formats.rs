//! A round trip through a real compositor's clipboard.
//!
//! Ignored by default because it takes the clipboard of whatever Wayland
//! display it finds. Run it only against a private compositor, such as
//! `sway` on the headless backend with its own `XDG_RUNTIME_DIR`, and name
//! that directory twice so the test can tell it was meant:
//!
//! ```sh
//! ICED_CLIPBOARD_TEST_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
//!     cargo test -p iced_winit --test clipboard_formats -- --ignored
//! ```
#![cfg(all(
    feature = "wayland",
    unix,
    not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
))]

use iced_winit::Clipboard;
use iced_winit::core::clipboard::{Content, Error, Format, Kind};

use std::sync::mpsc;
use std::time::Duration;

const SELECTION: &str = "application/x-example-selection+json";

fn private_display() {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR");

    assert!(
        runtime_dir.is_some()
            && runtime_dir == std::env::var_os("ICED_CLIPBOARD_TEST_RUNTIME_DIR")
            && std::env::var_os("WAYLAND_DISPLAY").is_some(),
        "run this only on a private Wayland display; see the module docs"
    );
}

fn read(clipboard: &Clipboard, kind: Kind) -> Result<Content, Error> {
    let (sender, receiver) = mpsc::channel();

    clipboard.read(kind, move |result| {
        let _ = sender.send(result);
    });

    receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("the read answers")
}

#[test]
#[ignore = "takes the clipboard of a Wayland display; run on a private one"]
fn one_write_offers_every_format_and_each_reads_back() {
    private_display();

    let png: &[u8] = b"\x89PNG\r\n\x1a\n not decoded on the way";
    let formats = vec![
        Format::text("Quarterly plan"),
        Format::html("<p><b>Quarterly</b> plan</p>"),
        Format::png(png),
        Format::new(SELECTION, r#"{"version":3}"#),
    ];

    let mut clipboard = Clipboard::new();
    let (sender, receiver) = mpsc::channel();

    clipboard.write(Content::Formats(formats.clone()), move |result| {
        let _ = sender.send(result);
    });

    assert_eq!(
        receiver.recv_timeout(Duration::from_secs(5)),
        Ok(Ok(())),
        "the write is accepted"
    );

    let offered = wl_clipboard_rs::paste::get_mime_types(
        wl_clipboard_rs::paste::ClipboardType::Regular,
        wl_clipboard_rs::paste::Seat::Unspecified,
    )
    .expect("the compositor lists the offer");

    for mime in [
        "text/plain",
        "text/plain;charset=utf-8",
        "text/html",
        "image/png",
        SELECTION,
    ] {
        assert!(offered.contains(mime), "{mime} is offered: {offered:?}");
    }

    for format in &formats {
        let mime: &'static str = Box::leak(format.mime.clone().into_boxed_str());
        let content = read(&clipboard, Kind::Mime(mime)).expect("the format reads back");

        assert_eq!(
            content.format(mime),
            Some(&format.data),
            "{mime} round trips"
        );
    }

    assert_eq!(
        read(&clipboard, Kind::Text),
        Ok(Content::Text("Quarterly plan".to_owned())),
        "an ordinary text paste still gets the plain text"
    );

    assert_eq!(
        read(&clipboard, Kind::Mime("application/x-never-offered")),
        Err(Error::ContentNotAvailable),
        "a type that was not offered is not available"
    );
}
