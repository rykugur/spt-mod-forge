// src/ui/palette.rs
// Palette modal + fuzzy filter for command palette (invoked via : or Ctrl-k).
// Per plan Step 12.1: simple fuzzy_filter (no crate dep).
// TDD: test written first using exact sample commands listed in plan.

use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::theme::Theme;

/// Simple fuzzy filter (no external crate).
/// Score: starts_with +10, contains +5, else 0 (case-insensitive substring match base).
/// Sort desc by score (stable on ties), return top matches (all if query empty).
pub fn fuzzy_filter<'a>(query: &str, items: &'a [&'a str]) -> Vec<(i32, &'a str)> {
    if query.trim().is_empty() {
        return items.iter().map(|&s| (0, s)).collect();
    }
    let q = query.to_ascii_lowercase();
    let mut scored: Vec<(i32, &str)> = items
        .iter()
        .map(|&item| {
            let i = item.to_ascii_lowercase();
            let score = if i.starts_with(&q) {
                10
            } else if i.contains(&q) {
                5
            } else {
                0
            };
            (score, item)
        })
        .filter(|(s, _)| *s > 0)
        .collect();
    scored.sort_by_key(|b| std::cmp::Reverse(b.0)); // desc score
    scored
}

/// Render the command palette modal (top input + filtered list below).
/// area is the full frame area; we calc centered popup inside.
/// input shows ">" + typed query (cursor simulated by reverse style on last char or trailing space).
/// j/k (or arrows) for selection in filtered (but selection state owned by caller; here render current).
/// Uses theme for border/accent.
pub fn render_palette(
    f: &mut Frame,
    area: Rect,
    query: &str,
    filtered: &[(i32, &str)], // from fuzzy_filter
    selected: usize,
    theme: &Theme,
) {
    let popup_area = centered_rect(60, 40, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Command Palette (type to filter, enter to run, esc close) ")
        .border_style(theme.border());
    let block = theme.apply_to_block(block);

    // input line + list
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // split inner: top for input (1 line), rest list
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Min(1),
        ])
        .split(inner);

    // input: prompt + query (style last char or add space for cursor feel)
    let input_line = Line::from(vec![
        Span::raw("> "),
        Span::raw(query),
        Span::styled(" ", Style::default().reversed()), // simple cursor indicator
    ]);
    let input_p = Paragraph::new(input_line).style(Style::default().fg(theme.accent()));
    f.render_widget(input_p, chunks[0]);

    // filtered list items (score not shown in v1, just the cmd strings)
    let list_items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, &(_, cmd))| {
            let style = if i == selected {
                Style::default().fg(theme.accent()).bold()
            } else {
                Style::default()
            };
            ListItem::new(cmd).style(style)
        })
        .collect();

    let list = List::new(list_items)
        .block(Block::default()); // no extra border, inner already
    f.render_widget(list, chunks[1]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // use classic split (consistent with existing main.rs style; works in ratatui 0.29 without Flex)
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
            ratatui::layout::Constraint::Percentage(percent_y),
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
            ratatui::layout::Constraint::Percentage(percent_x),
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_filter_test_with_plan_sample_commands() {
        // Exact sample commands from Task 12 plan description for palette:
        let samples: [&str; 10] = [
            "refresh lists (force)",
            "open settings",
            "toggle cache",
            "set color_scheme",
            "commit",
            "quit",
            "help",
            "toggle this mod",
            "view on Forge",
            "set color scheme", // variant
        ];
        let items: Vec<&str> = samples.iter().copied().collect();

        // empty query -> all with 0
        let all = fuzzy_filter("", &items);
        assert_eq!(all.len(), 10);

        // "ref" -> starts_with "refresh lists (force)" scores 10, should be first
        let res = fuzzy_filter("ref", &items);
        assert!(!res.is_empty());
        assert!(res[0].1.contains("refresh"), "top should be refresh: {:?}", res);
        assert_eq!(res[0].0, 10);

        // "set color" -> contains both "set color_scheme" and "set color scheme" score 5
        let res2 = fuzzy_filter("set color", &items);
        assert!(res2.len() >= 2);
        assert!(res2.iter().any(|(_, s)| s.contains("color_scheme")));
        // highest are the contains 5 (no start)
        assert!(res2[0].0 >= 5);

        // "commit" exact start -> 10
        let res3 = fuzzy_filter("commit", &items);
        assert_eq!(res3[0].1, "commit");
        assert_eq!(res3[0].0, 10);

        // no match
        let res4 = fuzzy_filter("zzzznotexist", &items);
        assert!(res4.is_empty());

        // "o" matches several contains/starts: open, toggle, ...
        let res5 = fuzzy_filter("o", &items);
        assert!(res5.len() >= 3);
        // "open settings" starts with o -> should rank high
        assert!(res5.iter().any(|(_,s)| *s == "open settings"));
    }

    #[test]
    fn fuzzy_sorts_desc_score_and_filters() {
        let items = vec!["abc", "a b c", "xyzabc", "abx"];
        let res = fuzzy_filter("a", &items);
        // "abc" starts 10, "a b c" starts 10, "abx" starts 10, "xyzabc" contains 5
        assert!(res.len() == 4, "all should match 'a' as start or contain: {:?}", res);
        assert!(res[0].0 == 10);
        assert!(res[1].0 == 10);
        assert!(res[2].0 == 10);
        assert!(res[3].0 == 5);
    }
}
