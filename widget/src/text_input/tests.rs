use super::*;

fn arrow(
    input: &mut TextInput<'_, (), Theme, ()>,
    tree: &mut Tree,
    named: keyboard::key::Named,
    modifiers: keyboard::Modifiers,
) -> bool {
    let bounds = Rectangle::with_size(Size::new(300.0, 50.0));
    let node = input.layout(
        tree,
        &(),
        &layout::Limits::new(Size::ZERO, bounds.size()),
        None,
    );
    let key = keyboard::Key::Named(named);
    let event = Event::Keyboard(keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    });
    let mut messages = Vec::new();
    input.update(
        tree,
        &Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &(),
        &mut Shell::new(&mut messages),
        &bounds,
    );
    let mut shell = Shell::new(&mut messages);
    input.update(
        tree,
        &event,
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &(),
        &mut shell,
        &bounds,
    );
    shell.is_event_captured()
}

fn focused<Message: Clone>(input: &TextInput<'_, Message, Theme, ()>, at: usize) -> Tree {
    let mut tree = Tree::new(input as &dyn Widget<Message, Theme, ()>);
    let state = tree.state.downcast_mut::<State<()>>();
    state.focus();
    state.cursor.move_to(at);
    tree
}

#[test]
fn unmodified_arrows_leave_only_from_the_corresponding_boundary() {
    let mut input = TextInput::new("", "e\u{301}🎮")
        .on_input(|_| ())
        .navigate_on_boundary(true);
    let mut tree = focused(&input, 0);
    assert!(!arrow(
        &mut input,
        &mut tree,
        keyboard::key::Named::ArrowLeft,
        keyboard::Modifiers::empty()
    ));
    assert!(arrow(
        &mut input,
        &mut tree,
        keyboard::key::Named::ArrowRight,
        keyboard::Modifiers::empty()
    ));
    assert!(arrow(
        &mut input,
        &mut tree,
        keyboard::key::Named::ArrowRight,
        keyboard::Modifiers::empty()
    ));
    assert!(!arrow(
        &mut input,
        &mut tree,
        keyboard::key::Named::ArrowRight,
        keyboard::Modifiers::empty()
    ));
    assert!(arrow(
        &mut input,
        &mut tree,
        keyboard::key::Named::ArrowLeft,
        keyboard::Modifiers::empty()
    ));
}

#[test]
fn the_end_sentinel_and_an_empty_field_are_boundaries() {
    for value in ["", "hello"] {
        let mut input = TextInput::new("", value)
            .on_input(|_| ())
            .navigate_on_boundary(true);
        let mut tree = focused(&input, usize::MAX);
        assert!(!arrow(
            &mut input,
            &mut tree,
            keyboard::key::Named::ArrowRight,
            keyboard::Modifiers::empty()
        ));
        if value.is_empty() {
            assert!(!arrow(
                &mut input,
                &mut tree,
                keyboard::key::Named::ArrowLeft,
                keyboard::Modifiers::empty()
            ));
        }
    }
}

#[test]
fn selection_collapses_before_an_arrow_can_leave_the_field() {
    let mut input = TextInput::new("", "hello")
        .on_input(|_| ())
        .navigate_on_boundary(true);
    let mut tree = focused(&input, 0);
    tree.state
        .downcast_mut::<State<()>>()
        .cursor
        .select_range(0, 5);
    assert!(arrow(
        &mut input,
        &mut tree,
        keyboard::key::Named::ArrowLeft,
        keyboard::Modifiers::empty()
    ));
    assert!(!arrow(
        &mut input,
        &mut tree,
        keyboard::key::Named::ArrowLeft,
        keyboard::Modifiers::empty()
    ));
}

