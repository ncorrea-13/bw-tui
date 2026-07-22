use super::{ERROR, MUTED, OK, WARN, boxed, panel};
use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::Paragraph,
};

pub(super) fn draw_generator_tab(frame: &mut Frame, app: &App, area: Rect) {
    let inner = panel(frame, area, "Password generator", 60, 12);
    draw_generator_options(frame, app, inner, "(c: copy)", None);
}

pub(super) fn draw_item_form_password_picker(frame: &mut Frame, app: &App) {
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
    let toggle = |on: bool| {
        if on {
            Style::default().fg(OK)
        } else {
            Style::default().fg(MUTED)
        }
    };

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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    frame.render_widget(
        Paragraph::new(format!("Length: {} (↑/↓)", opts.length)),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(format!(
            "[u] Uppercase: {}",
            if opts.uppercase { "yes" } else { "no" }
        ))
        .style(toggle(opts.uppercase)),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(format!(
            "[l] Lowercase: {}",
            if opts.lowercase { "yes" } else { "no" }
        ))
        .style(toggle(opts.lowercase)),
        chunks[2],
    );
    frame.render_widget(
        Paragraph::new(format!(
            "[n] Numbers: {}",
            if opts.numbers { "yes" } else { "no" }
        ))
        .style(toggle(opts.numbers)),
        chunks[3],
    );
    frame.render_widget(
        Paragraph::new(format!(
            "[s] Special: {}",
            if opts.special { "yes" } else { "no" }
        ))
        .style(toggle(opts.special)),
        chunks[4],
    );

    if app.busy {
        let label = app.busy_label.as_deref().unwrap_or("Working...");
        frame.render_widget(
            Paragraph::new(format!("{} {label}", app.spinner())).style(Style::default().fg(WARN)),
            chunks[5],
        );
    } else if let Some(err) = &app.generator.error {
        frame.render_widget(
            Paragraph::new(format!("\u{f071} {err}")).style(Style::default().fg(ERROR)),
            chunks[5],
        );
    } else if let Some(pw) = &app.generator.result {
        let result_line = if result_hint.is_empty() {
            format!("> {pw}")
        } else {
            format!("> {pw}  {result_hint}")
        };
        frame.render_widget(
            Paragraph::new(result_line)
                .style(Style::default().fg(WARN).add_modifier(Modifier::BOLD)),
            chunks[5],
        );
    } else {
        frame.render_widget(
            Paragraph::new("Enter: generate").style(Style::default().fg(MUTED)),
            chunks[5],
        );
    }

    if let Some(hint) = bottom_hint {
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(MUTED)),
            chunks[7],
        );
    }
}
