// src/ui/commit.rs
// Commit review modal + apply phase UI.
// Per plan Step 12.2: two columns TO INSTALL / TO UNINSTALL, list of names+counts,
// big "y to apply, n cancel", during apply show per-mod spinner + "downloading 34%" etc.
// Reuses animation::Spinner (braille) + Theme.accent/success etc for live status.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::animation::Spinner;
use crate::theme::Theme;
use crate::ManagedMod;

/// Render commit review modal (or apply progress).
/// to_install / to_uninstall are the slices from state.compute_pending().
/// applying: if true, show progress UI with spinners instead of y/n prompt.
/// progress map e.g. per id "downloading 34%" or use spinner + status text.
pub fn render_commit(
    f: &mut Frame,
    area: Rect,
    to_install: &[ManagedMod],
    to_uninstall: &[ManagedMod],
    applying: bool,
    progress: &std::collections::HashMap<i64, String>, // e.g. id -> "34%"
    spinner: &mut Spinner, // caller owns & ticks it in loop for animation
    theme: &Theme,
) {
    let popup_area = centered_rect(70, 50, area);
    f.render_widget(Clear, popup_area);

    let title = if applying { " Applying Changes... (esc to abort) " } else { " Commit Review (y: apply, n/esc: cancel) " };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme.border());
    let block = theme.apply_to_block(block);
    let inner = block.inner(popup_area);
    f.render_widget(&block, popup_area);

    // two column layout
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    // left: TO INSTALL
    let mut left_lines: Vec<Line> = vec![Line::from(Span::styled("TO INSTALL", Style::default().fg(theme.success()).bold()))];
    if to_install.is_empty() {
        left_lines.push(Line::from("(none)"));
    } else {
        for m in to_install {
            let ptext = progress.get(&m.forge_id).cloned().unwrap_or_default();
            if applying && !ptext.is_empty() {
                let spin = spinner.frame();
                left_lines.push(Line::from(format!("{} {} {} ({})", spin, m.name, ptext, m.last_known_version.as_deref().unwrap_or("?"))));
            } else {
                left_lines.push(Line::from(format!("- {} v{}", m.name, m.last_known_version.as_deref().unwrap_or("?"))));
            }
        }
    }
    left_lines.push(Line::from(format!("({} mods)", to_install.len())));
    let left_p = Paragraph::new(left_lines);
    f.render_widget(left_p, cols[0]);

    // right: TO UNINSTALL
    let mut right_lines: Vec<Line> = vec![Line::from(Span::styled("TO UNINSTALL", Style::default().fg(theme.error()).bold()))];
    if to_uninstall.is_empty() {
        right_lines.push(Line::from("(none)"));
    } else {
        for m in to_uninstall {
            if applying {
                let spin = spinner.frame();
                right_lines.push(Line::from(format!("{} {}", spin, m.name)));
            } else {
                right_lines.push(Line::from(format!("- {}", m.name)));
            }
        }
    }
    right_lines.push(Line::from(format!("({} mods)", to_uninstall.len())));
    let right_p = Paragraph::new(right_lines);
    f.render_widget(right_p, cols[1]);

    if !applying {
        let _footer = Paragraph::new("Press y to apply changes, n or Esc to cancel.").style(Style::default().fg(theme.warning()));
        // (footer text noted in title; render would require extra layout split for v1)
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    let popup = ratatui::layout::Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1];
    popup
}
