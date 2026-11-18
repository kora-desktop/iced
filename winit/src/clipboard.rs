//! Access the clipboard.
use crate::core::clipboard::{Content, Error, Format, Kind};

pub use platform::*;

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use super::*;

    use std::sync::{Arc, Mutex};
    use std::thread;

    /// A buffer for short-term storage and transfer within and between
    /// applications.
    pub struct Clipboard {
        state: State,
    }

    enum State {
        Connected {
            clipboard: Arc<Mutex<arboard::Clipboard>>,
        },
        Unavailable,
    }

    impl Clipboard {
        /// Creates a new [`Clipboard`] for the given window.
        pub fn new() -> Self {
            let clipboard = arboard::Clipboard::new();

            let state = match clipboard {
                Ok(clipboard) => State::Connected {
                    clipboard: Arc::new(Mutex::new(clipboard)),
                },
                Err(_) => State::Unavailable,
            };

            Clipboard { state }
        }

        /// Reads the current content of the [`Clipboard`] as text.
        pub fn read(
            &self,
            kind: Kind,
            callback: impl FnOnce(Result<Content, Error>) + Send + 'static,
        ) {
            let State::Connected { clipboard } = &self.state else {
                callback(Err(Error::ClipboardUnavailable));
                return;
            };

            let clipboard = clipboard.clone();

            let _ = thread::spawn(move || {
                let Ok(mut clipboard) = clipboard.lock() else {
                    callback(Err(Error::ClipboardUnavailable));
                    return;
                };

                if let Kind::Mime(mime) = kind {
                    callback(read_format(&mut clipboard, mime));
                    return;
                }

                let get = clipboard.get();

                let result = match kind {
                    Kind::Text => get.text().map(Content::Text),
                    Kind::Html => get.html().map(Content::Html),
                    #[cfg(feature = "image")]
                    Kind::Image => get.image().map(|image| {
                        let rgba = crate::core::Bytes::from_owner(image.bytes);
                        let size = crate::core::Size {
                            width: image.width as u32,
                            height: image.height as u32,
                        };

                        Content::Image(crate::core::clipboard::Image { rgba, size })
                    }),
                    Kind::Files => get.file_list().map(Content::Files),
                    kind => {
                        log::warn!("unsupported clipboard kind: {kind:?}");

                        Err(arboard::Error::ContentNotAvailable)
                    }
                }
                .map_err(to_error);

                callback(result);
            });
        }

        /// Writes the given text contents to the [`Clipboard`].
        pub fn write(
            &mut self,
            content: Content,
            callback: impl FnOnce(Result<(), Error>) + Send + 'static,
        ) {
            let State::Connected { clipboard } = &self.state else {
                callback(Err(Error::ClipboardUnavailable));
                return;
            };

            let clipboard = clipboard.clone();

            let _ = thread::spawn(move || {
                let Ok(mut clipboard) = clipboard.lock() else {
                    callback(Err(Error::ClipboardUnavailable));
                    return;
                };

                if let Content::Formats(formats) = content {
                    callback(write_formats(&mut clipboard, &formats));
                    return;
                }

                let set = clipboard.set();

                let result = match content {
                    Content::Text(text) => set.text(text),
                    Content::Html(html) => set.html(html, None),
                    #[cfg(feature = "image")]
                    Content::Image(image) => set.image(arboard::ImageData {
                        bytes: image.rgba.as_ref().into(),
                        width: image.size.width as usize,
                        height: image.size.height as usize,
                    }),
                    Content::Files(files) => set.file_list(&files),
                    content => {
                        log::warn!("unsupported clipboard content: {content:?}");

                        Err(arboard::Error::ClipboardNotSupported)
                    }
                }
                .map_err(to_error);

                callback(result);
            });
        }
    }

    impl Clipboard {
        /// Reads the primary selection as text.
        ///
        /// Only X11 and Wayland have a primary selection; elsewhere this
        /// reports the content as unavailable rather than silently returning
        /// the clipboard, which would make a middle-click paste produce
        /// whatever was last copied.
        pub fn read_primary(&self, callback: impl FnOnce(Result<String, Error>) + Send + 'static) {
            let State::Connected { clipboard } = &self.state else {
                callback(Err(Error::ClipboardUnavailable));
                return;
            };

            let clipboard = clipboard.clone();

            let _ = thread::spawn(move || {
                let Ok(mut clipboard) = clipboard.lock() else {
                    callback(Err(Error::ClipboardUnavailable));
                    return;
                };

                #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
                let result = {
                    use arboard::GetExtLinux as _;

                    clipboard
                        .get()
                        .clipboard(arboard::LinuxClipboardKind::Primary)
                        .text()
                        .map_err(to_error)
                };

                #[cfg(not(any(
                    target_os = "linux",
                    target_os = "freebsd",
                    target_os = "openbsd"
                )))]
                let result = {
                    let _ = &mut clipboard;
                    Err(Error::ContentNotAvailable)
                };

                callback(result);
            });
        }

        /// Publishes the given text as the primary selection.
        pub fn write_primary(
            &mut self,
            text: String,
            callback: impl FnOnce(Result<(), Error>) + Send + 'static,
        ) {
            let State::Connected { clipboard } = &self.state else {
                callback(Err(Error::ClipboardUnavailable));
                return;
            };

            let clipboard = clipboard.clone();

            let _ = thread::spawn(move || {
                let Ok(mut clipboard) = clipboard.lock() else {
                    callback(Err(Error::ClipboardUnavailable));
                    return;
                };

                #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
                let result = {
                    use arboard::SetExtLinux as _;

                    clipboard
                        .set()
                        .clipboard(arboard::LinuxClipboardKind::Primary)
                        .text(text)
                        .map_err(to_error)
                };

                #[cfg(not(any(
                    target_os = "linux",
                    target_os = "freebsd",
                    target_os = "openbsd"
                )))]
                let result = {
                    let _ = (&mut clipboard, text);
                    Err(Error::ClipboardUnavailable)
                };

                callback(result);
            });
        }
    }

    /// Offers every format under its MIME type through Wayland data control.
    ///
    /// Elsewhere, and on compositors without data control, only one
    /// representation survives, the first that exists of: HTML (with the
    /// plain text next to it), plain text, then a PNG (with the `image`
    /// feature). Other types are dropped.
    fn write_formats(clipboard: &mut arboard::Clipboard, formats: &[Format]) -> Result<(), Error> {
        #[cfg(all(
            feature = "wayland",
            unix,
            not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
        ))]
        if let Some(result) = super::data_control::write(formats) {
            return result;
        }

        let set = clipboard.set();

        match super::fallback(formats)? {
            super::Fallback::Html { html, alt } => set.html(html, alt),
            super::Fallback::Text(text) => set.text(text),
            #[cfg(feature = "image")]
            super::Fallback::Png(png) => {
                let image = super::decode_png(png)?;

                set.image(arboard::ImageData {
                    width: image.width() as usize,
                    height: image.height() as usize,
                    bytes: image.into_raw().into(),
                })
            }
        }
        .map_err(to_error)
    }

    /// Reads the content offered under `mime` through Wayland data control.
    ///
    /// Elsewhere only plain text, HTML and (with the `image` feature) PNG can
    /// be read, converted from what the platform holds.
    fn read_format(
        clipboard: &mut arboard::Clipboard,
        mime: &'static str,
    ) -> Result<Content, Error> {
        #[cfg(all(
            feature = "wayland",
            unix,
            not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
        ))]
        if let Some(result) = super::data_control::read(mime) {
            return result.map(|data| Content::Formats(vec![Format::new(mime, data)]));
        }

        let get = clipboard.get();

        let data = if crate::core::clipboard::is_text_mime(mime) {
            get.text().map(crate::core::Bytes::from)
        } else if mime == Format::HTML {
            get.html().map(crate::core::Bytes::from)
        } else {
            #[cfg(feature = "image")]
            if mime == Format::PNG {
                let image = get.image().map_err(to_error)?;

                return super::encode_png(image.width, image.height, image.bytes.into_owned())
                    .map(|png| Content::Formats(vec![Format::new(mime, png)]));
            }

            Err(arboard::Error::ContentNotAvailable)
        }
        .map_err(to_error)?;

        Ok(Content::Formats(vec![Format::new(mime, data)]))
    }

    fn to_error(error: arboard::Error) -> Error {
        match error {
            arboard::Error::ContentNotAvailable => Error::ContentNotAvailable,
            arboard::Error::ClipboardNotSupported => Error::ClipboardUnavailable,
            arboard::Error::ClipboardOccupied => Error::ClipboardOccupied,
            arboard::Error::ConversionFailure => Error::ConversionFailure,
            arboard::Error::Unknown { description } => Error::Unknown {
                description: Arc::new(description),
            },
            error => Error::Unknown {
                description: Arc::new(error.to_string()),
            },
        }
    }
}

