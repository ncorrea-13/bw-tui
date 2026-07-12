use crate::app::{App, LoginField, Screen, Tab, VaultMode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

const BG: Color = Color::Rgb(0x12, 0x0d, 0x1e);
const TEXT: Color = Color::Rgb(0xe8, 0xe4, 0xf0);
const MUTED: Color = Color::Rgb(0x71, 0x69, 0x86);
const ACCENT: Color = Color::Rgb(0x93, 0xab, 0xff);
const ACCENT_DIM: Color = Color::Rgb(0x40, 0x3c, 0x5c);
const WARN: Color = Color::Rgb(0xe3, 0xb3, 0x59);
const ERROR: Color = Color::Rgb(0xe3, 0x6f, 0x78);
const OK: Color = Color::Rgb(0x81, 0xcb, 0x9d);

pub fn draw(frame: &mut Frame, app: &App) {
    frame.render_widget(Block::default().style(Style::default().bg(BG).fg(TEXT)), frame.area());
    match &app.screen {
        Screen::ServerConfig { url, error, busy } => draw_server_config(frame, url, error.as_deref(), *busy),
        Screen::Login {
            email,
            password,
            focus,
            awaiting_2fa,
            code,
            method,
            error,
            busy,
        } => draw_login(frame, email, password, *focus, *awaiting_2fa, code, *method, error.as_deref(), *busy),
        Screen::Unlock {
            email,
            password,
            error,
            busy,
            relock_message,
        } => draw_unlock(frame, email.as_deref(), password, error.as_deref(), *busy, relock_message.as_deref()),
        Screen::Main => draw_main(frame, app),
    }
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

fn panel(frame: &mut Frame, area: Rect, title: &str, width: u16, height: u16) -> Rect {
    let area = centered(area, width, height);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)])
        .split(area);
    frame.render_widget(
        Paragraph::new(title).style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(Style::default().fg(ACCENT_DIM)),
        rows[1],
    );
    rows[2]
}

fn boxed(frame: &mut Frame, title: &str, width: u16, height: u16) -> Rect {
    panel(frame, frame.area(), title, width, height)
}

fn draw_server_config(frame: &mut Frame, url: &str, error: Option<&str>, busy: bool) {
    let inner = boxed(frame, "bitwarden-tui — server", 64, 9);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    frame.render_widget(
        Paragraph::new("Server URL (leave empty to use bitwarden.com):"),
        chunks[0],
    );
    frame.render_widget(Paragraph::new(format!("> {url}")), chunks[1]);
    if busy {
        frame.render_widget(
            Paragraph::new("Configuring...").style(Style::default().fg(WARN)),
            chunks[2],
        );
    } else if let Some(err) = error {
        frame.render_widget(
            Paragraph::new(format!("⚠ {err}")).style(Style::default().fg(ERROR)),
            chunks[2],
        );
    }
    frame.render_widget(
        Paragraph::new("Enter: continue   Esc: quit").style(Style::default().fg(MUTED)),
        chunks[3],
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_login(
    frame: &mut Frame,
    email: &str,
    password: &str,
    focus: LoginField,
    awaiting_2fa: bool,
    code: &str,
    method: crate::app::TwoFactorMethod,
    error: Option<&str>,
    busy: bool,
) {
    let inner = boxed(frame, "bitwarden-tui — log in", 64, 11);

    if awaiting_2fa {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)])
            .split(inner);
        frame.render_widget(Paragraph::new(format!("Method: {} (Tab to switch)", method.label())), chunks[0]);
        frame.render_widget(Paragraph::new(format!("Code: {code}")), chunks[1]);
        if busy {
            frame.render_widget(Paragraph::new("Verifying...").style(Style::default().fg(WARN)), chunks[2]);
        } else if let Some(err) = error {
            frame.render_widget(Paragraph::new(format!("⚠ {err}")).style(Style::default().fg(ERROR)), chunks[2]);
        }
        frame.render_widget(
            Paragraph::new("Enter: verify   Esc: quit").style(Style::default().fg(MUTED)),
            chunks[4],
        );
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    let email_style = if focus == LoginField::Email {
        Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(MUTED)
    };
    let pass_style = if focus == LoginField::Password {
        Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(MUTED)
    };

    frame.render_widget(Paragraph::new(format!("Email: {email}")).style(email_style), chunks[0]);
    let masked = "*".repeat(password.chars().count());
    frame.render_widget(Paragraph::new(format!("Password: {masked}")).style(pass_style), chunks[1]);

    if busy {
        frame.render_widget(Paragraph::new("Logging in...").style(Style::default().fg(WARN)), chunks[2]);
    } else if let Some(err) = error {
        frame.render_widget(Paragraph::new(format!("⚠ {err}")).style(Style::default().fg(ERROR)), chunks[2]);
    }

    frame.render_widget(
        Paragraph::new("Tab: switch field   Enter: next/submit   Esc: quit")
            .style(Style::default().fg(MUTED)),
        chunks[4],
    );
}

