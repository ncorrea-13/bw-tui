mod account;
mod auth;
mod generator;
mod item_form;
mod vault;

use crate::app::{App, Screen, Tab, VaultMode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
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
            auth::draw_server_config(frame, url, error.as_deref(), *busy, app.spinner())
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
        } => auth::draw_login(
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
        } => auth::draw_unlock(
            frame,
            email.as_deref(),
            password,
            error.as_deref(),
            *busy,
            relock_message.as_deref(),
            app.spinner(),
        ),
        Screen::Loading => auth::draw_loading(frame, app),
        Screen::Main if app.item_form.as_ref().is_some_and(|f| f.generator_open) => {
            generator::draw_item_form_password_picker(frame, app)
        }
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

fn draw_main(frame: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(5), Constraint::Length(1), Constraint::Length(1)])
        .split(frame.area());

    draw_tab_bar(frame, app, root[0]);

    match app.tab {
        Tab::Vault => vault::draw_vault_tab(frame, app, root[1]),
        Tab::Generator => generator::draw_generator_tab(frame, app, root[1]),
        Tab::Account => account::draw_account_tab(frame, app, root[1]),
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
            "j/k: move  gg/G: top/bottom  h/l: folder  /: search  Enter: view details  n: new item  R: refresh  Tab: switch view  q: quit"
        }
        Tab::Generator => "u/l/n/s: toggle  ↑/↓: length  Enter: generate  c: copy  Tab: switch view  Esc: quit",
        Tab::Account => "s: sync  l: lock  o: log out  Tab: switch view  Esc: quit",
    };
    frame.render_widget(Paragraph::new(help).style(Style::default().fg(MUTED)), area);
}
