use turborepo_vt100 as vt100;

mod helpers;

// test setting selection
// test copying
// test scrolling with a selection

#[test]
fn visible() {
    let mut parser = vt100::Parser::new(2, 4, 10);
    parser.process(b"foo\r\nbar\r\nbaz");

    // Make sure foo is off the screen
    assert_eq!(parser.screen().contents(), "bar\nbaz");
    parser.screen_mut().set_selection(0, 0, 0, 3);
    assert_eq!(parser.screen().selected_text().as_deref(), Some("bar"));
    parser.screen_mut().clear_selection();
    assert!(parser.screen().selected_text().is_none());
}

#[test]
fn multiline() {
    let mut parser = vt100::Parser::new(2, 4, 10);
    parser.process(b"foo\r\nbar\r\nbaz");

    // Make sure foo is off the screen
    assert_eq!(parser.screen().contents(), "bar\nbaz");
    parser.screen_mut().set_selection(0, 0, 1, 0);
    assert_eq!(parser.screen().selected_text().as_deref(), Some("bar\n"));
}

#[test]
fn scrolling_keeps_selection() {
    let mut parser = vt100::Parser::new(2, 4, 10);
    parser.process(b"foo\r\nbar\r\nbaz");

    assert_eq!(parser.screen().contents(), "bar\nbaz");
    parser.screen_mut().set_selection(0, 0, 0, 3);
    // Scroll so baz is off the screen
    parser.screen_mut().set_scrollback(1);
    // Bar should still be selected
    assert_eq!(parser.screen().selected_text().as_deref(), Some("bar"));
}

#[test]
fn adding_keeps_selection() {
    let mut parser = vt100::Parser::new(2, 4, 10);
    parser.process(b"foo\r\nbar");
    parser.screen_mut().set_selection(1, 0, 1, 3);
    parser.process(b"\r\nbaz");
    // Bar should still be selected
    assert_eq!(parser.screen().selected_text().as_deref(), Some("bar"));
}

#[test]
fn backwards_selection() {
    let mut parser = vt100::Parser::new(2, 4, 10);
    parser.process(b"foo\r\nbar\r\nbaz");

    assert_eq!(parser.screen().contents(), "bar\nbaz");
    parser.screen_mut().set_selection(1, 0, 0, 0);
    // Bar was selected from below
    assert_eq!(parser.screen().selected_text().as_deref(), Some("bar\n"));
}

#[test]
fn too_large() {
    let mut parser = vt100::Parser::new(2, 4, 10);
    parser.process(b"foo\r\nbar\r\nbaz");

    assert_eq!(parser.screen().contents(), "bar\nbaz");
    parser.screen_mut().set_selection(0, 0, 5, 0);
    // Entire screen was selected, but nothing extra
    assert_eq!(
        parser.screen().selected_text().as_deref(),
        Some("bar\nbaz\n")
    );
}
