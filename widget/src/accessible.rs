//! Name a widget for assistive technology.
//!
//! [`accessible`] gives the widget it wraps an accessible label, description
//! or state, such as whether it is the current item of its set. Without a
//! [`Role`], it amends the first accessible node its content reports (the
//! button inside, say). With one, it reports a node of its own, with the
//! content beneath it: use that for content that reports no node, such as an
//! icon standing for a status.
//!
//! The wrapper is otherwise invisible: it draws, lays out, handles events and
//! keeps state exactly as the widget it wraps.
use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::widget::operation::accessible::{self as info, Role};
use crate::core::widget::operation::{
    FocusGroup, Focusable, Outcome, Scrollable, Selectable, TextInput,
};
use crate::core::widget::tree::{self, Tree};
use crate::core::widget::{Id, Operation};
use crate::core::{Element, Event, Length, Rectangle, Shell, Size, Vector, Widget};

use std::any::Any;

/// A transparent wrapper that names its content for assistive technology.
#[allow(missing_debug_implementations)]
pub struct Accessible<'a, Message, Theme = crate::Theme, Renderer = crate::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    role: Option<Role>,
    overrides: Overrides,
}

#[derive(Debug, Clone, Default)]
struct Overrides {
    label: Option<String>,
    description: Option<String>,
    selected: Option<bool>,
    toggled: Option<bool>,
    disabled: Option<bool>,
    current: bool,
}

impl Overrides {
    fn apply<'b>(&'b self, mut base: info::Accessible<'b>) -> info::Accessible<'b> {
        if let Some(label) = &self.label {
            base.label = Some(label);
        }
        if let Some(description) = &self.description {
            base.description = Some(description);
        }
        if self.selected.is_some() {
            base.selected = self.selected;
        }
        if self.toggled.is_some() {
            base.toggled = self.toggled;
        }
        if let Some(disabled) = self.disabled {
            base.disabled = disabled;
        }
        base.current |= self.current;
        base
    }
}

impl<'a, Message, Theme, Renderer> Accessible<'a, Message, Theme, Renderer> {
    /// Wraps `content`.
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            content: content.into(),
            role: None,
            overrides: Overrides::default(),
        }
    }

    /// Reports a node of this [`Role`] for the content, instead of amending
    /// the content's own node.
    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    /// Sets the name assistive technology reads for the widget.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.overrides.label = Some(label.into());
        self
    }

    /// Sets a longer description, read after the name.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.overrides.description = Some(description.into());
        self
    }

    /// Marks the widget as the current item of its set (`aria-current`).
    pub fn current(mut self, current: bool) -> Self {
        self.overrides.current = current;
        self
    }

    /// Sets whether the widget is selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.overrides.selected = Some(selected);
        self
    }

    /// Sets whether the widget is toggled on.
    pub fn toggled(mut self, toggled: bool) -> Self {
        self.overrides.toggled = Some(toggled);
        self
    }

    /// Sets whether the widget is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.overrides.disabled = Some(disabled);
        self
    }
}

/// Passes an operation through, amending the first accessible node with
/// the wrapper's overrides.
struct Decorate<'a, 'b> {
    inner: &'a mut dyn Operation,
    overrides: &'b Overrides,
    done: &'a mut bool,
}

