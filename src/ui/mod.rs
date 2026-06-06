// src/ui/mod.rs
// Task 12: UI panes and modals (list, detail, palette fuzzy, settings, commit review).
// ratatui renders (side-effect only) + simple fuzzy (no extra crate) for command palette.
// Split panes (list+detail) + popovers for : (palette), s (settings), c/Enter (commit review).
// All fns take data slices + &Theme (or minimal view models) per plan (full AppState in Task 13).
// Mod decl supporting in models.rs left unstaged; only ui/ files committed in this task's steps.
// Follows YAGNI v1: basic scoring fuzzy, basic list/detail, modal calc with Clear+Block, live settings display (apply in 13),
// per-mod spinners reuse animation::Spinner + Theme in commit apply phase.

pub mod palette;
pub mod panes;
pub mod settings;
pub mod commit;
