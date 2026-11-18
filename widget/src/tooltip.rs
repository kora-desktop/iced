//! Tooltips display a hint of information over some element when hovered.
//!
//! By default, the tooltip is displayed immediately, however, this can be adjusted
//! with [`Tooltip::delay`]. It also shows at once, whatever the delay, while a widget inside
//! it holds the keyboard focus, and Escape hides it until the focus or the pointer moves on.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } }
//! # pub type State = ();
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! use iced::widget::{container, tooltip};
//!
//! enum Message {
//!     // ...
//! }
//!
//! fn view(_state: &State) -> Element<'_, Message> {
//!     tooltip(
//!         "Hover me to display the tooltip!",
//!         container("This is the tooltip contents!")
//!             .padding(10)
//!             .style(container::rounded_box),
//!         tooltip::Position::Bottom,
//!     ).into()
//! }
//! ```
use crate::container;
use crate::core::keyboard;
use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::text;
use crate::core::time::{Duration, Instant};
use crate::core::widget::{self, Widget};
use crate::core::window;
use crate::core::{Element, Event, Length, Padding, Pixels, Point, Rectangle, Shell, Size, Vector};

/// An element to display a widget over another.
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } }
/// # pub type State = ();
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// use iced::widget::{container, tooltip};
///
/// enum Message {
///     // ...
/// }
///
/// fn view(_state: &State) -> Element<'_, Message> {
///     tooltip(
///         "Hover me to display the tooltip!",
///         container("This is the tooltip contents!")
///             .padding(10)
///             .style(container::rounded_box),
///         tooltip::Position::Bottom,
///     ).into()
/// }
/// ```
pub struct Tooltip<'a, Message, Theme = crate::Theme, Renderer = crate::Renderer>
where
    Theme: container::Catalog,
    Renderer: text::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    tooltip: Element<'a, Message, Theme, Renderer>,
    position: Position,
    gap: f32,
    padding: f32,
    snap_within_viewport: bool,
    delay: Duration,
    class: Theme::Class<'a>,
}

impl<'a, Message, Theme, Renderer> Tooltip<'a, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: text::Renderer,
{
    /// The default padding of a [`Tooltip`] drawn by this renderer.
    const DEFAULT_PADDING: f32 = 5.0;