/// The one representation a clipboard without MIME offers keeps.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, PartialEq)]
enum Fallback {
    Html {
        html: String,
        alt: Option<String>,
    },
    Text(String),
    #[cfg(feature = "image")]
    Png(crate::core::Bytes),
}

#[cfg(not(target_arch = "wasm32"))]
fn fallback(formats: &[Format]) -> Result<Fallback, Error> {
    let find = |mime: &str| formats.iter().find(|format| format.mime == mime);
    let utf8 = |format: &Format| {
        format
            .as_str()
            .map(str::to_owned)
            .ok_or(Error::ConversionFailure)
    };

    let text = formats
        .iter()
        .find(|format| format.is_text())
        .map(utf8)
        .transpose()?;

    if let Some(html) = find(Format::HTML) {
        return Ok(Fallback::Html {
            html: utf8(html)?,
            alt: text,
        });
    }

    if let Some(text) = text {
        return Ok(Fallback::Text(text));
    }

    #[cfg(feature = "image")]
    if let Some(png) = find(Format::PNG) {
        return Ok(Fallback::Png(png.data.clone()));
    }

    Err(Error::ContentNotAvailable)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "image"))]
fn decode_png(png: crate::core::Bytes) -> Result<image::RgbaImage, Error> {
    image::load_from_memory_with_format(&png, image::ImageFormat::Png)
        .map(image::DynamicImage::into_rgba8)
        .map_err(|_| Error::ConversionFailure)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "image"))]
