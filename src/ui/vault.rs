use super::{centered, ACCENT, ACCENT_DIM, BG, MUTED, TEXT, WARN};
use crate::app::{App, VaultMode};
use crate::bw::Item;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub(super) fn draw_vault_tab(frame: &mut Frame, app: &App, area: Rect) {
    let folder_lines = super::wrapped_line_count(folder_bar_char_count(app), area.width).clamp(1, 4);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(folder_lines + 2), Constraint::Min(3)])
        .split(area);
    let (folder_area, content_area) = (rows[0], rows[1]);

    draw_folder_bar(frame, app, folder_area);

    let list_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .split(content_area);
    draw_search_bar(frame, app, list_rows[0]);
    draw_list(frame, app, list_rows[1]);

    if let Some(form) = &app.item_form {
        super::item_form::draw_item_form_popup(frame, form, content_area);
    } else if app.detail_open
        && let Some(item) = app.selected_item() {
            draw_detail_popup(frame, app, item, content_area);
        }
}

fn draw_folder_bar(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(ACCENT_DIM));
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
    frame.render_widget(Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false }), inner);
}

fn folder_bar_char_count(app: &App) -> usize {
    let total = app.folders.len() + 2;
    (0..total)
        .map(|i| app.folder_label(i).chars().count() + 2)
        .sum::<usize>()
        + (total.saturating_sub(1) * 2)
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
            let mut spans = vec![
                Span::styled(item.type_icon(), Style::default().fg(MUTED)),
                Span::raw(" "),
                Span::styled(item.name.clone(), Style::default().add_modifier(Modifier::BOLD)),
            ];
            if let Some(user) = item.username() {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(user.to_string(), Style::default().fg(MUTED)));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(ACCENT).fg(BG).add_modifier(Modifier::BOLD))
        .highlight_symbol("\u{f054} ");

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
        1 => vec!["Enter: copy password", "u: username", "t: TOTP", "r: reveal"],
        3 => vec!["Enter: copy number", "r: reveal"],
        2 => vec!["Enter: copy notes"],
        _ => vec![],
    };
    let has_notes = item.notes.as_deref().is_some_and(|n| !n.is_empty());
    if item.item_type != 2 && has_notes {
        parts.push("n: notes");
    }
    if matches!(item.item_type, 1..=4) {
        parts.push("e: edit");
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