    /// Creates a new [`Tooltip`].
    ///
    /// [`Tooltip`]: struct.Tooltip.html
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        tooltip: impl Into<Element<'a, Message, Theme, Renderer>>,
        position: Position,
    ) -> Self {
        Tooltip {
            content: content.into(),
            tooltip: tooltip.into(),
            position,
            gap: 0.0,
            padding: Self::DEFAULT_PADDING,
            snap_within_viewport: true,
            delay: Duration::ZERO,
            class: Theme::default(),
        }
    }

    /// Sets the gap between the content and its [`Tooltip`].
    pub fn gap(mut self, gap: impl Into<Pixels>) -> Self {
        self.gap = gap.into().0;
        self
    }

    /// Sets the padding of the [`Tooltip`].
    pub fn padding(mut self, padding: impl Into<Pixels>) -> Self {
        self.padding = padding.into().0;
        self
    }

    /// Sets the delay before the [`Tooltip`] is shown.
    ///
    /// Set to [`Duration::ZERO`] to be shown immediately.
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Sets whether the [`Tooltip`] is snapped within the viewport.
    pub fn snap_within_viewport(mut self, snap: bool) -> Self {
        self.snap_within_viewport = snap;
        self
    }

    /// Sets the style of the [`Tooltip`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> container::Style + 'a) -> Self
    where
        Theme::Class<'a>: From<container::StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as container::StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Tooltip`].
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Tooltip<'_, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: text::Renderer,
{
    fn children(&self) -> Vec<widget::Tree> {
        vec![
            widget::Tree::new(&self.content),
            widget::Tree::new(&self.tooltip),
        ]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.content.as_widget(), self.tooltip.as_widget()]);
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let redraw = matches!(event, Event::Window(window::Event::RedrawRequested(_)));

        if matches!(event, Event::Mouse(_)) || redraw {
            let state = tree.state.downcast_mut::<State>();
            let now = Instant::now();
            let cursor_position = cursor.position_over(layout.bounds());

            match (state.hover, cursor_position) {
                (Hover::Idle, Some(cursor_position)) => {
                    // The pointer came back: an Escape before no longer holds.
                    state.dismissed = false;

                    if self.delay == Duration::ZERO {
                        state.hover = Hover::Open { cursor_position };
                        shell.invalidate_layout();
                    } else {
                        state.hover = Hover::Hovered { at: now };
                    }

                    shell.request_redraw_at(now + self.delay);
                }
                (Hover::Hovered { .. }, None) => {
                    state.hover = Hover::Idle;
                }
                (Hover::Hovered { at, .. }, _) if at.elapsed() < self.delay => {
                    shell.request_redraw_at(now + self.delay - at.elapsed());
                }
                (Hover::Hovered { .. }, Some(cursor_position)) => {
                    state.hover = Hover::Open { cursor_position };
                    shell.invalidate_layout();
                }
                (Hover::Open { .. }, None) => {
                    state.hover = Hover::Idle;
                    shell.invalidate_layout();

                    if !redraw {
                        shell.request_redraw();
                    }
                }
                (Hover::Open { .. }, Some(_)) | (Hover::Idle, None) => (),
            }
        }

        let shown = tree
            .state
            .downcast_ref::<State>()
            .shown_at(layout.bounds())
            .is_some();

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            shell,
            viewport,
        );

        // Escape puts the tooltip away without moving the focus. The key goes on to the app.
        if shown
            && let Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) = event
        {
            tree.state.downcast_mut::<State>().dismissed = true;
            shell.invalidate_layout();
            shell.request_redraw();
        }

        // Focus moves by key, by press or by an operation, which redraws.
        if matches!(
            event,
            Event::Keyboard(_)
                | Event::Mouse(mouse::Event::ButtonPressed(_) | mouse::Event::ButtonReleased(_))
                | Event::Touch(_)
        ) || redraw
        {
            let mut holds = HoldsFocus(false);
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                &mut holds,
            );

            let state = tree.state.downcast_mut::<State>();
            if holds.0 != state.focused {
                state.focused = holds.0;
                if holds.0 {
                    state.dismissed = false;
                }
                shell.invalidate_layout();

                if !redraw {
                    shell.request_redraw();
                }
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        inherited_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            inherited_style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_ref::<State>();

        let mut children = tree.children.iter_mut();

        let content = self.content.as_widget_mut().overlay(
            children.next().unwrap(),
            layout,
            renderer,
            viewport,
            translation,
        );

        let tooltip = if let Some(cursor_position) = state.shown_at(layout.bounds()) {
            Some(overlay::Element::new(Box::new(Overlay {
                position: layout.position() + translation,
                tooltip: &mut self.tooltip,
                tree: children.next().unwrap(),
                cursor_position,
                content_bounds: layout.bounds(),
                snap_within_viewport: self.snap_within_viewport,
                positioning: self.position,
                gap: self.gap,
                padding: self.padding,
                class: &self.class,
            })))
        } else {
            None
        };

        if content.is_some() || tooltip.is_some() {
            Some(
                overlay::Group::with_children(content.into_iter().chain(tooltip).collect())
                    .overlay(),
            )
        } else {
            None
        }
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        });
    }
}

impl<'a, Message, Theme, Renderer> From<Tooltip<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: container::Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(
        tooltip: Tooltip<'a, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(tooltip)
    }
}

/// The position of the tooltip. Defaults to following the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    /// The tooltip will appear on the top of the widget.
    #[default]
    Top,
    /// The tooltip will appear on the bottom of the widget.
    Bottom,
    /// The tooltip will appear on the left of the widget.
    Left,
    /// The tooltip will appear on the right of the widget.
    Right,
    /// The tooltip will appear below and to the right of the cursor, offset by
    /// the `gap`, and stay at the position where the hover started.
    FollowCursor,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct State {
    hover: Hover,
    /// A widget inside the content holds the focus.
    focused: bool,
    /// Escape put the tooltip away.
    dismissed: bool,
}

impl State {
    /// Where the tooltip follows the cursor from, while it shows. A focus shows it from the
    /// middle of the content.
    fn shown_at(&self, bounds: Rectangle) -> Option<Point> {
        if self.dismissed {
            return None;
        }

        match self.hover {
            Hover::Open { cursor_position } => Some(cursor_position),
            _ if self.focused => Some(bounds.center()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Hover {
    #[default]
    Idle,
    Hovered {
        at: Instant,
    },
    Open {
        cursor_position: Point,
    },
}

/// Finds whether any focusable it visits holds the focus.
struct HoldsFocus(bool);

impl widget::Operation for HoldsFocus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn widget::operation::Focusable,
    ) {
        self.0 |= state.is_focused();
    }
}

struct Overlay<'a, 'b, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: text::Renderer,
{
    position: Point,
    tooltip: &'b mut Element<'a, Message, Theme, Renderer>,
    tree: &'b mut widget::Tree,
    cursor_position: Point,
    content_bounds: Rectangle,
    snap_within_viewport: bool,
    positioning: Position,
    gap: f32,
    padding: f32,
    class: &'b Theme::Class<'a>,
}

impl<Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for Overlay<'_, '_, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: text::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);

