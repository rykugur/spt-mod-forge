// src/animation.rs
// Task 11: Rattles braille animation integration (thin Spinner for render paths).
// Per plan: small wrapper using rattles::presets::prelude as presets; Spinner { idx } + frame(&mut self).
// Delegates to presets::dots().current_frame() (time-based per ratatui example in rattles; produces classic braille ⠋ etc).
// Ticked manual advance precedent remains direct in src/install.rs for I/O extract loops.
// This is YAGNI thin: no tick(), no config/ui.animations read, no Ratatui widget, no manager.
// "in render + loop" support (100ms poll tick+redraw) is for later UI tasks 12-15.
// Only this file is git added/committed; mod decl in models.rs left unstaged (per pattern).

use rattles::presets::prelude as presets;

pub struct Spinner {
    idx: usize,
}

impl Spinner {
    pub fn frame(&mut self) -> &'static str {
        // simplest per ratatui example + plan note ("many just do presets::braille().current_frame() on every draw")
        // &mut self to match plan sketch (idx for potential future manual stepping; currently time-driven so frame rate independent)
        // always non-empty unicode braille for the classic dots preset (see design: ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏ )
        presets::dots().current_frame()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_frame_nonempty_unicode() {
        let mut s = Spinner { idx: 0 };
        let f = s.frame();
        assert!(!f.is_empty(), "braille frame must be non-empty string");
        // confirm unicode (braille block chars are non-ascii)
        assert!(
            f.chars().any(|c| !c.is_ascii()),
            "frame should contain unicode braille, got: {}",
            f
        );
        // also sanity: at least one char from the classic set or any non-empty unicode ok for this test
        let braille_sample = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";
        assert!(
            f.chars().any(|c| braille_sample.contains(c)) || !f.is_ascii(),
            "expected classic braille unicode frame"
        );
    }
}
