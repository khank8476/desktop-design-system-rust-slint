slint::include_modules!();

pub mod db;
pub mod system_info;
pub mod file_tree;

/// Queries the operating system for its active color scheme (Dark / Light).
/// Fully compatible across Windows, macOS, and Linux.
pub fn is_system_dark_mode() -> bool {
    match dark_light::detect() {
        dark_light::Mode::Dark => true,
        dark_light::Mode::Light => false,
        dark_light::Mode::Default => false,
    }
}

