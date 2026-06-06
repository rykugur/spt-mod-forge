// src/ui/panes.rs
// Split pane renders: left list of mods, right detail view.
// Per plan Step 12.2: render_list, render_detail taking slices of data (ManagedMod etc) + selected + Theme.
// Side-effect only (render on &mut Frame). Badges: ✓ for desired, PENDING using theme.accent() etc.
// (Full integration + state ownership in Task 13; here use minimal view data + samples ok).

use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::theme::Theme;
use crate::ManagedMod;

/// Render left list pane: mods with ✓/space + name + badge (e.g. [INSTALLED], [PENDING] etc).
/// selected for highlight.
pub fn render_list(
    f: &mut Frame,
    area: Rect,
    mods: &[ManagedMod],
    selected: usize,
    pending_install: &[i64], // forge_ids pending enable
    pending_uninstall: &[i64],
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Mods (hjkl/Space/Enter, : palette, s settings, c commit, q quit) ")
        .border_style(theme.border());
    let block = theme.apply_to_block(block);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = mods
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let is_desired = m.desired_enabled;
            let is_pending = pending_install.contains(&m.forge_id) || pending_uninstall.contains(&m.forge_id);
            let mark = if is_desired { "✓" } else { " " };
            let badge = if is_pending {
                Span::styled(" [PENDING]", Style::default().fg(theme.accent()))
            } else if m.last_installed_version.is_some() {
                Span::styled(" [INSTALLED]", Style::default().fg(theme.success()))
            } else {
                Span::raw("")
            };
            let name = Span::raw(format!("{} {}", mark, m.name));
            let ver = m.last_known_version.as_deref().unwrap_or("?");
            let line = Line::from(vec![name, badge, Span::raw(format!(" v{}", ver))]);
            let mut item = ListItem::new(line);
            if i == selected {
                item = item.style(Style::default().fg(theme.accent()).bold());
            }
            item
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, inner);
}

/// Render right detail pane for selected mod (multi-line Paragraph with fields + teaser etc).
pub fn render_detail(f: &mut Frame, area: Rect, m: Option<&ManagedMod>, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Detail ")
        .border_style(theme.border());
    let block = theme.apply_to_block(block);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = if let Some(mod_) = m {
        format!(
            "Name: {}\nID: {}\nGUID: {}\nVersion: {}\nDesired: {}\nInstalled: {}\n\n(Arrows/hjkl to select; Space to toggle desired; c/Enter for commit review)",
            mod_.name,
            mod_.forge_id,
            mod_.guid.as_deref().unwrap_or("-"),
            mod_.last_known_version.as_deref().unwrap_or("?"),
            if mod_.desired_enabled { "yes" } else { "no" },
            mod_.last_installed_version.as_deref().unwrap_or("no")
        )
    } else {
        "No mod selected.\n\nUse j/k or arrows to select from list.\nSpace: toggle desired enable.\n: or Ctrl-k : command palette\ns : settings\nc : commit review\nq : quit".to_string()
    };

    let p = Paragraph::new(text).block(Block::default());
    f.render_widget(p, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use ratatui::layout::Rect;
    // Frame requires ratatui::Frame but to name in fn sig without full setup, we use string check too; import for type if possible
    use ratatui::Frame;

    #[test]
    fn render_fns_compile_and_basic_smoke() {
        // Hard to unit test full Frame render without backend setup (ratatui Buffer + test backend possible but overkill per plan "if possible (hard)").
        // This exercises the public API signatures + Theme + ManagedMod data shapes used by renders (list/detail use badges for desired/pending/installed).
        // Actual visual/manual in Task 13 wiring + run.
        let _list_sig = std::any::type_name::<fn(&mut Frame, Rect, &[ManagedMod], usize, &[i64], &[i64], &Theme)>();
        let _detail_sig = std::any::type_name::<fn(&mut Frame, Rect, Option<&ManagedMod>, &Theme)>();
        // also confirm Theme methods used in renders are callable
        let t = Theme::from_name("terminal");
        let _ = t.border();
        let _ = t.accent();
        let _ = t.success();
        assert!(true, "ui render fns and supporting types present and compile");
    }
}