        let tooltip_layout = self.tooltip.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(
                Size::ZERO,
                if self.snap_within_viewport {
                    viewport.size()
                } else {
                    Size::INFINITE
                },
            )
            .shrink(Padding::new(self.padding)),
        );

        let text_bounds = tooltip_layout.bounds();
        let x_center = self.position.x + (self.content_bounds.width - text_bounds.width) / 2.0;
        let y_center = self.position.y + (self.content_bounds.height - text_bounds.height) / 2.0;

        let mut tooltip_bounds = {
            let offset = match self.positioning {
                Position::Top => Vector::new(
                    x_center,
                    self.position.y - text_bounds.height - self.gap - self.padding,
                ),
                Position::Bottom => Vector::new(
                    x_center,
                    self.position.y + self.content_bounds.height + self.gap + self.padding,
                ),
                Position::Left => Vector::new(
                    self.position.x - text_bounds.width - self.gap - self.padding,
                    y_center,
                ),
                Position::Right => Vector::new(
                    self.position.x + self.content_bounds.width + self.gap + self.padding,
                    y_center,
                ),
                Position::FollowCursor => {
                    let translation = self.position - self.content_bounds.position();

                    Vector::new(
                        self.cursor_position.x + self.gap,
                        self.cursor_position.y + self.gap,
                    ) + translation
                }
            };

            Rectangle {
                x: offset.x - self.padding,
                y: offset.y - self.padding,
                width: text_bounds.width + self.padding * 2.0,
                height: text_bounds.height + self.padding * 2.0,
            }
        };

        if self.snap_within_viewport {
            if tooltip_bounds.x < viewport.x {
                tooltip_bounds.x = viewport.x;
            } else if viewport.x + viewport.width < tooltip_bounds.x + tooltip_bounds.width {
                tooltip_bounds.x = viewport.x + viewport.width - tooltip_bounds.width;
            }

            if tooltip_bounds.y < viewport.y {
                tooltip_bounds.y = viewport.y;
            } else if viewport.y + viewport.height < tooltip_bounds.y + tooltip_bounds.height {
                tooltip_bounds.y = viewport.y + viewport.height - tooltip_bounds.height;
            }
        }

        layout::Node::with_children(
            tooltip_bounds.size(),
            vec![tooltip_layout.translate(Vector::new(self.padding, self.padding))],
        )
        .translate(Vector::new(tooltip_bounds.x, tooltip_bounds.y))
    }

    /// Lets operations read what the tooltip shows while it is open: its text
    /// and named containers only. Nothing in it joins focus or scrolling, since
    /// it disappears as soon as the pointer leaves.
    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.tooltip.as_widget_mut().operate(
            self.tree,
            layout.children().next().unwrap(),
            renderer,
            &mut ReadOnly(operation),
        );
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        inherited_style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: mouse::Cursor,
    ) {
        let style = theme.style(self.class);

        container::draw_background(renderer, &style, layout.bounds());

        let defaults = renderer::Style {
            text_color: style.text_color.unwrap_or(inherited_style.text_color),
        };

        self.tooltip.as_widget().draw(
            self.tree,
            renderer,
            theme,
            &defaults,
            layout.children().next().unwrap(),
            cursor_position,
            &Rectangle::with_size(Size::INFINITE),
        );
    }
}

/// Passes on only what a reader of the screen needs: text, named containers,
/// and the walk into children.
struct ReadOnly<'a>(&'a mut dyn widget::Operation);

