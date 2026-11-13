//! A held key that acts, such as Enter submitting a form, acts once per
//! press, as in a native toolkit. Typing and newlines still repeat.
//!
//! Skipped with the `wgpu` feature: it builds the renderer without a backend,
//! so the simulator can't make one.
#![cfg(not(feature = "wgpu"))]
use iced_test::Simulator;
use iced_widget::core::keyboard::{self, key};
use iced_widget::core::widget::Id;
use iced_widget::core::widget::operation::focusable;
use iced_widget::core::{Element, Event, Point, Theme};
use iced_widget::text_editor::{self, Binding, Content};
use iced_widget::{combo_box, text_input};

type Renderer = iced_renderer::Renderer;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Typed(String),
    Submitted,
    Edited,
    Sent,
    Picked(String),
}

fn pressed(key: keyboard::Key, text: Option<&str>, repeat: bool) -> Event {
    Event::Keyboard(keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: text.map(Into::into),
        repeat,
    })
}

fn released(key: keyboard::Key) -> Event {
    Event::Keyboard(keyboard::Event::KeyReleased {
        key: key.clone(),
        modified_key: key,
        physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
    })
}

/// One press, `repeats` repeats of the held key, then the release.
fn hold(key: keyboard::Key, text: Option<&str>, repeats: usize) -> Vec<Event> {
    let mut events = vec![pressed(key.clone(), text, false)];
    events.extend((0..repeats).map(|_| pressed(key.clone(), text, true)));
    events.push(released(key));
    events
}

fn enter() -> keyboard::Key {
    keyboard::Key::Named(key::Named::Enter)
}

#[test]
fn a_held_enter_submits_once_and_typing_still_repeats() {
    let field: Element<'_, Message, Theme, Renderer> = text_input("Name", "")
        .id("name")
        .on_input(Message::Typed)
        .on_submit(Message::Submitted)
        .into();
    let mut ui = Simulator::new(field);
    ui.operate(&mut focusable::focus(Id::new("name")));

    let _ = ui.simulate(hold(enter(), Some("\r"), 20));
    let _ = ui.simulate(hold(enter(), Some("\r"), 0));
    let _ = ui.simulate(hold(keyboard::Key::Character("a".into()), Some("a"), 2));

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages
            .iter()
            .filter(|m| **m == Message::Submitted)
            .count(),
        2,
        "one submit per press: {messages:?}"
    );
    assert_eq!(
        messages
            .iter()
            .filter(|m| matches!(m, Message::Typed(_)))
            .count(),
        3,
        "a held letter types on every repeat: {messages:?}"
    );
}

#[test]
fn an_editor_binding_sees_repeats_so_send_fires_once_and_newlines_repeat() {
    let content = Content::<Renderer>::new();
    let editor: Element<'_, Message, Theme, Renderer> = text_editor::TextEditor::new(&content)
        .id("composer")
        .on_action(|_| Message::Edited)
        .key_binding(|press| match press.key.as_ref() {
            keyboard::Key::Named(key::Named::Enter) if !press.modifiers.shift() => {
                (!press.repeat).then_some(Binding::Custom(Message::Sent))
            }
            _ => Binding::from_key_press(press),
        })
        .into();
    let mut ui = Simulator::new(editor);
    ui.operate(&mut focusable::focus(Id::new("composer")));

    let _ = ui.simulate(hold(enter(), Some("\r"), 20));
    let _ = ui.simulate(hold(enter(), Some("\r"), 0));

    let mut shift_enter = hold(enter(), Some("\r"), 2);
    for event in &mut shift_enter {
        if let Event::Keyboard(
            keyboard::Event::KeyPressed { modifiers, .. }
            | keyboard::Event::KeyReleased { modifiers, .. },
        ) = event
        {
            *modifiers = keyboard::Modifiers::SHIFT;
        }
    }
    let _ = ui.simulate(shift_enter);

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages.iter().filter(|m| **m == Message::Sent).count(),
        2,
        "one send per press: {messages:?}"
    );
    assert_eq!(
        messages.iter().filter(|m| **m == Message::Edited).count(),
        3,
        "a held Shift+Enter adds a line on every repeat: {messages:?}"
    );
}

#[test]
fn a_held_enter_picks_one_combo_box_option() {
    // The pick unfocuses the field, so its repeats reach nothing.
    let options = combo_box::State::new(vec!["Apple".to_owned(), "Pear".to_owned()]);
    let combo: Element<'_, Message, Theme, Renderer> =
        iced_widget::combo_box(&options, "Fruit", None, Message::Picked).into();
    let mut ui = Simulator::new(combo);
    // The field fills the top of the window; a click there focuses it.
    ui.point_at(Point::new(20.0, 10.0));
    let _ = ui.simulate(iced_test::simulator::click());
    let _ = ui.simulate([pressed(
        keyboard::Key::Named(key::Named::ArrowDown),
        None,
        false,
    )]);

    let _ = ui.simulate(hold(enter(), Some("\r"), 20));

    let picked: Vec<_> = ui
        .into_messages()
        .filter(|m| matches!(m, Message::Picked(_)))
        .collect();
    assert_eq!(picked.len(), 1, "one pick for one press: {picked:?}");
}
