//! Access the clipboard.
use crate::core::Bytes;
use crate::core::clipboard::{Content, Error, Format, Kind};
use crate::futures::futures::channel::oneshot;
use crate::task::{self, Task};

use std::path::PathBuf;
use std::sync::Arc;

/// A clipboard action to be performed by some [`Task`].
///
/// [`Task`]: crate::Task
#[derive(Debug)]
pub enum Action {
    /// Read the clipboard and produce `T` with the result.
    Read {
        /// The [`Kind`] of [`Content`] to read.
        kind: Kind,
        /// The channel to send the read contents.
        channel: oneshot::Sender<Result<Content, Error>>,
    },

    /// Write the given contents to the clipboard.
    Write {
        /// The [`Content`] to be written.
        content: Content,

        /// The channel to send the write result.
        channel: oneshot::Sender<Result<(), Error>>,
    },

    /// Read the primary selection and produce `T` with the result.
    ReadPrimary {
        /// The channel to send the read text.
        channel: oneshot::Sender<Result<String, Error>>,
    },

    /// Publish the given text as the primary selection.
    WritePrimary {
        /// The text to publish.
        text: String,

        /// The channel to send the write result.
        channel: oneshot::Sender<Result<(), Error>>,
    },
}

/// Read the primary selection.
///
/// The primary selection is the X11 and Wayland convention where selecting
/// text makes it available for a middle-click paste, without disturbing the
/// clipboard the user last copied to deliberately. It is text-only, so unlike
/// [`read`] there is no [`Kind`] to choose.
///
/// On platforms without a primary selection this yields
/// [`Error::ContentNotAvailable`].
pub fn read_primary() -> Task<Result<Arc<String>, Error>> {
    task::oneshot(|channel| crate::Action::Clipboard(Action::ReadPrimary { channel }))
        .map(|result| result.map(Arc::new))
}

/// Publish the given text as the primary selection.
///
/// Does not touch the clipboard: the two are separate selections, and
/// copy-on-select must not overwrite what the user last copied.
pub fn write_primary(text: impl Into<String>) -> Task<Result<(), Error>> {
    let text = text.into();

    task::oneshot(|channel| crate::Action::Clipboard(Action::WritePrimary { text, channel }))
}

/// Read the given [`Kind`] of [`Content`] from the clipboard.
pub fn read(kind: Kind) -> Task<Result<Arc<Content>, Error>> {
    task::oneshot(|channel| crate::Action::Clipboard(Action::Read { kind, channel }))
        .map(|result| result.map(Arc::new))
}

/// Read the current text contents of the clipboard.
pub fn read_text() -> Task<Result<Arc<String>, Error>> {
    task::oneshot(|channel| {
        crate::Action::Clipboard(Action::Read {
            kind: Kind::Text,
            channel,
        })
    })
    .map(|result| {
        let Ok(Content::Text(text)) = result else {
            return Err(Error::ContentNotAvailable);
        };

        Ok(Arc::new(text))
    })
}

/// Read the current HTML contents of the clipboard.
pub fn read_html() -> Task<Result<Arc<String>, Error>> {
    task::oneshot(|channel| {
        crate::Action::Clipboard(Action::Read {
            kind: Kind::Html,
            channel,
        })
    })
    .map(|result| {
        let Ok(Content::Html(html)) = result else {
            return Err(Error::ContentNotAvailable);
        };

        Ok(Arc::new(html))
    })
}

/// Read the current file paths of the clipboard.
pub fn read_files() -> Task<Result<Arc<[PathBuf]>, Error>> {
    task::oneshot(|channel| {
        crate::Action::Clipboard(Action::Read {
            kind: Kind::Files,
            channel,
        })
    })
    .map(|result| {
        let Ok(Content::Files(files)) = result else {
            return Err(Error::ContentNotAvailable);
        };

        Ok(Arc::from(files))
    })
}

/// Read the current [`Image`](crate::core::clipboard::Image) of the clipboard.
#[cfg(feature = "image")]
pub fn read_image() -> Task<Result<crate::core::clipboard::Image, Error>> {
    task::oneshot(|channel| {
        crate::Action::Clipboard(Action::Read {
            kind: Kind::Image,
            channel,
        })
    })
    .map(|result| {
        let Ok(Content::Image(image)) = result else {
            return Err(Error::ContentNotAvailable);
        };

        Ok(image)
    })
}

/// Write the given [`Content`] to the clipboard.
pub fn write(content: impl Into<Content>) -> Task<Result<(), Error>> {
    let content = content.into();

    task::oneshot(|channel| crate::Action::Clipboard(Action::Write { content, channel }))
}

/// Write the same content in several [`Format`]s at once.
///
/// Every format is offered under its MIME type, and a reader takes the one it
/// understands: plain text for any app, next to HTML, a PNG or an app's own
/// type. Put [`Format::text`] in whenever there is a text form, since that is
/// what backends without MIME offers keep.
pub fn write_formats(formats: impl IntoIterator<Item = Format>) -> Task<Result<(), Error>> {
    write(Content::Formats(formats.into_iter().collect()))
}

/// Read the clipboard content offered under the given MIME type.
///
/// Yields [`Error::ContentNotAvailable`] when the clipboard offers no such
/// type.
pub fn read_format(mime: &'static str) -> Task<Result<Bytes, Error>> {
    task::oneshot(move |channel| {
        crate::Action::Clipboard(Action::Read {
            kind: Kind::Mime(mime),
            channel,
        })
    })
    .map(move |result| format_data(result?, mime))
}

fn format_data(content: Content, mime: &str) -> Result<Bytes, Error> {
    content
        .format(mime)
        .cloned()
        .ok_or(Error::ContentNotAvailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_format_read_yields_the_bytes_of_its_mime() {
        let content =
            Content::Formats(vec![Format::new("application/x-example", b"42".as_slice())]);

        assert_eq!(
            format_data(content, "application/x-example"),
            Ok(Bytes::from_static(b"42"))
        );
    }

    #[test]
    fn a_format_read_refuses_other_content() {
        let wrong_mime = Content::Formats(vec![Format::html("<p>")]);

        assert_eq!(
            format_data(wrong_mime, "text/plain"),
            Err(Error::ContentNotAvailable)
        );
        assert_eq!(
            format_data(Content::Text("plain".to_owned()), "text/plain"),
            Err(Error::ContentNotAvailable)
        );
    }
}
