//! A `responsive` laid out again at the same size keeps what it built, so its widgets draw
//! with what they took from the frame's update. A surface that sizes itself to its content
//! lays the interface out between the two, and a rebuilt `button` drew as disabled there.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use iced_test::runtime::user_interface::{Cache, UserInterface};
use iced_widget::core::layout::{self, Layout};
use iced_widget::core::time::Instant;
use iced_widget::core::widget::{Tree, Widget};
use iced_widget::core::{
    Element, Event, Length, Rectangle, Shell, Size, Theme, mouse, renderer, window,
};
use iced_widget::responsive;

/// Draws whether it saw a redraw, as `button` draws the status it took from one.
struct Probe {
    seen: bool,
    drawn: Rc<RefCell<Vec<bool>>>,
}

impl Widget<(), Theme, ()> for Probe {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &(),
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &(),
        _shell: &mut Shell<'_, ()>,
        _viewport: &Rectangle,
    ) {
        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            self.seen = true;
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        _renderer: &mut (),
        _theme: &Theme,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        self.drawn.borrow_mut().push(self.seen);
    }
}

const SIZE: Size = Size::new(200.0, 100.0);

/// A frame as an auto-sized layer surface draws it: the update, a layout at `measure`, one
/// back at `SIZE`, then the draw. Returns what the probe drew and the sizes it was built for.
fn frame(measure: Size) -> (Vec<bool>, Vec<Size>) {
    let drawn = Rc::new(RefCell::new(Vec::new()));
    let built = Rc::new(RefCell::new(Vec::new()));
    let calls = Rc::new(Cell::new(0));
    let view = {
        let (drawn, built, calls) = (drawn.clone(), built.clone(), calls.clone());
        responsive(move |size| {
            calls.set(calls.get() + 1);
            built.borrow_mut().push(size);
            Element::new(Probe {
                seen: false,
                drawn: drawn.clone(),
            })
        })
    };
    let mut renderer = ();
    let mut ui = UserInterface::build(view, SIZE, Cache::default(), &mut renderer);
    let mut messages = Vec::new();
    let redraw = Event::Window(window::Event::RedrawRequested(Instant::now()));
    let _ = ui.update(
        &[redraw],
        mouse::Cursor::Unavailable,
        &mut renderer,
        &mut messages,
    );
    let mut ui = ui
        .relayout(measure, &mut renderer)
        .relayout(SIZE, &mut renderer);
    ui.draw(
        &mut renderer,
        &Theme::Light,
        &renderer::Style::default(),
        mouse::Cursor::Unavailable,
    );
    let drawn = drawn.borrow().clone();
    let built = built.borrow().clone();
    assert_eq!(calls.get(), built.len());
    (drawn, built)
}

#[test]
fn a_relayout_at_the_same_size_keeps_the_content() {
    let (drawn, built) = frame(SIZE);
    assert_eq!(drawn, [true], "the widget drawn missed the frame's update");
    assert_eq!(built, [SIZE]);
}

#[test]
fn a_relayout_at_another_size_builds_for_it() {
    let tall = Size::new(SIZE.width, 400.0);
    let (drawn, built) = frame(tall);
    assert_eq!(built, [SIZE, tall, SIZE]);
    // Built again for the size, the widget has not seen an update yet.
    assert_eq!(drawn, [false]);
}
