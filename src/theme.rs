// src/theme.rs
use ratatui::{style::{Color, Style}, widgets::Block};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Terminal,
    Catppuccin,
    TokyoNight,
    Dracula,
    // add more as easy
}

impl Theme {
    pub fn from_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "catppuccin" | "catppuccin-mocha" => Theme::Catppuccin,
            "tokyonight" | "tokyo-night" => Theme::TokyoNight,
            "dracula" => Theme::Dracula,
            _ => Theme::Terminal,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Theme::Terminal => "terminal",
            Theme::Catppuccin => "catppuccin",
            Theme::TokyoNight => "tokyonight",
            Theme::Dracula => "dracula",
        }
    }

    pub fn border(&self) -> Style {
        match self {
            Theme::Terminal => Style::default().fg(Color::Reset).bg(Color::Reset),
            Theme::Catppuccin => Style::default().fg(Color::Rgb(137, 180, 250)), // blue-ish
            Theme::TokyoNight => Style::default().fg(Color::Rgb(122, 162, 247)),
            Theme::Dracula => Style::default().fg(Color::Rgb(189, 147, 249)),
        }
    }

    pub fn accent(&self) -> Color {
        match self {
            Theme::Terminal => Color::Reset,
            Theme::Catppuccin => Color::Rgb(245, 194, 231), // pink
            Theme::TokyoNight => Color::Rgb(158, 206, 106),
            Theme::Dracula => Color::Rgb(80, 250, 123),
        }
    }

    pub fn success(&self) -> Color {
        match self {
            Theme::Terminal => Color::Reset,
            Theme::Catppuccin => Color::Rgb(166, 227, 161),
            Theme::TokyoNight => Color::Rgb(158, 206, 106),
            Theme::Dracula => Color::Rgb(80, 250, 123),
        }
    }

    pub fn warning(&self) -> Color {
        match self {
            Theme::Terminal => Color::Reset,
            Theme::Catppuccin => Color::Rgb(249, 226, 175),
            Theme::TokyoNight => Color::Rgb(224, 175, 104),
            Theme::Dracula => Color::Rgb(241, 250, 140),
        }
    }

    pub fn error(&self) -> Color {
        match self {
            Theme::Terminal => Color::Reset,
            Theme::Catppuccin => Color::Rgb(243, 139, 168),
            Theme::TokyoNight => Color::Rgb(247, 118, 142),
            Theme::Dracula => Color::Rgb(255, 85, 85),
        }
    }

    /// Minimal YAGNI helper: applies the theme's border style to a ratatui Block.
    /// Future UI code can do theme.apply_to_block(Block::default().borders(...))
    pub fn apply_to_block<'a>(&self, block: Block<'a>) -> Block<'a> {
        block.border_style(self.border())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_theme_uses_reset() {
        let t = Theme::from_name("terminal");
        assert_eq!(t, Theme::Terminal);
        let b = t.border();
        // In ratatui, default fg is None which renders as terminal; we use Reset explicitly for "respect terminal"
        assert!(matches!(b.fg, Some(Color::Reset)) || b.fg.is_none());
    }
}
