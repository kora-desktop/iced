//! Access the clipboard.
use crate::dnd;
use std::path::PathBuf;
use std::sync::Arc;

/// A set of clipboard requests.
#[derive(Debug)]
pub struct Clipboard {
    /// The read requests the runtime must fulfill.
    pub reads: Vec<Kind>,
    /// The content that must be written to the clipboard by the runtime,
    /// if any.
    pub write: Option<Content>,
    /// Pending DnD requests from widgets.
    pub dnd_requests: Vec<dnd::Request>,

    /// Whether a widget has asked to read the primary selection.
    ///
    /// A flag rather than a `Vec<Kind>` like `reads`: the primary selection is
    /// text and only text, so there is no format to choose between. Kept
    /// separate from `reads` because [`Event::Read`] carries no `Kind`, so a
    /// primary read arriving on that channel would be indistinguishable from
    /// an ordinary paste — and any focused text input would swallow it.
    pub primary_reads: bool,

    /// Text to publish as the primary selection, if any.
    pub primary_write: Option<String>,
}

impl Clipboard {
    /// Creates a new empty set of [`Clipboard`] requests.
    pub fn new() -> Self {
        Self {
            reads: Vec::new(),
            write: None,
            dnd_requests: Vec::new(),
            primary_reads: false,
            primary_write: None,
        }
    }

    /// Merges the current [`Clipboard`] requests with others.
    pub fn merge(&mut self, other: &mut Self) {
        self.reads.append(&mut other.reads);
        self.write = other.write.take().or(self.write.take());
        self.dnd_requests.append(&mut other.dnd_requests);
        self.primary_reads |= other.primary_reads;
        // Last writer in the frame wins, matching how `write` merges. Copy on
        // select fires on every mouse release, so same-frame collisions are
        // ordinary rather than exceptional.
        self.primary_write = other.primary_write.take().or(self.primary_write.take());
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

/// A clipboard event.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The clipboard was read.
    Read(Result<Arc<Content>, Error>),

    /// The clipboard was written.
    Written(Result<(), Error>),

    /// The primary selection was read.
    ///
    /// Its own variant so a widget can tell a middle-click paste from an
    /// ordinary one; [`Event::Read`] carries no discriminator.
    PrimaryRead(Result<Arc<String>, Error>),

    /// The primary selection was written.
    PrimaryWritten(Result<(), Error>),
}

/// Some clipboard content.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum Content {
    Text(String),
    Html(String),
    #[cfg(feature = "image")]
    Image(Image),
    Files(Vec<PathBuf>),
    /// The same content in several [`Format`]s, offered all at once.
    ///
    /// A reader picks the one it understands, so a copy can carry plain text
    /// for every app next to HTML, a PNG or an app's own type. Reading a
    /// [`Kind::Mime`] yields this variant with the one format read.
    ///
    /// Backends without MIME offers write a fallback instead; see the
    /// platform's clipboard for which.
    Formats(Vec<Format>),
}

impl Content {
    /// The data of the [`Format`] with the given MIME type, if this content
    /// holds one.
    pub fn format(&self, mime: &str) -> Option<&crate::Bytes> {
        let Self::Formats(formats) = self else {
            return None;
        };

        formats
            .iter()
            .find(|format| format.mime == mime)
            .map(|format| &format.data)
    }
}

impl From<Vec<Format>> for Content {
    fn from(formats: Vec<Format>) -> Self {
        Self::Formats(formats)
    }
}

/// One representation of some clipboard content, named by its MIME type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format {
    /// The MIME type, such as `text/plain` or `image/png`.
    pub mime: String,

    /// The bytes, encoded as the MIME type says.
    pub data: crate::Bytes,
}

impl Format {
    /// The MIME type of UTF-8 plain text.
    pub const TEXT: &'static str = "text/plain";
    /// The MIME type of HTML.
    pub const HTML: &'static str = "text/html";
    /// The MIME type of a PNG image.
    pub const PNG: &'static str = "image/png";

    /// Creates a [`Format`] with the given MIME type and data.
    pub fn new(mime: impl Into<String>, data: impl Into<crate::Bytes>) -> Self {
        Self {
            mime: mime.into(),
            data: data.into(),
        }
    }

    /// Plain UTF-8 text.
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(Self::TEXT, text.into())
    }

    /// An HTML fragment.
    pub fn html(html: impl Into<String>) -> Self {
        Self::new(Self::HTML, html.into())
    }

    /// An encoded PNG image.
    pub fn png(png: impl Into<crate::Bytes>) -> Self {
        Self::new(Self::PNG, png)
    }

    /// Whether this is plain text, with or without a charset parameter.
    pub fn is_text(&self) -> bool {
        is_text_mime(&self.mime)
    }

    /// The data as UTF-8, if it is valid UTF-8.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }
}

