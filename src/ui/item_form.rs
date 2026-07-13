use super::{centered, ACCENT, BG, ERROR, MUTED, TEXT};
use crate::app::{ItemForm, ItemFormField, ItemFormMode, ItemKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub(super) fn draw_item_form_popup(frame: &mut Frame, form: &ItemForm, area: Rect) {
    let visible_fields = form.kind.fields();

    let width = 76u16.min(area.width.saturating_sub(2)).max(20);
    let height = (visible_fields.len() as u16 + 6).clamp(8, area.height.saturating_sub(2).max(8));
    let rect = centered(area, width, height);

    let mode_label = match form.mode {
        ItemFormMode::Create => "New",
        ItemFormMode::Edit { .. } => "Edit",
    };
    let title = format!(" {mode_label} {} item ", form.kind.label());
    frame.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG))
        .title(Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let mut constraints = vec![Constraint::Length(1); visible_fields.len()];
    constraints.push(Constraint::Length(1));
    constraints.push(Constraint::Min(1));
    constraints.push(Constraint::Length(1));
    constraints.push(Constraint::Length(1));
    let rows = Layout::default().direction(Direction::Vertical).constraints(constraints).split(inner);

    for (i, &field) in visible_fields.iter().enumerate() {
        let style = if form.focus == field {
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(MUTED)
        };
        let text = if field == ItemFormField::Password {
            let shown = if form.password_revealed {
                form.password.clone()
            } else {
                "*".repeat(form.password.chars().count())
            };
            format!("{}: {shown}", field.label())
        } else if field == ItemFormField::Number {
            format!("{}: {}", field.label(), group_digits_by_four(form.field_value(field)))
        } else {
            format!("{}: {}", field.label(), form.field_value(field))
        };
        frame.render_widget(Paragraph::new(text).style(style), rows[i]);
    }

    let error_row = visible_fields.len();
    if let Some(err) = &form.error {
        frame.render_widget(Paragraph::new(format!("⚠ {err}")).style(Style::default().fg(ERROR)), rows[error_row]);
    }

    let mut ctrl_hints: Vec<&str> = vec![];
    if matches!(form.mode, ItemFormMode::Create) {
        ctrl_hints.push("Ctrl+T: change type");
    }
    if matches!(form.kind, ItemKind::Login) {
        ctrl_hints.push("Ctrl+G: random password");
        if matches!(form.mode, ItemFormMode::Edit { .. }) {
            ctrl_hints.push("Ctrl+R: view current password");
        }
    }

    let plain_hint_row = error_row + 2;
    let ctrl_hint_row = plain_hint_row + 1;
    frame.render_widget(
        Paragraph::new("Tab: field  Enter: save  Esc: cancel").style(Style::default().fg(MUTED)),
        rows[plain_hint_row],
    );
    if !ctrl_hints.is_empty() {
        frame.render_widget(
            Paragraph::new(ctrl_hints.join("  ")).style(Style::default().fg(MUTED)),
            rows[ctrl_hint_row],
        );
    }
}

fn group_digits_by_four(number: &str) -> String {
    number
        .chars()
        .collect::<Vec<_>>()
        .chunks(4)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}
