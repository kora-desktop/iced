//! A pick list's menu stays inside the window, short of its edges by the
//! margin it is given: it opens toward the side with more room, and a list
//! too long for that room shrinks and scrolls.
//!
//! Skipped with the `wgpu` feature: it builds the renderer without a backend,
//! so the simulator can't make one.
#![cfg(not(feature = "wgpu"))]
use iced_test::Simulator;
use iced_widget::core::keyboard::{self, key::Named};
use iced_widget::core::widget::{Id, Operation, operation};
use iced_widget::core::{Element, Length, Rectangle, Settings, Size, Theme, Vector};
use iced_widget::{button, column, pick_list, space};

type Renderer = iced_renderer::Renderer;

const WINDOW: Size = Size::new(320.0, 400.0);
const MARGIN: f32 = 8.0;

#[derive(Debug, Clone)]
struct Picked;

/// Where the open menu's list is: the one scrollable in these views.
#[derive(Default)]
struct MenuBounds(Option<Rectangle>);

impl Operation for MenuBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        _translation: Vector,
        _state: &mut dyn operation::Scrollable,
    ) {
        self.0 = Some(bounds);
    }
}

/// Opens a long pick list placed at the top or the bottom of the window and
/// returns its menu's bounds.
fn open_menu(at_bottom: bool, margin: Option<f32>) -> Rectangle {
    let options: Vec<String> = (0..48).map(|i| format!("Option {i}")).collect();
    let mut picker = pick_list(Some(&options[20]), options.as_slice(), String::clone)
        .on_select(|_| Picked)
        .width(Length::Fill);
    if let Some(margin) = margin {
        picker = picker.menu_edge_margin(margin);
    }
    let filler = space().height(Length::Fill);
    let view: Element<'_, Picked, Theme, Renderer> = if at_bottom {
        column![filler, picker].into()
    } else {
        column![picker, filler].into()
    };

    let mut ui: Simulator<'_, Picked, Theme, Renderer> =
        Simulator::with_size(Settings::default(), WINDOW, view);
    ui.operate(&mut button::style_focus::<()>(true));
    let _ = ui.tap_key(keyboard::Key::Named(Named::Enter));

    let mut menu = MenuBounds::default();
    ui.operate(&mut operation::black_box(&mut menu));
    menu.0.expect("the menu is open")
}

#[test]
fn a_menu_opening_up_stops_short_of_the_top_edge() {
    let menu = open_menu(true, Some(MARGIN));

    assert_eq!(menu.y, MARGIN, "{menu:?}");
}

#[test]
fn a_menu_opening_down_stops_short_of_the_bottom_edge() {
    let menu = open_menu(false, Some(MARGIN));

    assert!(menu.y > 0.0, "{menu:?} opens below its field");
    assert_eq!(menu.y + menu.height, WINDOW.height - MARGIN, "{menu:?}");
}

#[test]
fn without_a_margin_the_menu_fills_the_room_to_the_edge() {
    let up = open_menu(true, None);
    let down = open_menu(false, None);

    assert_eq!(up.y, 0.0, "{up:?}");
    assert_eq!(down.y + down.height, WINDOW.height, "{down:?}");
}