/// Whether the MIME type names UTF-8 plain text: `text/plain`, with no
/// charset or with `charset=utf-8`.
pub fn is_text_mime(mime: &str) -> bool {
    let mut parts = mime.split(';').map(str::trim);

    if !parts
        .next()
        .is_some_and(|essence| essence.eq_ignore_ascii_case(Format::TEXT))
    {
        return false;
    }

    parts.all(|parameter| {
        let Some((name, value)) = parameter.split_once('=') else {
            return parameter.is_empty();
        };

        !name.trim().eq_ignore_ascii_case("charset")
            || value.trim().trim_matches('"').eq_ignore_ascii_case("utf-8")
    })
}

impl From<String> for Content {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

#[cfg(feature = "image")]
impl From<Image> for Content {
    fn from(image: Image) -> Self {
        Self::Image(image)
    }
}

impl From<Vec<PathBuf>> for Content {
    fn from(files: Vec<PathBuf>) -> Self {
        Self::Files(files)
    }
}

/// The kind of some clipboard [`Content`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum Kind {
    Text,
    Html,
    #[cfg(feature = "image")]
    Image,
    Files,
    /// The content offered under the given MIME type, as
    /// [`Content::Formats`] with that one format.
    Mime(&'static str),
}

/// A clipboard image.
#[cfg(feature = "image")]
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// The pixels of the image in RGBA format.
    pub rgba: crate::Bytes,

    /// The physical [`Size`](crate::Size) of the image.
    pub size: crate::Size<u32>,
}

/// A clipboard error.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// The clipboard in the current environment is either not present or could not be accessed.
    ClipboardUnavailable,

    /// The native clipboard is not accessible due to being held by another party.
    ClipboardOccupied,

    /// The clipboard contents were not available in the requested format.
    /// This could either be due to the clipboard being empty or the clipboard contents having
    /// an incompatible format to the requested one
    ContentNotAvailable,

    /// The image or the text that was about the be transferred to/from the clipboard could not be
    /// converted to the appropriate format.
    ConversionFailure,

    /// Any error that doesn't fit the other error types.
    Unknown {
        /// A description only meant to help the developer that should not be relied on as a
        /// means to identify an error case during runtime.
        description: Arc<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_find_their_data_by_mime() {
        let content = Content::from(vec![
            Format::text("plain"),
            Format::html("<b>rich</b>"),
            Format::new("application/x-example+json", b"{}".as_slice()),
        ]);

        assert_eq!(
            content.format("text/html").map(AsRef::as_ref),
            Some(b"<b>rich</b>".as_slice())
        );
        assert_eq!(
            content
                .format("application/x-example+json")
                .map(AsRef::as_ref),
            Some(b"{}".as_slice())
        );
        assert_eq!(content.format("image/png"), None);
        assert_eq!(Content::Text("plain".to_owned()).format("text/plain"), None);
    }

    #[test]
    fn only_utf8_plain_text_counts_as_text() {
        assert!(is_text_mime("text/plain"));
        assert!(is_text_mime("text/plain;charset=utf-8"));
        assert!(is_text_mime("TEXT/PLAIN; charset=\"UTF-8\""));
        assert!(!is_text_mime("text/plain;charset=utf-16"));
        assert!(!is_text_mime("text/html"));
        assert!(!is_text_mime("text/plainish"));
        assert!(Format::text("x").is_text());
        assert_eq!(Format::text("héllo").as_str(), Some("héllo"));
    }

    #[test]
    fn merging_keeps_a_primary_read_from_either_side() {
        // Reads are a flag, not a queue: two widgets asking in one frame want
        // the same answer, and reading twice would prompt the compositor twice.
        let mut a = Clipboard::new();
        let mut b = Clipboard::new();
        b.primary_reads = true;

        a.merge(&mut b);
        assert!(a.primary_reads);
    }

    #[test]
    fn the_last_primary_write_in_a_frame_wins() {
        // Copy-on-select fires on every mouse release, so two writes in one
        // frame is ordinary. Matching how `write` merges keeps the two
        // channels behaving the same.
        let mut a = Clipboard::new();
        a.primary_write = Some("first".to_owned());
        let mut b = Clipboard::new();
        b.primary_write = Some("second".to_owned());

        a.merge(&mut b);
        assert_eq!(a.primary_write.as_deref(), Some("second"));
    }

    #[test]
    fn merging_an_empty_request_leaves_a_pending_primary_write_alone() {
        let mut a = Clipboard::new();
        a.primary_write = Some("keep me".to_owned());

        a.merge(&mut Clipboard::new());
        assert_eq!(a.primary_write.as_deref(), Some("keep me"));
    }

    #[test]
    fn the_primary_channel_is_independent_of_the_clipboard() {
        // The whole point of the separate channel: copy-on-select must not
        // disturb what the user last copied deliberately.
        let mut a = Clipboard::new();
        let mut b = Clipboard::new();
        b.primary_write = Some("selected".to_owned());

        a.merge(&mut b);
        assert!(a.write.is_none());
        assert!(a.reads.is_empty());
    }
}