impl Operation for Decorate<'_, '_> {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        let overrides = self.overrides;
        let done = &mut *self.done;
        self.inner.traverse(&mut |inner| {
            operate(&mut Decorate {
                inner,
                overrides,
                done: &mut *done,
            });
        });
    }

    fn accessible(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        accessible: &info::Accessible<'_>,
    ) {
        if *self.done {
            self.inner.accessible(id, bounds, accessible);
        } else {
            *self.done = true;
            self.inner
                .accessible(id, bounds, &self.overrides.apply(accessible.clone()));
        }
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        self.inner.container(id, bounds);
    }

    fn focusable(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        self.inner.focusable(id, bounds, state);
    }

    fn auto_focusable(&mut self, id: Option<&Id>, bounds: Rectangle) {
        self.inner.auto_focusable(id, bounds);
    }

    fn focus_order_hint(&mut self, order: u32) {
        self.inner.focus_order_hint(order);
    }

    fn enter_focus_group(&mut self, group: FocusGroup) {
        self.inner.enter_focus_group(group);
    }

    fn exit_focus_group(&mut self) {
        self.inner.exit_focus_group();
    }

    fn action_hintable(&mut self, id: Option<&Id>, bounds: Rectangle, hints: &dyn Any) {
        self.inner.action_hintable(id, bounds, hints);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn Scrollable,
    ) {
        self.inner
            .scrollable(id, bounds, content_bounds, translation, state);
    }

    fn text_input(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn TextInput) {
        self.inner.text_input(id, bounds, state);
    }

    fn text(&mut self, id: Option<&Id>, bounds: Rectangle, text: &str) {
        self.inner.text(id, bounds, text);
    }

    fn selectable_text(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn Selectable) {
        self.inner.selectable_text(id, bounds, state);
    }

    fn custom(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn Any) {
        self.inner.custom(id, bounds, state);
    }

    fn finish(&self) -> Outcome<()> {
        self.inner.finish()
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Accessible<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let bounds = layout.bounds();
        let content = &mut self.content;

        if let Some(role) = self.role {
            operation.accessible(
                None,
                bounds,
                &self.overrides.apply(info::Accessible {
                    role,
                    ..info::Accessible::default()
                }),
            );
            operation.container(None, bounds);
            operation.traverse(&mut |operation| {
                content
                    .as_widget_mut()
                    .operate(tree, layout, renderer, operation);
            });
            return;
        }

        let mut decorated = false;
        let mut decorate = Decorate {
            inner: operation,
            overrides: &self.overrides,
            done: &mut decorated,
        };
        content
            .as_widget_mut()
            .operate(tree, layout, renderer, &mut decorate);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget_mut()
            .update(tree, event, layout, cursor, renderer, shell, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<Accessible<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(accessible: Accessible<'a, Message, Theme, Renderer>) -> Self {
        Self::new(accessible)
    }
}

/// Wraps `content` so it can be named for assistive technology.
///
/// See the [module docs](self).
pub fn accessible<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Accessible<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    Accessible::new(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::core::Point;
    use crate::{Space, button};

    /// Records the accessible nodes an operation is told about, in order.
    #[derive(Default)]
    struct Recorder {
        seen: Vec<(Role, Option<String>, bool, Option<bool>)>,
    }

    impl Operation for Recorder {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }

        fn accessible(
            &mut self,
            _id: Option<&Id>,
            _bounds: Rectangle,
            accessible: &info::Accessible<'_>,
        ) {
            self.seen.push((
                accessible.role,
                accessible.label.map(str::to_owned),
                accessible.current,
                accessible.selected,
            ));
        }
    }

    fn operate(widget: &mut Accessible<'_, (), crate::Theme, ()>) -> Recorder {
        let bounds = Rectangle::new(Point::new(10.0, 20.0), Size::new(48.0, 48.0));
        let mut tree = Tree::new(&*widget as &dyn Widget<(), crate::Theme, ()>);
        let node =
            layout::Node::with_children(bounds.size(), vec![layout::Node::new(bounds.size())])
                .move_to(bounds.position());
        let mut recorder = Recorder::default();

        widget.operate(&mut tree, Layout::new(&node), &(), &mut recorder);

        recorder
    }

    #[test]
    fn names_the_button_it_wraps_and_marks_it_current() {
        let mut tile = accessible(button(Space::new()).on_press(()))
            .label("Files")
            .current(true);

        let recorder = operate(&mut tile);

        assert_eq!(
            recorder.seen,
            vec![(Role::Button, Some("Files".to_owned()), true, None)]
        );
    }

    #[test]
    fn with_a_role_it_reports_a_node_of_its_own() {
        let mut status = accessible(Space::new())
            .role(Role::Image)
            .label("Battery 80%")
            .selected(false);

        let recorder = operate(&mut status);

        assert_eq!(
            recorder.seen,
            vec![(
                Role::Image,
                Some("Battery 80%".to_owned()),
                false,
                Some(false)
            )]
        );
    }

    #[test]
    fn keeps_the_state_of_the_widget_it_wraps() {
        let plain: Element<'_, (), crate::Theme, ()> = button(Space::new()).on_press(()).into();
        let named = accessible(button(Space::new()).on_press(())).label("Files");

        assert_eq!(
            Widget::<(), crate::Theme, ()>::tag(&named),
            plain.as_widget().tag()
        );
    }
}
