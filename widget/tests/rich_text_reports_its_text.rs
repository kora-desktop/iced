//! Rich text reports its whole text to operations, as plain text does, so a
//! selector, a test driver or a screen reader can find it. Each link it can
//! follow is reported as a link, at the link's place on screen.
//!
//! The link tests need a renderer that measures text, which this crate's tests
//! don't build by default, so they return early without one. Run them with:
//!
//! ```sh
//! ICED_TEST_BACKEND=tiny-skia cargo test -p iced_widget --test rich_text_reports_its_text \
//!     --features iced_renderer/tiny-skia,iced_renderer/wayland,iced_renderer/x11
//! ```
//!
//! Skipped with the `wgpu` feature: it builds the renderer without a backend,
//! so the simulator can't make one.
#![cfg(not(feature = "wgpu"))]
use iced_test::Simulator;
use iced_widget::core::widget::operation::accessible::{Accessible, Role};
use iced_widget::core::widget::{Id, Operation};
use iced_widget::core::{Element, Rectangle, Theme};
use iced_widget::{rich_text, span};

type Renderer = iced_renderer::Renderer;

/// Whether the renderer measures text; the null one this crate builds by default doesn't.
fn measures_text() -> bool {
    std::any::TypeId::of::<Renderer>() != std::any::TypeId::of::<()>()
}

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Link(&'static str),
}

fn terms<'a>(follows_links: bool) -> Element<'a, Message, Theme, Renderer> {
    let text = rich_text![
        span("Read the "),
        span("terms of use").link("terms"),
        span(" first."),
    ];

    if follows_links {
        text.on_link_click(Message::Link).into()
    } else {
        text.into()
    }
}

/// What a rich text reports: its texts and the links among its accessible nodes.
#[derive(Default)]
struct Report {
    texts: Vec<String>,
    links: Vec<(String, Rectangle)>,
}

impl Operation for Report {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn text(&mut self, _id: Option<&Id>, _bounds: Rectangle, text: &str) {
        self.texts.push(text.to_owned());
    }

    fn accessible(&mut self, _id: Option<&Id>, bounds: Rectangle, accessible: &Accessible<'_>) {
        if accessible.role == Role::Link {
            self.links
                .push((accessible.label.unwrap_or_default().to_owned(), bounds));
        }
    }
}

fn report(ui: &mut Simulator<'_, Message, Theme, Renderer>) -> Report {
    let mut report = Report::default();
    ui.operate(&mut report);
    report
}

#[test]
fn rich_text_is_found_by_its_whole_text() {
    let mut ui = Simulator::new(terms(true));

    assert_eq!(report(&mut ui).texts, ["Read the terms of use first."]);
    assert!(ui.find("Read the terms of use first.").is_ok());
}

#[test]
fn a_link_is_reported_where_a_click_follows_it() {
    if !measures_text() {
        return;
    }
    let mut ui = Simulator::new(terms(true));

    let links = report(&mut ui).links;
    let [(label, bounds)] = links.as_slice() else {
        panic!("expected one link, got {links:?}");
    };
    assert_eq!(label, "terms of use");
    assert!(bounds.x > 0.0 && bounds.width > 0.0 && bounds.height > 0.0);

    // An assistive tool follows a link by clicking the middle of its node.
    ui.point_at(bounds.center());
    let _ = ui.simulate(iced_test::simulator::click());

    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        [Message::Link("terms")]
    );
}

#[test]
fn a_link_nothing_follows_is_plain_text() {
    let mut ui = Simulator::new(terms(false));

    let report = report(&mut ui);
    assert_eq!(report.texts, ["Read the terms of use first."]);
    assert!(report.links.is_empty());
}
