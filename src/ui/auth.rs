use super::{boxed, ERROR, MUTED, TEXT, WARN};
use crate::app::{App, LoginField};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::Paragraph,
    Frame,
};

pub(super) fn draw_loading(frame: &mut Frame, app: &App) {
    let inner = boxed(frame, "bw-tui", 40, 5);
    frame.render_widget(
        Paragraph::new(format!("{} Loading…", app.spinner())).style(Style::default().fg(WARN)),
        inner,
    );
}

pub(super) fn draw_server_config(frame: &mut Frame, url: &str, error: Option<&str>, busy: bool, spinner: &str) {
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
            Paragraph::new(format!("\u{f071} {err}")).style(Style::default().fg(ERROR)),
            chunks[2],
        );
    }
    frame.render_widget(
        Paragraph::new("Enter: continue   Esc: quit").style(Style::default().fg(MUTED)),
        chunks[3],
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_login(
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
            frame.render_widget(Paragraph::new(format!("\u{f071} {err}")).style(Style::default().fg(ERROR)), chunks[2]);
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
        frame.render_widget(Paragraph::new(format!("\u{f071} {err}")).style(Style::default().fg(ERROR)), chunks[2]);
    }

    frame.render_widget(
        Paragraph::new("Tab: switch field   Enter: next/submit   Esc: quit")
            .style(Style::default().fg(MUTED)),
        chunks[4],
    );
}

pub(super) fn draw_unlock(
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
        frame.render_widget(Paragraph::new(format!("\u{f071} {err}")).style(Style::default().fg(ERROR)), chunks[2]);
    }

    frame.render_widget(
        Paragraph::new("Enter: unlock   Esc: quit").style(Style::default().fg(MUTED)),
        chunks[4],
    );
}