fn draw_unlock(frame: &mut Frame, email: Option<&str>, password: &str, error: Option<&str>, busy: bool, relock: Option<&str>) {
    let inner = boxed(frame, "bitwarden-tui", 60, 9);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    let prompt = relock
        .map(|s| s.to_string())
        .unwrap_or_else(|| match email {
            Some(e) => format!("Enter the master password for {e}:"),
            None => "Enter your master password:".to_string(),
        });
    frame.render_widget(Paragraph::new(prompt), chunks[0]);

    let masked = "*".repeat(password.chars().count());
    let field_style = if busy { Style::default().fg(MUTED) } else { Style::default().fg(TEXT) };
    frame.render_widget(Paragraph::new(format!("> {masked}")).style(field_style), chunks[1]);

    if busy {
        frame.render_widget(Paragraph::new("Unlocking...").style(Style::default().fg(WARN)), chunks[2]);
    } else if let Some(err) = error {
        frame.render_widget(Paragraph::new(format!("⚠ {err}")).style(Style::default().fg(ERROR)), chunks[2]);
    }

    frame.render_widget(
        Paragraph::new("Enter: unlock   Esc: quit").style(Style::default().fg(MUTED)),
        chunks[4],
    );
}

fn draw_main(frame: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(5), Constraint::Length(1), Constraint::Length(1)])
        .split(frame.area());

    draw_tab_bar(frame, app, root[0]);

    match app.tab {
        Tab::Vault => draw_vault_tab(frame, app, root[1]),
        Tab::Generator => draw_generator_tab(frame, app, root[1]),
        Tab::Account => draw_account_tab(frame, app, root[1]),
    }

    draw_status(frame, app, root[2]);
    draw_help(frame, app, root[3]);
}

fn draw_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let tabs = [("Vault", Tab::Vault), ("Generator", Tab::Generator), ("Account", Tab::Account)];
    let mut spans = vec![];
    for (i, (label, tab)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("    ", Style::default().fg(MUTED)));
        }
        let style = if *tab == app.tab {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(MUTED)
        };
        spans.push(Span::styled(*label, style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Renders a dim vertical rule down the left edge of `area` and returns the
/// remaining inner rect, giving column separation without a boxed border.
fn divider(frame: &mut Frame, area: Rect) -> Rect {
    let block = Block::default().borders(Borders::LEFT).border_style(Style::default().fg(ACCENT_DIM));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

fn section_title(frame: &mut Frame, title: &str, area: Rect) -> Rect {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);
    frame.render_widget(
        Paragraph::new(title).style(Style::default().fg(MUTED).add_modifier(Modifier::BOLD)),
        rows[0],
    );
    rows[1]
}

fn draw_vault_tab(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(18), Constraint::Percentage(37), Constraint::Percentage(45)])
        .split(area);

    draw_folders(frame, app, cols[0]);

    let col1 = divider(frame, cols[1]);
    let vault_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .split(col1);
    draw_search_bar(frame, app, vault_rows[0]);
    draw_list(frame, app, vault_rows[1]);

    let col2 = divider(frame, cols[2]);
    draw_detail(frame, app, col2);
}

