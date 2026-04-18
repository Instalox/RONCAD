//! Desktop shell: panels, toolbars, inspectors, viewport host.
//! UI never owns geometry truth; it reads state and emits commands.

mod dimensions;

pub mod inspector;
pub mod project_tree;
pub mod right_sidebar;
pub mod shell;
pub mod status_bar;
pub mod theme;
pub mod tool_shelf;
pub mod toolbar;
pub mod viewport;

pub use shell::{ShellContext, ShellResponse, render_shell};
pub use theme::apply_dark_theme;
