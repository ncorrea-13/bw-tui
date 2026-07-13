use crate::app::{App, ItemForm, ItemFormField, ItemFormMode, LoginField, Screen, Tab, VaultMode};
use crate::bw::Item;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
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
        Screen::ServerConfig { url, error, busy } => {
            draw_server_config(frame, url, error.as_deref(), *busy, app.spinner())
        }
        Screen::Login {
            email,
            password,
            focus,
            awaiting_2fa,
            code,
            method,
            error,
            busy,
        } => draw_login(
            frame,
            email,
            password,
            *focus,
            *awaiting_2fa,
            code,
            *method,
            error.as_deref(),
            *busy,
            app.spinner(),
        ),
        Screen::Unlock {
            email,
            password,
            error,
            busy,
            relock_message,
        } => draw_unlock(
            frame,
            email.as_deref(),
            password,
            error.as_deref(),
            *busy,
            relock_message.as_deref(),
            app.spinner(),
        ),
        Screen::Loading => draw_loading(frame, app),
        Screen::Main if app.item_form.as_ref().is_some_and(|f| f.generator_open) => {
            draw_item_form_password_picker(frame, app)
        }
        Screen::Main => draw_main(frame, app),
    }
}

fn draw_loading(frame: &mut Frame, app: &App) {
    let inner = boxed(frame, "bw-tui", 40, 5);
    frame.render_widget(
        Paragraph::new(format!("{} Loading…", app.spinner())).style(Style::default().fg(WARN)),
        inner,
    );
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

fn draw_server_config(frame: &mut Frame, url: &str, error: Option<&str>, busy: bool, spinner: &str) {
    let inner = boxed(frame, "bw-tui — server", 64, 9);
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
            Paragraph::new(format!("{spinner} Configuring...")).style(Style::default().fg(WARN)),
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
    spinner: &str,
) {
    let inner = boxed(frame, "bw-tui — log in", 64, 11);

    if awaiting_2fa {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)])
            .split(inner);
        frame.render_widget(Paragraph::new(format!("Method: {} (Tab to switch)", method.label())), chunks[0]);
        frame.render_widget(Paragraph::new(format!("Code: {code}")), chunks[1]);
        if busy {
            frame.render_widget(Paragraph::new(format!("{spinner} Verifying...")).style(Style::default().fg(WARN)), chunks[2]);
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
        frame.render_widget(Paragraph::new(format!("{spinner} Logging in...")).style(Style::default().fg(WARN)), chunks[2]);
    } else if let Some(err) = error {
        frame.render_widget(Paragraph::new(format!("⚠ {err}")).style(Style::default().fg(ERROR)), chunks[2]);
    }

    frame.render_widget(
        Paragraph::new("Tab: switch field   Enter: next/submit   Esc: quit")
            .style(Style::default().fg(MUTED)),
        chunks[4],
    );
}

fn draw_unlock(
    frame: &mut Frame,
    email: Option<&str>,
    password: &str,
    error: Option<&str>,
    busy: bool,
    relock: Option<&str>,
    spinner: &str,
) {
    let inner = boxed(frame, "bw-tui", 60, 9);
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
        frame.render_widget(Paragraph::new(format!("{spinner} Unlocking...")).style(Style::default().fg(WARN)), chunks[2]);
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


fn draw_vault_tab(frame: &mut Frame, app: &App, area: Rect) {
    let (folder_area, content_area) = if app.show_folders {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(3)])
            .split(area);
        (Some(rows[0]), rows[1])
    } else {
        (None, area)
    };

    if let Some(folder_area) = folder_area {
        draw_folder_bar(frame, app, folder_area);
    }

    let list_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .split(content_area);
    draw_search_bar(frame, app, list_rows[0]);
    draw_list(frame, app, list_rows[1]);

    if let Some(form) = &app.item_form {
        draw_item_form_popup(frame, form, content_area);
    } else if app.detail_open
        && let Some(item) = app.selected_item() {
            draw_detail_popup(frame, app, item, content_area);
        }
}

fn draw_folder_bar(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(ACCENT_DIM));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let total = app.folders.len() + 2;
    let mut spans = Vec::with_capacity(total * 2);
    for i in 0..total {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let style = if i == app.folder_index {
            Style::default().bg(ACCENT).fg(BG).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(MUTED)
        };
        spans.push(Span::styled(format!(" {} ", app.folder_label(i)), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), inner);
}