#[test]
fn modified_arrows_remain_editing_shortcuts_at_a_boundary() {
    for modifiers in [
        keyboard::Modifiers::SHIFT,
        keyboard::Modifiers::CTRL,
        keyboard::Modifiers::ALT,
        keyboard::Modifiers::LOGO,
    ] {
        let mut input = TextInput::new("", "hello")
            .on_input(|_| ())
            .navigate_on_boundary(true);
        let mut tree = focused(&input, 0);
        assert!(arrow(
            &mut input,
            &mut tree,
            keyboard::key::Named::ArrowLeft,
            modifiers
        ));
    }
}

#[test]
fn ordinary_text_fields_still_capture_arrows_at_their_boundaries() {
    let mut input = TextInput::new("", "hello").on_input(|_| ());
    let mut tree = focused(&input, 0);
    assert!(arrow(
        &mut input,
        &mut tree,
        keyboard::key::Named::ArrowLeft,
        keyboard::Modifiers::empty()
    ));
    tree.state
        .downcast_mut::<State<()>>()
        .cursor
        .move_to(usize::MAX);
    assert!(arrow(
        &mut input,
        &mut tree,
        keyboard::key::Named::ArrowRight,
        keyboard::Modifiers::empty()
    ));
}

fn input_method(input: &mut TextInput<'_, (), Theme, ()>, tree: &mut Tree) -> InputMethod {
    let bounds = Rectangle::with_size(Size::new(300.0, 50.0));
    let node = input.layout(
        tree,
        &(),
        &layout::Limits::new(Size::ZERO, bounds.size()),
        None,
    );
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    input.update(
        tree,
        &Event::Window(window::Event::RedrawRequested(Instant::now())),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &(),
        &mut shell,
        &bounds,
    );
    shell.input_method().clone()
}

fn surrounding_of(
    input: &mut TextInput<'_, (), Theme, ()>,
    tree: &mut Tree,
) -> Option<input_method::Surrounding> {
    match input_method(input, tree) {
        InputMethod::Enabled { surrounding, .. } => surrounding,
        InputMethod::Disabled => panic!("a focused field enables the input method"),
    }
}

fn around(text: &str, cursor: usize, anchor: usize) -> Option<input_method::Surrounding> {
    Some(input_method::Surrounding {
        text: text.to_owned(),
        cursor,
        anchor,
    })
}

#[test]
fn a_focused_field_tells_the_input_method_the_text_around_its_caret() {
    let mut input = TextInput::new("Search", "Find").on_input(|_| ());
    let mut tree = focused(&input, 4);
    assert_eq!(surrounding_of(&mut input, &mut tree), around("Find", 4, 4));

    let mut empty = TextInput::new("Search", "").on_input(|_| ());
    let mut tree = focused(&empty, 0);
    assert_eq!(surrounding_of(&mut empty, &mut tree), around("", 0, 0));
}

#[test]
fn surrounding_offsets_are_bytes_and_the_anchor_is_where_a_selection_began() {
    // "e" + a combining accent is one grapheme of 3 bytes; the gamepad is 4.
    let mut input = TextInput::new("", "e\u{301}🎮 go").on_input(|_| ());
    let mut tree = focused(&input, 2);
    let text = "e\u{301}🎮 go";
    assert_eq!(surrounding_of(&mut input, &mut tree), around(text, 7, 7));

    // Selected leftwards from the end: the caret is at the gamepad, the anchor at the end.
    tree.state
        .downcast_mut::<State<()>>()
        .cursor
        .select_range(5, 1);
    assert_eq!(surrounding_of(&mut input, &mut tree), around(text, 3, 10));
}

#[test]
fn a_secure_field_never_tells_its_text() {
    let mut input = TextInput::new("Password", "hunter2")
        .on_input(|_| ())
        .secure(true);
    let mut tree = focused(&input, 7);
    match input_method(&mut input, &mut tree) {
        InputMethod::Enabled {
            purpose,
            surrounding,
            ..
        } => {
            assert_eq!(purpose, input_method::Purpose::Secure);
            assert_eq!(surrounding, None);
        }
        InputMethod::Disabled => panic!("a focused field enables the input method"),
    }
}

