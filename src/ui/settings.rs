// src/ui/settings.rs
// Settings modal render.
// Per plan: list of rows e.g. "color scheme: [terminal]  (use left/right or enter to cycle)"
// live on change would call app.apply_color_scheme (stub here; full in Task 13).
// Other: animations toggle, default_sort etc from UiConfig but minimal for v1.

use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::theme::Theme;

/// Render settings modal (centered popup).
/// current_scheme e.g. "terminal". On 'left/right' in event loop (T13) would cycle and re-render + apply.
pub fn render_settings(
    f: &mut Frame,
    area: Rect,
    current_scheme: &str,
    animations_enabled: bool,
    theme: &Theme,
) {
    // inline centered (dupe for YAGNI; palette's is private)
    let popup_area = centered_rect(50, 30, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Settings (left/right to change, enter/esc close) ")
        .border_style(theme.border());
    let block = theme.apply_to_block(block);
    let inner = block.inner(popup_area);
    f.render_widget(&block, popup_area);

    let lines = vec![
        Line::from(format!("color scheme: [{}]  (left/right or enter to cycle)", current_scheme)),
        Line::from(format!("animations: [{}]  (toggle with space)", if animations_enabled { "on" } else { "off" })),
        Line::from(""),
        Line::from("Changes apply live (color hot-reload via Theme)."),
        Line::from("Full persistence + more options in later tasks."),
    ];

    let p = Paragraph::new(lines).style(Style::default().fg(theme.accent()));
    f.render_widget(p, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
            ratatui::layout::Constraint::Percentage(percent_y),
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    let popup = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
            ratatui::layout::Constraint::Percentage(percent_x),
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1];
    popup
}
