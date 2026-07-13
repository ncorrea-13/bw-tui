use super::{panel, ERROR, MUTED};
use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

pub(super) fn draw_account_tab(frame: &mut Frame, app: &App, area: Rect) {
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