impl widget::Operation for ReadOnly<'_> {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation)) {
        self.0
            .traverse(&mut |operation| operate(&mut ReadOnly(operation)));
    }

    fn container(&mut self, id: Option<&widget::Id>, bounds: Rectangle) {
        self.0.container(id, bounds);
    }

    fn text(&mut self, id: Option<&widget::Id>, bounds: Rectangle, text: &str) {
        self.0.text(id, bounds, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::core::keyboard::key;
    use crate::{Button, Space, Theme};

    type TestTooltip<'a> = Tooltip<'a, (), Theme, ()>;

    fn tooltip(delay: Duration) -> TestTooltip<'static> {
        Tooltip::new(
            Button::new(Space::new().width(40).height(20)).on_press(()),
            Space::new().width(30).height(10),
            Position::Bottom,
        )
        .delay(delay)
    }

    struct Setup {
        tooltip: TestTooltip<'static>,
        tree: widget::Tree,
        node: layout::Node,
    }

    impl Setup {
        fn new(delay: Duration) -> Self {
            let mut tooltip = tooltip(delay);
            let mut tree = widget::Tree::new(&tooltip as &dyn Widget<(), Theme, ()>);
            let node = tooltip.layout(
                &mut tree,
                &(),
                &layout::Limits::new(Size::ZERO, Size::new(200.0, 100.0)),
            );

            Self {
                tooltip,
                tree,
                node,
            }
        }

        fn send(&mut self, event: Event, cursor: mouse::Cursor) {
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);

            self.tooltip.update(
                &mut self.tree,
                &event,
                Layout::new(&self.node),
                cursor,
                &(),
                &mut shell,
                &Rectangle::with_size(Size::new(200.0, 100.0)),
            );
        }

        fn redraw(&mut self) {
            self.send(
                Event::Window(window::Event::RedrawRequested(Instant::now())),
                mouse::Cursor::Unavailable,
            );
        }

        /// Focuses or unfocuses the button, as a keyboard's focus operation does, then redraws
        /// as the runtime does after one.
        fn focus(&mut self, focused: bool) {
            struct Focus(bool);

            impl widget::Operation for Focus {
                fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation)) {
                    operate(self);
                }

                fn focusable(
                    &mut self,
                    _id: Option<&widget::Id>,
                    _bounds: Rectangle,
                    state: &mut dyn widget::operation::Focusable,
                ) {
                    if self.0 {
                        state.focus();
                    } else {
                        state.unfocus();
                    }
                }
            }

            self.tooltip.operate(
                &mut self.tree,
                Layout::new(&self.node),
                &(),
                &mut Focus(focused),
            );
            self.redraw();
        }

        fn escape(&mut self) {
            self.send(
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(key::Named::Escape),
                    modified_key: keyboard::Key::Named(key::Named::Escape),
                    physical_key: key::Physical::Code(key::Code::Escape),
                    location: keyboard::Location::Standard,
                    modifiers: keyboard::Modifiers::empty(),
                    text: None,
                    repeat: false,
                }),
                mouse::Cursor::Unavailable,
            );
        }

        fn shown(&mut self) -> bool {
            self.tooltip
                .overlay(
                    &mut self.tree,
                    Layout::new(&self.node),
                    &(),
                    &Rectangle::with_size(Size::new(200.0, 100.0)),
                    Vector::ZERO,
                )
                .is_some()
        }
    }

    #[test]
    fn focus_shows_the_tooltip_at_once_and_losing_it_hides_it() {
        let mut setup = Setup::new(Duration::from_secs(1));
        setup.redraw();
        assert!(!setup.shown());

        setup.focus(true);
        assert!(setup.shown(), "a focus shows it without the hover delay");

        setup.focus(false);
        assert!(!setup.shown(), "the focus moved on");
    }

    #[test]
    fn escape_hides_the_tooltip_until_the_focus_comes_back() {
        let mut setup = Setup::new(Duration::ZERO);
        setup.focus(true);
        assert!(setup.shown());

        setup.escape();
        assert!(!setup.shown(), "Escape puts it away");
        setup.redraw();
        assert!(!setup.shown(), "and it stays away while the focus stays");

        setup.focus(false);
        setup.focus(true);
        assert!(setup.shown(), "a new focus shows it again");
    }

    #[test]
    fn escape_hides_a_hover_tooltip_until_the_pointer_comes_back() {
        let mut setup = Setup::new(Duration::ZERO);
        let over = mouse::Cursor::Available(Point::new(10.0, 10.0));
        let moved = |position| Event::Mouse(mouse::Event::CursorMoved { position });

        setup.send(moved(Point::new(10.0, 10.0)), over);
        assert!(setup.shown());

        setup.escape();
        setup.send(moved(Point::new(12.0, 10.0)), over);
        assert!(!setup.shown(), "moving inside does not bring it back");

        setup.send(moved(Point::new(150.0, 80.0)), mouse::Cursor::Unavailable);
        setup.send(moved(Point::new(10.0, 10.0)), over);
        assert!(setup.shown(), "coming back does");
    }
}
