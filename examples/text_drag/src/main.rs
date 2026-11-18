//! Drag selected text: inside the editor it moves; out of the window it
//! offers its own MIME type before the text. Run two windows and drop on the
//! other one's zone to see what arrives.
use iced::advanced::dnd::{self, DndAction};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::{Shell, mouse};
use iced::widget::{column, container, text, text_editor};
use iced::{Element, Event, Fill, Length, Point, Rectangle, Size, Theme};

const MIME: &str = "application/x-example-selection+json";

pub fn main() -> iced::Result {
    iced::application(Example::default, Example::update, Example::view).run()
}

struct Example {
    content: text_editor::Content,
    dropped: String,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            content: text_editor::Content::with_text(
                "Select some words, then drag them.\nDrop inside to move them, or on another window.",
            ),
            dropped: String::from("Nothing dropped yet"),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
    Dropped(String, String),
}

impl Example {
    fn update(&mut self, message: Message) {
        match message {
            Message::Edit(action) => self.content.perform(action),
            Message::Dropped(mime, data) => {
                println!("dropped {mime}: {data}");
                self.dropped = format!("{mime}: {data}");
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let editor = text_editor(&self.content)
            .height(200)
            .on_action(Message::Edit)
            .drag_formats(|text| {
                vec![(
                    MIME.to_owned(),
                    format!("{{\"text\":{text:?}}}").into_bytes(),
                )]
            });

        column![
            editor,
            container(Element::new(DropZone {
                label: text(&self.dropped).into()
            }))
            .padding(10)
            .width(Fill)
            .height(Fill)
            .style(container::bordered_box),
        ]
        .spacing(10)
        .padding(10)
        .into()
    }
}

/// Takes a drop of text, the example's MIME type first.
struct DropZone<'a> {
    label: Element<'a, Message>,
}

#[derive(Default)]
struct Offer {
    mime: Option<String>,
    over: bool,
}

impl Widget<Message, Theme, iced::Renderer> for DropZone<'_> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<Offer>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(Offer::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.label)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.label));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Fill, Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = self
            .label
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        layout::Node::with_children(limits.max(), vec![child])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let offer = tree.state.downcast_mut::<Offer>();
        let Event::Dnd(event) = event else {
            return;
        };
        let hover = |offer: &mut Offer, x: f64, y: f64, shell: &mut Shell<'_, Message>| {
            #[allow(clippy::cast_possible_truncation)]
            let over = layout.bounds().contains(Point::new(x as f32, y as f32));
            offer.over = over && offer.mime.is_some();
            shell.accept_dnd_mime_type(offer.over.then(|| offer.mime.clone()).flatten());
            if offer.over {
                shell.set_dnd_actions(DndAction::COPY, DndAction::COPY);
            }
        };
        match event {
            dnd::Event::Enter { x, y, mime_types } => {
                offer.mime = [MIME, "text/plain;charset=utf-8", "text/plain"]
                    .into_iter()
                    .find(|m| mime_types.iter().any(|t| t == m))
                    .map(str::to_owned);
                hover(offer, *x, *y, shell);
            }
            dnd::Event::Motion { x, y } => hover(offer, *x, *y, shell),
            dnd::Event::Leave => *offer = Offer::default(),
            dnd::Event::Drop if offer.over => {
                if let Some(mime) = offer.mime.clone() {
                    shell.request_dnd_data(mime);
                }
            }
            dnd::Event::DataReceived { mime_type, data } if offer.over => {
                shell.publish(Message::Dropped(
                    mime_type.clone(),
                    String::from_utf8_lossy(data).into_owned(),
                ));
                shell.finish_dnd();
                *offer = Offer::default();
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.label.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout.children().next().expect("the label"),
            cursor,
            viewport,
        );
    }
}
