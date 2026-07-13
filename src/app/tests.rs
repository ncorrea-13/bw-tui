use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

fn item(name: &str, folder_id: Option<&str>) -> Item {
    Item {
        id: name.to_string(),
        name: name.to_string(),
        item_type: 1,
        login: None,
        card: None,
        identity: None,
        fields: None,
        notes: None,
        folder_id: folder_id.map(str::to_string),
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn vault_app(items: Vec<Item>, folders: Vec<Folder>) -> App {
    let mut app = App::new();
    app.screen = Screen::Main;
    app.tab = Tab::Vault;
    app.items = items;
    app.folders = folders;
    app.refilter();
    app
}

#[test]
fn move_selection_wraps_around() {
    let mut app = vault_app(vec![item("Alpha", None), item("Beta", None), item("Gamma", None)], vec![]);
    assert_eq!(app.selected, 0);
    app.move_selection(-1);
    assert_eq!(app.selected, 2);
    app.move_selection(1);
    assert_eq!(app.selected, 0);
}

#[test]
fn vim_jk_navigate_list() {
    let mut app = vault_app(vec![item("Alpha", None), item("Beta", None), item("Gamma", None)], vec![]);
    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.selected, 1);
    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.selected, 2);
    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.selected, 0, "j should wrap past the last item");
    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(app.selected, 2, "k should wrap before the first item");
}

#[test]
fn vim_gg_and_g_jump_top_bottom() {
    let mut app = vault_app(vec![item("Alpha", None), item("Beta", None), item("Gamma", None)], vec![]);
    app.selected = 1;
    app.handle_key(key(KeyCode::Char('g')));
    assert_eq!(app.selected, 1, "a single 'g' should not jump yet");
    app.handle_key(key(KeyCode::Char('g')));
    assert_eq!(app.selected, 0, "'gg' should jump to the top");
    app.handle_key(key(KeyCode::Char('G')));
    assert_eq!(app.selected, 2, "'G' should jump to the bottom");
}

#[test]
fn vim_g_sequence_breaks_on_other_key() {
    let mut app = vault_app(vec![item("Alpha", None), item("Beta", None), item("Gamma", None)], vec![]);
    app.selected = 1;
    app.handle_key(key(KeyCode::Char('g')));
    app.handle_key(key(KeyCode::Char('j'))); // interrupts the gg sequence
    assert_eq!(app.selected, 2);
    app.handle_key(key(KeyCode::Char('g')));
    assert_eq!(app.selected, 2, "the interrupted 'g' should not have jumped");
}

#[test]
fn slash_enters_search_mode_and_filters() {
    let mut app = vault_app(vec![item("Netflix", None), item("Amazon", None)], vec![]);
    assert_eq!(app.vault_mode, VaultMode::Normal);
    app.handle_key(key(KeyCode::Char('/')));
    assert_eq!(app.vault_mode, VaultMode::Search);
    for c in "net".chars() {
        app.handle_key(key(KeyCode::Char(c)));
    }
    assert_eq!(app.filtered.len(), 1);
    assert_eq!(app.items[app.filtered[0]].name, "Netflix");

    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.vault_mode, VaultMode::Normal, "Enter should confirm and return to Normal mode");
    assert_eq!(app.query, "net", "the query should survive confirming the search");

    app.handle_key(key(KeyCode::Esc));
    assert!(app.query.is_empty(), "Esc in Normal mode should clear an active filter");
    assert_eq!(app.filtered.len(), 2);
}

#[test]
fn search_esc_cancels_and_clears_query() {
    let mut app = vault_app(vec![item("Netflix", None), item("Amazon", None)], vec![]);
    app.handle_key(key(KeyCode::Char('/')));
    app.handle_key(key(KeyCode::Char('n')));
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.vault_mode, VaultMode::Normal);
    assert!(app.query.is_empty());
    assert_eq!(app.filtered.len(), 2);
}

#[test]
fn vim_hl_cycle_folders() {
    let folders = vec![
        Folder { id: Some("f1".into()), name: "Work".into() },
        Folder { id: Some("f2".into()), name: "Personal".into() },
    ];
    let items = vec![item("A", Some("f1")), item("B", Some("f2")), item("C", None)];
    let mut app = vault_app(items, folders);
    assert_eq!(app.folder_index, 0); // All

    app.handle_key(key(KeyCode::Char('l')));
    assert_eq!(app.folder_index, 1); // No folder
    assert_eq!(app.filtered.len(), 1);
    assert_eq!(app.items[app.filtered[0]].name, "C");

    app.handle_key(key(KeyCode::Char('l')));
    assert_eq!(app.folder_index, 2); // Work
    assert_eq!(app.filtered.len(), 1);
    assert_eq!(app.items[app.filtered[0]].name, "A");

    app.handle_key(key(KeyCode::Char('h')));
    assert_eq!(app.folder_index, 1);
}

#[test]
fn q_and_esc_quit_in_normal_mode() {
    let mut app = vault_app(vec![item("Alpha", None)], vec![]);
    app.handle_key(key(KeyCode::Char('q')));
    assert!(app.should_quit);
}

#[test]
fn enter_opens_detail_then_second_enter_closes_it() {
    let mut app = vault_app(vec![item("Netflix", None)], vec![]);
    assert!(!app.detail_open);
    app.handle_key(key(KeyCode::Enter));
    assert!(app.detail_open, "first Enter should open the detail popup");
    app.handle_key(key(KeyCode::Enter));
    assert!(!app.detail_open, "second Enter should copy the password and close the popup");
}

#[test]
fn esc_closes_detail_popup_without_quitting() {
    let mut app = vault_app(vec![item("Netflix", None)], vec![]);
    app.handle_key(key(KeyCode::Enter));
    assert!(app.detail_open);
    app.handle_key(key(KeyCode::Esc));
    assert!(!app.detail_open, "Esc should close the popup");
    assert!(!app.should_quit, "Esc should not quit while the popup is open");
}

#[test]
fn navigation_keys_are_ignored_while_detail_popup_is_open() {
    let mut app = vault_app(vec![item("Alpha", None), item("Beta", None)], vec![]);
    app.handle_key(key(KeyCode::Enter));
    assert!(app.detail_open);
    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.selected, 0, "j should not move the selection while the popup is open");
}

#[test]
fn folder_bar_wraps_instead_of_clipping_at_narrow_width() {
    let folders: Vec<Folder> = (0..12)
        .map(|i| Folder { id: Some(format!("f{i}")), name: format!("Folder{i:02}") })
        .collect();
    let app = vault_app(vec![item("Alpha", None)], folders);

    let backend = TestBackend::new(90, 28);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| crate::ui::draw(frame, &app)).unwrap();

    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();

    for i in 0..12 {
        let label = format!("Folder{i:02}");
        assert!(rendered.contains(&label), "expected to find '{label}' in the rendered frame");
    }
}