fn draw_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let folder_hint = if !app.show_folders && app.folder_index != 0 {
        format!("  ·  {}  (f: folders)", app.folder_label(app.folder_index))
    } else {
        String::new()
    };
    let count = format!("Items  {}/{}{folder_hint}", app.filtered.len(), app.items.len());
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

fn draw_detail_popup(frame: &mut Frame, app: &App, item: &Item, area: Rect) {
    let lines = item_detail_lines(app, item);

    let width = 78u16.min(area.width.saturating_sub(2)).max(20);
    let height = (lines.len() as u16 + 4).clamp(8, area.height.saturating_sub(2).max(8));
    let rect = centered(area, width, height);

    frame.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG))
        .title(Span::styled(format!(" {} ", item.name), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), rows[0]);
    frame.render_widget(Paragraph::new(detail_footer(item)).style(Style::default().fg(MUTED)), rows[1]);
}

fn draw_item_form_popup(frame: &mut Frame, form: &ItemForm, area: Rect) {
    let width = 66u16.min(area.width.saturating_sub(2)).max(20);
    let height = 9u16.clamp(8, area.height.saturating_sub(2).max(8));
    let rect = centered(area, width, height);

    let title = match form.mode {
        ItemFormMode::Create => " New login item ",
        ItemFormMode::Edit { .. } => " Edit login item ",
    };
    frame.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG))
        .title(Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let field_style = |field: ItemFormField| {
        if form.focus == field {
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(MUTED)
        }
    };

    frame.render_widget(
        Paragraph::new(format!("Name: {}", form.name)).style(field_style(ItemFormField::Name)),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(format!("Username: {}", form.username)).style(field_style(ItemFormField::Username)),
        rows[1],
    );
    let masked_password = "*".repeat(form.password.chars().count());
    frame.render_widget(
        Paragraph::new(format!("Password: {masked_password}")).style(field_style(ItemFormField::Password)),
        rows[2],
    );

    if let Some(err) = &form.error {
        frame.render_widget(Paragraph::new(format!("⚠ {err}")).style(Style::default().fg(ERROR)), rows[3]);
    }

    frame.render_widget(
        Paragraph::new("Tab: field  Enter: save  Ctrl+G: random password  Esc: cancel")
            .style(Style::default().fg(MUTED)),
        rows[5],
    );
}

fn mask_card_number(number: &str) -> String {
    let last4: String = number.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
    if last4.is_empty() {
        "•••• •••• •••• ••••".to_string()
    } else {
        format!("•••• •••• •••• {last4}")
    }
}

fn detail_footer(item: &Item) -> String {
    let mut parts: Vec<&str> = match item.item_type {
        1 => vec!["Enter: copy password", "u: username", "t: TOTP", "r: reveal", "e: edit"],
        3 => vec!["Enter: copy number", "r: reveal"],
        2 => vec!["Enter: copy notes"],
        _ => vec![],
    };
    let has_notes = item.notes.as_deref().is_some_and(|n| !n.is_empty());
    if item.item_type != 2 && has_notes {
        parts.push("n: notes");
    }
    parts.push("Esc: close");
    parts.join("  ")
}