fn draw_folders(frame: &mut Frame, app: &App, area: Rect) {
    let inner = section_title(frame, "Folders", area);
    let total = app.folders.len() + 2;
    let items: Vec<ListItem> = (0..total).map(|i| ListItem::new(app.folder_label(i))).collect();
    let list = List::new(items)
        .highlight_style(Style::default().bg(ACCENT).fg(BG).add_modifier(Modifier::BOLD));
    let mut state = ListState::default();
    state.select(Some(app.folder_index));
    frame.render_stateful_widget(list, inner, &mut state);
}

fn draw_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let count = format!("Items  {}/{}", app.filtered.len(), app.items.len());
    frame.render_widget(Paragraph::new(count).style(Style::default().fg(MUTED).add_modifier(Modifier::BOLD)), rows[0]);

    let (text, style) = match app.vault_mode {
        VaultMode::Search => (format!("/{}", app.query), Style::default().fg(ACCENT)),
        VaultMode::Normal if app.query.is_empty() => ("press / to search".to_string(), Style::default().fg(MUTED)),
        VaultMode::Normal => (format!("/{}  (Esc to clear)", app.query), Style::default().fg(TEXT)),
    };
    frame.render_widget(Paragraph::new(text).style(style), rows[1]);
}

fn draw_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .map(|&i| {
            let item = &app.items[i];
            let user = item.username().unwrap_or("-");
            let line = Line::from(vec![
                Span::styled(item.name.clone(), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled(user.to_string(), Style::default().fg(MUTED)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(ACCENT).fg(BG).add_modifier(Modifier::BOLD))
        .highlight_symbol("➤ ");

    let mut state = ListState::default();
    if !app.filtered.is_empty() {
        state.select(Some(app.selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_detail(frame: &mut Frame, app: &App, area: Rect) {
    let inner = section_title(frame, "Detail", area);

    let Some(item) = app.selected_item() else {
        frame.render_widget(Paragraph::new("No results").style(Style::default().fg(MUTED)), inner);
        return;
    };

    let mut lines = vec![
        Line::from(vec![Span::styled("Name: ", Style::default().fg(MUTED)), Span::raw(item.name.clone())]),
        Line::from(vec![Span::styled("Type: ", Style::default().fg(MUTED)), Span::raw(item.type_label())]),
    ];

    match item.item_type {
        3 => {
            if let Some(summary) = item.card_summary() {
                lines.push(Line::from(vec![Span::styled("Card: ", Style::default().fg(MUTED)), Span::raw(summary)]));
            }
        }
        4 => {
            if let Some(summary) = item.identity_summary() {
                lines.push(Line::from(vec![Span::styled("Identity: ", Style::default().fg(MUTED)), Span::raw(summary)]));
            }
        }
        _ => {
            lines.push(Line::from(vec![
                Span::styled("Username: ", Style::default().fg(MUTED)),
                Span::raw(item.username().unwrap_or("-").to_string()),
            ]));
            if let Some(uri) = item.first_uri() {
                lines.push(Line::from(vec![Span::styled("URL: ", Style::default().fg(MUTED)), Span::raw(uri.to_string())]));
            }
            lines.push(Line::from(vec![
                Span::styled("TOTP: ", Style::default().fg(MUTED)),
                Span::raw(if item.has_totp() { "yes (t)" } else { "no" }),
            ]));
            let password_line = match &app.reveal {
                Some((id, pw)) if id == &item.id => format!("Password: {pw}"),
                _ => "Password: •••••••• (press r to reveal)".to_string(),
            };
            lines.push(Line::raw(""));
            lines.push(Line::styled(password_line, Style::default().fg(WARN)));
        }
    }

    let fields = item.visible_fields();
    if !fields.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled("Custom fields:", Style::default().fg(MUTED)));
        for (name, value) in fields {
            lines.push(Line::raw(format!("  {name}: {value}")));
        }
    }

    if let Some(notes) = &item.notes
        && !notes.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::styled("Notes:", Style::default().fg(MUTED)));
            lines.push(Line::raw(notes.clone()));
        }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn draw_generator_tab(frame: &mut Frame, app: &App, area: Rect) {
    let inner = panel(frame, area, "Password generator", 60, 12);

    let opts = &app.generator.opts;
    let toggle = |on: bool| if on { Style::default().fg(OK) } else { Style::default().fg(MUTED) };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    frame.render_widget(Paragraph::new(format!("Length: {} (↑/↓)", opts.length)), chunks[0]);
    frame.render_widget(
        Paragraph::new(format!("[u] Uppercase: {}", if opts.uppercase { "yes" } else { "no" })).style(toggle(opts.uppercase)),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(format!("[l] Lowercase: {}", if opts.lowercase { "yes" } else { "no" })).style(toggle(opts.lowercase)),
        chunks[2],
    );
    frame.render_widget(
        Paragraph::new(format!("[n] Numbers: {}", if opts.numbers { "yes" } else { "no" })).style(toggle(opts.numbers)),
        chunks[3],
    );
    frame.render_widget(
        Paragraph::new(format!("[s] Special: {}", if opts.special { "yes" } else { "no" })).style(toggle(opts.special)),
        chunks[4],
    );

    if let Some(err) = &app.generator.error {
        frame.render_widget(Paragraph::new(format!("⚠ {err}")).style(Style::default().fg(ERROR)), chunks[5]);
    } else if let Some(pw) = &app.generator.result {
        frame.render_widget(
            Paragraph::new(format!("> {pw}  (c: copy)")).style(Style::default().fg(WARN).add_modifier(Modifier::BOLD)),
            chunks[5],
        );
    } else {
        frame.render_widget(
            Paragraph::new("Enter: generate").style(Style::default().fg(MUTED)),
            chunks[5],
        );
    }
}

fn draw_account_tab(frame: &mut Frame, app: &App, area: Rect) {
    let inner = panel(frame, area, "Account", 70, 10);

    let mut lines = vec![];
    if let Some(status) = &app.server_status {
        lines.push(Line::from(vec![
            Span::styled("Server: ", Style::default().fg(MUTED)),
            Span::raw(status.server_url.clone().unwrap_or_else(|| "bitwarden.com".to_string())),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Account: ", Style::default().fg(MUTED)),
            Span::raw(status.user_email.clone().unwrap_or_default()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Last sync: ", Style::default().fg(MUTED)),
            Span::raw(status.last_sync.clone().unwrap_or_else(|| "never".to_string())),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::raw(format!("Cached items: {}", app.items.len())));
    lines.push(Line::raw(""));

    if app.confirm_logout {
        lines.push(Line::styled(
            "Really log out? you'll need to log in again (y/n)",
            Style::default().fg(ERROR).add_modifier(Modifier::BOLD),
        ));
    } else {
        lines.push(Line::raw("[s] Sync now"));
        lines.push(Line::raw("[l] Lock vault"));
        lines.push(Line::raw("[o] Log out"));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let remaining = app.session_remaining();
    let mins = remaining / 60;
    let secs = remaining % 60;
    let session_txt = format!("Session expires in {mins:02}:{secs:02}");

    let text = if let Some(status) = &app.status {
        format!("{}   |   {session_txt}", status.text)
    } else {
        session_txt
    };

    frame.render_widget(Paragraph::new(text).style(Style::default().fg(ACCENT)).alignment(Alignment::Left), area);
}

fn draw_help(frame: &mut Frame, app: &App, area: Rect) {
    let help = match app.tab {
        Tab::Vault if app.vault_mode == VaultMode::Search => "type to filter  Enter: confirm  Esc: cancel",
        Tab::Vault => {
            "j/k: move  gg/G: top/bottom  h/l: folder  /: search  Enter: copy password  u: username  t: TOTP  r: reveal  R: refresh  Tab: switch view  q: quit"
        }
        Tab::Generator => "u/l/n/s: toggle  ↑/↓: length  Enter: generate  c: copy  Tab: switch view  Esc: quit",
        Tab::Account => "s: sync  l: lock  o: log out  Tab: switch view  Esc: quit",
    };
    frame.render_widget(Paragraph::new(help).style(Style::default().fg(MUTED)), area);
}