/// Ctrl+K belongs to the shell's command palette, so a focused field lets it
/// through to the window rather than swallowing it.
#[test]
fn a_focused_field_leaves_ctrl_k_to_the_window() {
    let mut input = TextInput::new("", "hello").on_input(|_| ());
    let mut tree = focused(&input, 2);
    let bounds = Rectangle::with_size(Size::new(300.0, 50.0));
    let node = input.layout(
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, bounds.size()),
        None,
    );
    let modifiers = keyboard::Modifiers::COMMAND;
    let mut messages = Vec::new();
    for (event, captured) in [
        (keyboard::Event::ModifiersChanged(modifiers), false),
        (
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Character("k".into()),
                modified_key: keyboard::Key::Character("k".into()),
                physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyK),
                location: keyboard::Location::Standard,
                modifiers,
                text: Some("\u{b}".into()),
                repeat: false,
            },
            false,
        ),
    ] {
        let mut shell = Shell::new(&mut messages);
        input.update(
            &mut tree,
            &Event::Keyboard(event),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &(),
            &mut shell,
            &bounds,
        );
        assert_eq!(shell.is_event_captured(), captured);
    }
    assert!(messages.is_empty(), "Ctrl+K types nothing");
}

/// What one key event did to a focused field.
#[derive(Debug, Default)]
struct Outcome {
    captured: bool,
    published: Vec<String>,
    copied: Option<String>,
    read_clipboard: bool,
}

fn send(
    input: &mut TextInput<'_, String, Theme, ()>,
    tree: &mut Tree,
    event: keyboard::Event,
) -> Outcome {
    let bounds = Rectangle::with_size(Size::new(300.0, 50.0));
    let node = input.layout(
        tree,
        &(),
        &layout::Limits::new(Size::ZERO, bounds.size()),
        None,
    );
    let mut published = Vec::new();
    let mut shell = Shell::new(&mut published);
    input.update(
        tree,
        &Event::Keyboard(event),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &(),
        &mut shell,
        &bounds,
    );
    let captured = shell.is_event_captured();
    let clipboard = shell.clipboard_mut();
    let copied = match clipboard.write.take() {
        Some(clipboard::Content::Text(text)) => Some(text),
        _ => None,
    };
    let read_clipboard = !clipboard.reads.is_empty();
    Outcome {
        captured,
        published,
        copied,
        read_clipboard,
    }
}

fn letter(c: &str, modifiers: keyboard::Modifiers) -> keyboard::Event {
    let code = match c {
        "a" => keyboard::key::Code::KeyA,
        "b" => keyboard::key::Code::KeyB,
        "c" => keyboard::key::Code::KeyC,
        "v" => keyboard::key::Code::KeyV,
        "x" => keyboard::key::Code::KeyX,
        "y" => keyboard::key::Code::KeyY,
        "z" => keyboard::key::Code::KeyZ,
        _ => unreachable!("no test presses {c}"),
    };
    // What winit reports: Ctrl turns a letter into a control character,
    // Super leaves it a letter.
    let text = if modifiers.control() {
        char::from_u32(u32::from(c.as_bytes()[0] - b'a' + 1)).map(|c| c.to_string().into())
    } else if modifiers.shift() {
        Some(c.to_uppercase().into())
    } else {
        Some(c.into())
    };
    keyboard::Event::KeyPressed {
        key: keyboard::Key::Character(c.into()),
        modified_key: keyboard::Key::Character(c.into()),
        physical_key: keyboard::key::Physical::Code(code),
        location: keyboard::Location::Standard,
        modifiers,
        text,
        repeat: false,
    }
}

fn named(named: keyboard::key::Named, modifiers: keyboard::Modifiers) -> keyboard::Event {
    let key = keyboard::Key::Named(named);
    keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    }
}

