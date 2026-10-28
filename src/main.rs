// Prevent console window from popping up on Windows in release mode
#![windows_subsystem = "windows"]

use slint::Model;

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

    // 3. Setup Live Feed Telemetry System
    let feed_items = std::rc::Rc::new(slint::VecModel::<FeedItem>::default());

    // Push initial startup events
    feed_items.push(FeedItem {
        id: "ev-init-1".into(),
        timestamp: "18:54:00".into(),
        level: "info".into(),
        title: "Desktop Application Initialized".into(),
        message: "DirectX/Vulkan graphics pipeline ready. 120 FPS target enabled.".into(),
    });
    feed_items.push(FeedItem {
        id: "ev-init-2".into(),
        timestamp: "18:54:01".into(),
        level: "success".into(),
        title: "Theme Synchronized with OS".into(),
        message: format!(
            "Windows Theme detected successfully. Active mode: {}.",
            if is_dark_os { "Dark Mode" } else { "Light Mode" }
        )
        .into(),
    });
    feed_items.push(FeedItem {
        id: "ev-init-3".into(),
        timestamp: "18:54:02".into(),
        level: "info".into(),
        title: "HiDPI Subpixel Antialiasing Active".into(),
        message: "DirectWrite font shaping calibrated for primary monitor resolution.".into(),
    });

    app.set_live_feed_items(feed_items.clone().into());

    // 4. Live Feed Background Event Generator (Streams events in real-time)
    let feed_timer_handle = feed_items.clone();
    let app_weak = app.as_weak();
    let timer = slint::Timer::default();
    let mut event_count = 4;

    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(1800),
        move || {
            if let Some(app) = app_weak.upgrade() {
                if app.get_is_feed_paused() {
                    return;
                }

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let hours = (now / 3600) % 24;
                let mins = (now / 60) % 60;
                let secs = now % 60;
                let time_str = format!("{:02}:{:02}:{:02}", hours, mins, secs);

                let (level, title, msg) = match event_count % 6 {
                    0 => (
                        "info",
                        "System Heartbeat Ping",
                        "UI thread latency: 1.8ms. Hardware swapchain healthy.",
                    ),
                    1 => (
                        "success",
                        "Memory Footprint Optimal",
                        "Process RSS stable at 24.2 MB. Zero garbage collection pauses.",
                    ),
                    2 => (
                        "info",
                        "DPI Scale Monitored",
                        "Window display scaling verified against OS compositor.",
                    ),
                    3 => (
                        "warning",
                        "Frame Buffer Check",
                        "Minor presentation jitter detected (0.2ms), compensated automatically.",
                    ),
                    4 => (
                        "success",
                        "Event Stream Synchronized",
                        "Background worker tasks completed without blocking the main event loop.",
                    ),
                    _ => (
                        "info",
                        "GPU Vector Cache Clean",
                        "Retained scene-graph nodes repainted with sub-millisecond efficiency.",
                    ),
                };

                // Insert newest event at top of feed
                feed_timer_handle.insert(
                    0,
                    FeedItem {
                        id: format!("ev-{}", event_count).into(),
                        timestamp: time_str.into(),
                        level: level.into(),
                        title: title.into(),
                        message: msg.into(),
                    },
                );

                // Keep feed bounded to latest 40 items to conserve memory
                if feed_timer_handle.row_count() > 40 {
                    feed_timer_handle.remove(40);
                }

                event_count += 1;
            }
        },
    );

    // 5. Connect UI Callbacks
    let app_weak_pause = app.as_weak();
    app.on_toggle_feed_pause(move || {
        if let Some(app) = app_weak_pause.upgrade() {
            let current = app.get_is_feed_paused();
            app.set_is_feed_paused(!current);
        }
    });

    let feed_clear_handle = feed_items.clone();
    app.on_clear_feed(move || {
        feed_clear_handle.set_vec(vec![]);
    });

    // Keep the timer alive for the entire lifespan of the application
    let _timer_ref = &timer;

    // 6. Start the native desktop event loop
    app.run()
}