fn item_detail_lines(app: &App, item: &Item) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled("Type: ", Style::default().fg(MUTED)),
        Span::raw(item.type_label()),
    ])];

    match item.item_type {
        3 => {
            let card = item.card.as_ref();
            if let Some(holder) = card.and_then(|c| c.cardholder_name.as_deref()) {
                lines.push(Line::from(vec![Span::styled("Cardholder: ", Style::default().fg(MUTED)), Span::raw(holder.to_string())]));
            }
            if let Some(brand) = card.and_then(|c| c.brand.as_deref()) {
                lines.push(Line::from(vec![Span::styled("Brand: ", Style::default().fg(MUTED)), Span::raw(brand.to_string())]));
            }
            if let Some((m, y)) = card.and_then(|c| Some((c.exp_month.as_deref()?, c.exp_year.as_deref()?))) {
                lines.push(Line::from(vec![Span::styled("Expires: ", Style::default().fg(MUTED)), Span::raw(format!("{m}/{y}"))]));
            }
            let revealed = app.reveal.as_ref().is_some_and(|(id, _)| id == &item.id);
            let number_line = match card.and_then(|c| c.number.as_deref()) {
                None => "Number: not on file".to_string(),
                Some(number) => match &app.reveal {
                    Some((id, revealed)) if id == &item.id => format!("Number: {revealed}"),
                    _ => format!("Number: {}  (press r to reveal)", mask_card_number(number)),
                },
            };
            lines.push(Line::raw(""));
            lines.push(Line::styled(number_line, Style::default().fg(WARN)));
            let cvv_line = match card.and_then(|c| c.code.as_deref()) {
                None => "CVV: not on file".to_string(),
                Some(_) if revealed => match &app.reveal_cvv {
                    Some(cvv) => format!("CVV: {cvv}"),
                    None => "CVV: •••".to_string(),
                },
                Some(_) => "CVV: •••  (press r to reveal)".to_string(),
            };
            lines.push(Line::styled(cvv_line, Style::default().fg(WARN)));
        }
        4 => {
            if let Some(summary) = item.identity_summary() {
                lines.push(Line::from(vec![Span::styled("Identity: ", Style::default().fg(MUTED)), Span::raw(summary)]));
            }
        }
        1 => {
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
        _ => {}
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

    lines
}

fn draw_generator_tab(frame: &mut Frame, app: &App, area: Rect) {
    let inner = panel(frame, area, "Password generator", 60, 12);
    draw_generator_options(frame, app, inner, "(c: copy)", None);
}

fn draw_item_form_password_picker(frame: &mut Frame, app: &App) {
    let inner = boxed(frame, "bw-tui — generate password", 60, 13);
    draw_generator_options(
        frame,
        app,
        inner,
        "",
        Some("Enter: use this password   g: regenerate   Esc: cancel"),
    );
}

fn draw_generator_options(
    frame: &mut Frame,
    app: &App,
    inner: Rect,
    result_hint: &str,
    bottom_hint: Option<&str>,
) {
    let opts = &app.generator.opts;
    let toggle = |on: bool| if on { Style::default().fg(OK) } else { Style::default().fg(MUTED) };

    let mut constraints = vec![
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ];
    if bottom_hint.is_some() {
        constraints.push(Constraint::Length(1));
    }
    let chunks = Layout::default().direction(Direction::Vertical).constraints(constraints).split(inner);

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

    if app.busy {
        let label = app.busy_label.as_deref().unwrap_or("Working...");
        frame.render_widget(
            Paragraph::new(format!("{} {label}", app.spinner())).style(Style::default().fg(WARN)),
            chunks[5],
        );
    } else if let Some(err) = &app.generator.error {
        frame.render_widget(Paragraph::new(format!("⚠ {err}")).style(Style::default().fg(ERROR)), chunks[5]);
    } else if let Some(pw) = &app.generator.result {
        let result_line = if result_hint.is_empty() { format!("> {pw}") } else { format!("> {pw}  {result_hint}") };
        frame.render_widget(
            Paragraph::new(result_line).style(Style::default().fg(WARN).add_modifier(Modifier::BOLD)),
            chunks[5],
        );
    } else {
        frame.render_widget(
            Paragraph::new("Enter: generate").style(Style::default().fg(MUTED)),
            chunks[5],
        );
    }

    if let Some(hint) = bottom_hint {
        frame.render_widget(Paragraph::new(hint).style(Style::default().fg(MUTED)), chunks[7]);
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

    let (message, style) = if app.busy {
        let label = app.busy_label.as_deref().unwrap_or("Working...");
        (format!("{} {label}", app.spinner()), Style::default().fg(WARN))
    } else if let Some(status) = &app.status {
        (status.text.clone(), Style::default().fg(ACCENT))
    } else {
        (String::new(), Style::default().fg(ACCENT))
    };

    let text = if message.is_empty() { session_txt } else { format!("{message}   |   {session_txt}") };

    frame.render_widget(Paragraph::new(text).style(style).alignment(Alignment::Left), area);
}

fn draw_help(frame: &mut Frame, app: &App, area: Rect) {
    let help = match app.tab {
        Tab::Vault if app.item_form.is_some() => "Tab: field  Enter: save  Ctrl+G: random password  Esc: cancel",
        Tab::Vault if app.detail_open => "Enter: copy password  u: username  t: TOTP  r: reveal  Esc: close",
        Tab::Vault if app.vault_mode == VaultMode::Search => "type to filter  Enter: confirm  Esc: cancel",
        Tab::Vault => {
            "j/k: move  gg/G: top/bottom  f: folders  h/l: folder  /: search  Enter: view details  n: new item  R: refresh  Tab: switch view  q: quit"
        }
        Tab::Generator => "u/l/n/s: toggle  ↑/↓: length  Enter: generate  c: copy  Tab: switch view  Esc: quit",
        Tab::Account => "s: sync  l: lock  o: log out  Tab: switch view  Esc: quit",
    };
    frame.render_widget(Paragraph::new(help).style(Style::default().fg(MUTED)), area);
}