fn release(c: &str, modifiers: keyboard::Modifiers) -> keyboard::Event {
    keyboard::Event::KeyReleased {
        key: keyboard::Key::Character(c.into()),
        modified_key: keyboard::Key::Character(c.into()),
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers,
    }
}

/// Holds `modifiers`, as the runtime reports before the key itself.
fn chord(
    input: &mut TextInput<'_, String, Theme, ()>,
    tree: &mut Tree,
    modifiers: keyboard::Modifiers,
    key: keyboard::Event,
) -> Outcome {
    let _ = send(input, tree, keyboard::Event::ModifiersChanged(modifiers));
    let outcome = send(input, tree, key);
    let _ = send(
        input,
        tree,
        keyboard::Event::ModifiersChanged(keyboard::Modifiers::empty()),
    );
    outcome
}

fn field(value: &str) -> TextInput<'static, String, Theme, ()> {
    TextInput::new("", value).on_input(|value| value)
}

fn selection(tree: &mut Tree, input: &TextInput<'_, String, Theme, ()>) -> Option<(usize, usize)> {
    tree.state
        .downcast_mut::<State<()>>()
        .cursor
        .selection(&input.value)
}

const SUPER: keyboard::Modifiers = keyboard::Modifiers::LOGO;
const CTRL: keyboard::Modifiers = keyboard::Modifiers::CTRL;

/// The command key of the platform, and Super off macOS (Kora's editing chords).
fn editing_modifiers() -> Vec<keyboard::Modifiers> {
    if cfg!(target_os = "macos") {
        vec![keyboard::Modifiers::COMMAND]
    } else {
        vec![CTRL, SUPER]
    }
}

#[test]
fn copy_and_cut_answer_to_ctrl_and_super() {
    for modifiers in editing_modifiers() {
        let mut input = field("hello world");
        let mut tree = focused(&input, 0);
        tree.state
            .downcast_mut::<State<()>>()
            .cursor
            .select_range(6, 11);

        let copy = chord(&mut input, &mut tree, modifiers, letter("c", modifiers));
        assert!(copy.captured, "{modifiers:?}+C");
        assert_eq!(copy.copied.as_deref(), Some("world"), "{modifiers:?}+C");
        assert!(copy.published.is_empty(), "{modifiers:?}+C types nothing");

        let cut = chord(&mut input, &mut tree, modifiers, letter("x", modifiers));
        assert_eq!(cut.copied.as_deref(), Some("world"), "{modifiers:?}+X");
        assert_eq!(cut.published, ["hello "], "{modifiers:?}+X");
    }
}

#[test]
fn paste_and_select_all_answer_to_ctrl_and_super() {
    for modifiers in editing_modifiers() {
        let mut input = field("hello");
        let mut tree = focused(&input, 5);

        let paste = chord(&mut input, &mut tree, modifiers, letter("v", modifiers));
        assert!(paste.captured || paste.read_clipboard, "{modifiers:?}+V");
        assert!(paste.read_clipboard, "{modifiers:?}+V reads the clipboard");
        assert!(paste.published.is_empty(), "{modifiers:?}+V types no v");

        let mut input = field("hello");
        let mut tree = focused(&input, 2);
        let all = chord(&mut input, &mut tree, modifiers, letter("a", modifiers));
        assert!(all.captured, "{modifiers:?}+A");
        assert!(all.published.is_empty(), "{modifiers:?}+A types no a");
        assert_eq!(
            selection(&mut tree, &input),
            Some((0, 5)),
            "{modifiers:?}+A"
        );
    }
}

#[test]
fn undo_and_redo_answer_to_ctrl_and_super() {
    for modifiers in editing_modifiers() {
        let mut input = field("hello");
        let mut tree = focused(&input, 5);
        let typed = send(
            &mut input,
            &mut tree,
            letter("b", keyboard::Modifiers::empty()),
        );
        assert_eq!(typed.published, ["hellob"]);

        let undo = chord(&mut input, &mut tree, modifiers, letter("z", modifiers));
        assert_eq!(undo.published, ["hello"], "{modifiers:?}+Z");

        let redo_modifiers = modifiers | keyboard::Modifiers::SHIFT;
        let redo = chord(
            &mut input,
            &mut tree,
            redo_modifiers,
            letter("z", redo_modifiers),
        );
        assert_eq!(redo.published, ["hellob"], "{modifiers:?}+Shift+Z");
    }
}