fn encode_png(width: usize, height: usize, rgba: Vec<u8>) -> Result<crate::core::Bytes, Error> {
    let image = image::RgbaImage::from_raw(width as u32, height as u32, rgba)
        .ok_or(Error::ConversionFailure)?;
    let mut png = std::io::Cursor::new(Vec::new());

    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|_| Error::ConversionFailure)?;

    Ok(png.into_inner().into())
}

/// Several MIME types in one write, through the data control protocols.
///
/// Each call returns `None` where data control is out of reach (not a Wayland
/// session, or a compositor without it), so the caller falls back.
#[cfg(all(
    feature = "wayland",
    unix,
    not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
))]
mod data_control {
    use super::{Error, Format};
    use crate::core::Bytes;
    use crate::core::clipboard::is_text_mime;

    use std::io::Read;
    use std::sync::Arc;
    use wl_clipboard_rs::{copy, paste};

    pub fn write(formats: &[Format]) -> Option<Result<(), Error>> {
        let _wayland = std::env::var_os("WAYLAND_DISPLAY")?;

        let mut options = copy::Options::new();
        let _ = options.clipboard(copy::ClipboardType::Regular);

        match options.copy_multi(sources(formats)) {
            Ok(()) => Some(Ok(())),
            Err(
                copy::Error::WaylandConnection(_)
                | copy::Error::MissingProtocol { .. }
                | copy::Error::NoSeats,
            ) => None,
            Err(error) => Some(Err(unknown(error))),
        }
    }

    pub fn read(mime: &str) -> Option<Result<Bytes, Error>> {
        let _wayland = std::env::var_os("WAYLAND_DISPLAY")?;

        let wanted = if is_text_mime(mime) {
            paste::MimeType::Text
        } else {
            paste::MimeType::Specific(mime)
        };

        match paste::get_contents(
            paste::ClipboardType::Regular,
            paste::Seat::Unspecified,
            wanted,
        ) {
            Ok((mut pipe, _)) => {
                let mut data = Vec::new();

                Some(
                    pipe.read_to_end(&mut data)
                        .map(|_| Bytes::from(data))
                        .map_err(unknown),
                )
            }
            Err(paste::Error::ClipboardEmpty | paste::Error::NoMimeType) => {
                Some(Err(Error::ContentNotAvailable))
            }
            Err(
                paste::Error::WaylandConnection(_)
                | paste::Error::MissingProtocol { .. }
                | paste::Error::NoSeats,
            ) => None,
            Err(error) => Some(Err(unknown(error))),
        }
    }

