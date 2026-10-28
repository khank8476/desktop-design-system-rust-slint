// Prevent console window from popping up on Windows in release mode
#![windows_subsystem = "windows"]

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    // 1. Create the native App Window
    let app = AppWindow::new()?;

    // 2. Auto-detect OS Dark / Light mode (Windows, macOS, Linux)
    let is_dark_os = match dark_light::detect() {
        dark_light::Mode::Dark => true,
        dark_light::Mode::Light => false,
        dark_light::Mode::Default => false,
    };
    app.global::<Palette>().set_dark_mode(is_dark_os);

    // 3. We let the OS Window Manager position the window naturally
    // (Windows default placement avoids off-screen issues and multi-monitor bugs)

    // 4. Start the native desktop event loop
    app.run()
}
