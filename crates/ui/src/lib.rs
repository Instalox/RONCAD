//! Desktop shell: panels, toolbars, inspectors, viewport host.
//! UI never owns geometry truth; it reads state and emits commands.

mod command_palette;
mod dimensions;
mod hud_state;

pub use command_palette::CommandPaletteState;
pub mod inspector;
pub mod project_tree;
pub mod right_sidebar;
pub mod shell;
pub mod status_bar;
pub mod theme;
pub mod tool_shelf;
pub mod toolbar;
pub mod viewport;

pub use hud_state::HudEditState;
pub use shell::{render_shell, ShellContext, ShellResponse};
pub use theme::apply_dark_theme;
pub use viewport::{ViewportInteractionHandler, ViewportInteractionState};