/// `Ctrl+Y` redoes; `Super+Y` is not one of Kora's editing chords.
#[test]
fn only_ctrl_y_redoes() {
    let mut input = field("hello");
    let mut tree = focused(&input, 5);
    let _ = send(
        &mut input,
        &mut tree,
        letter("b", keyboard::Modifiers::empty()),
    );
    let _ = chord(
        &mut input,
        &mut tree,
        keyboard::Modifiers::COMMAND,
        letter("z", keyboard::Modifiers::COMMAND),
    );
    let redo = chord(
        &mut input,
        &mut tree,
        keyboard::Modifiers::COMMAND,
        letter("y", keyboard::Modifiers::COMMAND),
    );
    assert_eq!(redo.published, ["hellob"]);

    if !cfg!(target_os = "macos") {
        let _ = chord(&mut input, &mut tree, CTRL, letter("z", CTRL));
        let super_y = chord(&mut input, &mut tree, SUPER, letter("y", SUPER));
        assert!(
            super_y.published.is_empty(),
            "Super+Y neither redoes nor types"
        );
        assert!(!super_y.captured);
    }
}

/// `Ctrl+Super` is neither modifier's chord: nothing is copied or typed.
#[cfg(not(target_os = "macos"))]
#[test]
fn ctrl_and_super_together_are_not_an_editing_chord() {
    let both = CTRL | SUPER;
    let mut input = field("hello");
    let mut tree = focused(&input, 0);
    tree.state
        .downcast_mut::<State<()>>()
        .cursor
        .select_range(0, 5);
    for c in ["c", "x", "v", "a", "z"] {
        let outcome = chord(&mut input, &mut tree, both, letter(c, both));
        assert!(!outcome.captured, "Ctrl+Super+{c}");
        assert_eq!(outcome.copied, None, "Ctrl+Super+{c}");
        assert!(!outcome.read_clipboard, "Ctrl+Super+{c}");
        assert!(outcome.published.is_empty(), "Ctrl+Super+{c}");
    }
}

/// A held Super names a shortcut. A key no widget binds under it types
/// nothing and goes on to the window, in every order the keys can come in.
#[cfg(not(target_os = "macos"))]
#[test]
fn a_held_super_types_nothing() {
    let empty = keyboard::Modifiers::empty();
    let mut input = field("hello");
    let mut tree = focused(&input, 5);

    // Super down, B down, B up, Super up; held B repeats while Super is down.
    let _ = send(
        &mut input,
        &mut tree,
        keyboard::Event::ModifiersChanged(SUPER),
    );
    let press = send(&mut input, &mut tree, letter("b", SUPER));
    assert!(!press.captured, "Super+B is the window's");
    assert!(press.published.is_empty(), "Super+B types no b");
    let mut held = letter("b", SUPER);
    if let keyboard::Event::KeyPressed { repeat, .. } = &mut held {
        *repeat = true;
    }
    assert!(send(&mut input, &mut tree, held).published.is_empty());
    assert!(
        send(&mut input, &mut tree, release("b", SUPER))
            .published
            .is_empty()
    );
    let _ = send(
        &mut input,
        &mut tree,
        keyboard::Event::ModifiersChanged(empty),
    );

    // Super down, B down, Super up, B up: the release types nothing either.
    let _ = send(
        &mut input,
        &mut tree,
        keyboard::Event::ModifiersChanged(SUPER),
    );
    assert!(
        send(&mut input, &mut tree, letter("b", SUPER))
            .published
            .is_empty()
    );
    let _ = send(
        &mut input,
        &mut tree,
        keyboard::Event::ModifiersChanged(empty),
    );
    assert!(
        send(&mut input, &mut tree, release("b", empty))
            .published
            .is_empty()
    );

    // B down, Super down, B up, Super up: the B typed before Super counts.
    assert_eq!(
        send(&mut input, &mut tree, letter("b", empty)).published,
        ["hellob"]
    );
    let _ = send(
        &mut input,
        &mut tree,
        keyboard::Event::ModifiersChanged(SUPER),
    );
    assert!(
        send(&mut input, &mut tree, release("b", SUPER))
            .published
            .is_empty()
    );
    let _ = send(
        &mut input,
        &mut tree,
        keyboard::Event::ModifiersChanged(empty),
    );

    // Once Super is up, letters type again.
    assert_eq!(
        send(&mut input, &mut tree, letter("b", empty)).published,
        ["hellobb"]
    );
}