    /// One source per MIME type, the first of each winning. Plain text is
    /// offered under every common text type, so older readers find it.
    pub(super) fn sources(formats: &[Format]) -> Vec<copy::MimeSource> {
        let mut seen: Vec<&str> = Vec::with_capacity(formats.len());
        let mut text = false;

        formats
            .iter()
            .filter_map(|format| {
                let mime_type = if format.is_text() {
                    if std::mem::replace(&mut text, true) {
                        return None;
                    }

                    copy::MimeType::Text
                } else if seen.contains(&format.mime.as_str()) {
                    return None;
                } else {
                    seen.push(&format.mime);

                    copy::MimeType::Specific(format.mime.clone())
                };

                Some(copy::MimeSource {
                    source: copy::Source::Bytes(format.data.to_vec().into_boxed_slice()),
                    mime_type,
                })
            })
            .collect()
    }

    fn unknown(error: impl std::fmt::Display) -> Error {
        Error::Unknown {
            description: Arc::new(error.to_string()),
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    fn custom() -> Format {
        Format::new("application/x-example+json", b"{\"id\":1}".as_slice())
    }

    #[test]
    fn the_fallback_keeps_html_with_its_plain_text() {
        let formats = [custom(), Format::text("plain"), Format::html("<b>rich</b>")];

        assert_eq!(
            fallback(&formats),
            Ok(Fallback::Html {
                html: "<b>rich</b>".to_owned(),
                alt: Some("plain".to_owned()),
            })
        );
    }

    #[test]
    fn the_fallback_keeps_plain_text_over_an_image() {
        let formats = [Format::png(b"not read".as_slice()), Format::text("plain")];

        assert_eq!(fallback(&formats), Ok(Fallback::Text("plain".to_owned())));
    }

    #[test]
    fn the_fallback_refuses_text_that_is_not_utf8() {
        let formats = [Format::new("text/plain", b"\xff\xfe".as_slice())];

        assert_eq!(fallback(&formats), Err(Error::ConversionFailure));
    }

    #[test]
    fn a_custom_type_alone_has_no_fallback() {
        assert_eq!(fallback(&[custom()]), Err(Error::ContentNotAvailable));
    }

    #[cfg(feature = "image")]
    #[test]
    fn the_fallback_keeps_a_png_when_there_is_no_text() {
        let png = encode_png(1, 1, vec![255, 0, 0, 255]).expect("encode");
        let Ok(Fallback::Png(kept)) = fallback(&[custom(), Format::png(png.clone())]) else {
            panic!("expected the PNG");
        };

        assert_eq!(
            decode_png(kept).expect("decode").into_raw(),
            vec![255, 0, 0, 255]
        );
    }

    #[cfg(all(
        feature = "wayland",
        unix,
        not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
    ))]
    #[test]
    fn data_control_offers_each_type_once_and_text_under_every_text_type() {
        use wl_clipboard_rs::copy::MimeType;

        let formats = [
            Format::text("first"),
            Format::html("<p>"),
            Format::new("text/plain;charset=utf-8", "second"),
            Format::html("<p>again"),
            custom(),
        ];
        let sources = data_control::sources(&formats);
        let types: Vec<_> = sources.iter().map(|source| &source.mime_type).collect();

        assert_eq!(
            types,
            [
                &MimeType::Text,
                &MimeType::Specific("text/html".to_owned()),
                &MimeType::Specific("application/x-example+json".to_owned()),
            ]
        );
        assert_eq!(
            sources[0].source,
            wl_clipboard_rs::copy::Source::Bytes(b"first".to_vec().into_boxed_slice())
        );
    }
}

// TODO: Wasm support
#[cfg(target_arch = "wasm32")]
mod platform {
    use super::*;

    /// A buffer for short-term storage and transfer within and between
    /// applications.
    pub struct Clipboard;

    impl Clipboard {
        /// Creates a new [`Clipboard`] for the given window.
        pub fn new() -> Self {
            Self
        }

        /// Reads the current content of the [`Clipboard`] as text.
        pub fn read(&self, _kind: Kind, callback: impl FnOnce(Result<Content, Error>)) {
            callback(Err(Error::ClipboardUnavailable));
        }

        /// Reads the current content of the primary selection as text.
        pub fn read_primary(&self, callback: impl FnOnce(Result<String, Error>)) {
            callback(Err(Error::ClipboardUnavailable));
        }

        /// Writes the given text contents to the primary selection.
        pub fn write_primary(&mut self, _text: String, callback: impl FnOnce(Result<(), Error>)) {
            callback(Err(Error::ClipboardUnavailable));
        }

        /// Writes the given contents to the [`Clipboard`].
        pub fn write(&mut self, _content: Content, callback: impl FnOnce(Result<(), Error>)) {
            callback(Err(Error::ClipboardUnavailable));
        }
    }
}