#[test]
fn alt_and_ctrl_arrows_move_and_select_by_words() {
    for jump in [keyboard::Modifiers::ALT, keyboard::Modifiers::CTRL] {
        if cfg!(target_os = "macos") && jump == keyboard::Modifiers::CTRL {
            continue;
        }
        let mut input = field("one two three");
        let mut tree = focused(&input, 13);
        let left = chord(
            &mut input,
            &mut tree,
            jump,
            named(keyboard::key::Named::ArrowLeft, jump),
        );
        assert!(left.captured, "{jump:?}+Left");
        assert_eq!(
            tree.state
                .downcast_mut::<State<()>>()
                .cursor
                .state(&input.value),
            cursor::State::Index(8),
            "{jump:?}+Left jumps a word"
        );

        let select = jump | keyboard::Modifiers::SHIFT;
        let _ = chord(
            &mut input,
            &mut tree,
            select,
            named(keyboard::key::Named::ArrowLeft, select),
        );
        assert_eq!(
            selection(&mut tree, &input),
            Some((4, 8)),
            "{jump:?}+Shift+Left"
        );

        tree.state.downcast_mut::<State<()>>().cursor.move_to(0);
        let right = chord(
            &mut input,
            &mut tree,
            jump,
            named(keyboard::key::Named::ArrowRight, jump),
        );
        assert!(right.captured, "{jump:?}+Right");
        let after_word = tree
            .state
            .downcast_mut::<State<()>>()
            .cursor
            .state(&input.value);
        assert!(
            matches!(after_word, cursor::State::Index(3 | 4)),
            "{jump:?}+Right jumps a word: {after_word:?}"
        );
        let _ = chord(
            &mut input,
            &mut tree,
            select,
            named(keyboard::key::Named::ArrowRight, select),
        );
        assert!(
            selection(&mut tree, &input).is_some(),
            "{jump:?}+Shift+Right selects"
        );
    }
}

#[test]
fn alt_and_ctrl_delete_by_words() {
    for jump in [keyboard::Modifiers::ALT, keyboard::Modifiers::CTRL] {
        if cfg!(target_os = "macos") && jump == keyboard::Modifiers::CTRL {
            continue;
        }
        let mut input = field("one two three");
        let mut tree = focused(&input, 13);
        let back = chord(
            &mut input,
            &mut tree,
            jump,
            named(keyboard::key::Named::Backspace, jump),
        );
        assert_eq!(back.published, ["one two "], "{jump:?}+Backspace");

        let mut input = field("one two three");
        let mut tree = focused(&input, 0);
        let delete = chord(
            &mut input,
            &mut tree,
            jump,
            named(keyboard::key::Named::Delete, jump),
        );
        assert_eq!(delete.published.len(), 1, "{jump:?}+Delete");
        assert!(
            delete.published[0].starts_with(" two") || delete.published[0].starts_with("two"),
            "{jump:?}+Delete removes the first word: {:?}",
            delete.published
        );
    }
}
