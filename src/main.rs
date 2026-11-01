// Prevent console window from popping up on Windows in release mode
#![windows_subsystem = "windows"]

use desktop_design_system::db::DatabaseManager;
use desktop_design_system::file_tree::{FileNode, FileTreeManager};
use desktop_design_system::shortcut::{
    check_shortcut, create_shortcut, get_desktop_dir, remove_shortcut, ShortcutResult, ShortcutStatus,
};
use std::path::PathBuf;
use desktop_design_system::system_info::SystemDiagnostics;
use slint::Model;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

slint::include_modules!();

enum DbCmd {
    LoadAll,
    Insert {
        service: String,
        runtime: String,
        status: String,
        latency: String,
    },
    Update {
        id: i64,
        service: String,
        runtime: String,
        status: String,
        latency: String,
    },
    Delete {
        id: i64,
    },
    SetPreference {
        key: String,
        value: String,
    },
}

fn main() -> Result<(), slint::PlatformError> {
    // 1. Create the native App Window
    let app = AppWindow::new()?;

    // 2. Initialize Display Mode: Check SQLite Preferences first, fallback to OS auto-detection
    let is_dark_os = match dark_light::detect() {
        dark_light::Mode::Dark => true,
        dark_light::Mode::Light => false,
        dark_light::Mode::Default => false,
    };

    let mut is_dark = is_dark_os;
    let mut theme_pref_desc = "Auto-detected from OS theme".to_string();

    if let Ok(db_init) = DatabaseManager::new("desktop_crud.db") {
        if let Ok(Some(saved_theme)) = db_init.get_preference("theme_mode") {
            match saved_theme.as_str() {
                "dark" => {
                    is_dark = true;
                    theme_pref_desc = "Restored from SQLite preferences (Slate Dark)".to_string();
                }
                "light" => {
                    is_dark = false;
                    theme_pref_desc = "Restored from SQLite preferences (Clean Light)".to_string();
                }
                _ => {
                    is_dark = is_dark_os;
                    theme_pref_desc = "Synced with OS system theme (Default)".to_string();
                }
            }
        }
    }

    app.global::<Palette>().set_dark_mode(is_dark);
    app.set_theme_mode_status(theme_pref_desc.clone().into());

    // Restore persistent accent color palette if saved in SQLite
    if let Ok(db_init) = DatabaseManager::new("desktop_crud.db") {
        if let Ok(Some(saved_accent)) = db_init.get_preference("accent_color") {
            if let Some(col) = parse_hex_color(&saved_accent) {
                app.global::<Palette>().set_accent(col);
                app.set_demo_color(col);
                app.set_demo_color_hex(saved_accent.into());
            }
        }
    }

    // 3. Setup Live Feed Telemetry System
    let feed_items = Rc::new(slint::VecModel::<FeedItem>::default());

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
        title: "Theme Mode Synchronized".into(),
        message: format!(
            "Active: {}. ({})",
            if is_dark { "Slate Dark" } else { "Clean Light" },
            theme_pref_desc
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
    let app_weak_timer = app.as_weak();
    let timer = slint::Timer::default();
    let mut event_count = 4;

    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(1800),
        move || {
            if let Some(app) = app_weak_timer.upgrade() {
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

                // Keep feed bounded to latest 40 items
                if feed_timer_handle.row_count() > 40 {
                    feed_timer_handle.remove(40);
                }

                event_count += 1;
            }
        },
    );

    // 5. Connect Feed Callbacks
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

    // 6. Connect Toast Notification Trigger with Auto-Dismiss Timer
    let toast_timer = Rc::new(RefCell::new(slint::Timer::default()));
    let toast_timer_clone = toast_timer.clone();
    let app_weak_toast = app.as_weak();

    app.on_trigger_sample_toast(move |status, title, message| {
        if let Some(app) = app_weak_toast.upgrade() {
            app.set_toast_status(status);
            app.set_toast_title(title);
            app.set_toast_message(message);
            app.set_toast_open(true);

            // Auto-dismiss toast after 3.5 seconds
            let app_weak_dismiss = app.as_weak();
            toast_timer_clone.borrow_mut().start(
                slint::TimerMode::SingleShot,
                std::time::Duration::from_millis(3500),
                move || {
                    if let Some(app) = app_weak_dismiss.upgrade() {
                        app.set_toast_open(false);
                    }
                },
            );
        }
    });

    // 7. Setup Background SQLite Worker (Fearless Concurrency & Zero UI Latency)
    let (db_tx, db_rx) = mpsc::channel::<DbCmd>();
    let app_weak_db = app.as_weak();

    std::thread::spawn(move || {
        let db = match DatabaseManager::new("desktop_crud.db") {
            Ok(db) => db,
            Err(e) => {
                eprintln!("Failed to open SQLite database: {e}");
                return;
            }
        };

        while let Ok(cmd) = db_rx.recv() {
            match cmd {
                DbCmd::LoadAll => {
                    if let Ok(records) = db.get_all() {
                        let items: Vec<TableRowData> = records
                            .into_iter()
                            .map(|r| TableRowData {
                                id: r.id.to_string().into(),
                                service: r.service.into(),
                                runtime: r.runtime.into(),
                                status: r.status.into(),
                                latency: r.latency.into(),
                            })
                            .collect();

                        let app_weak = app_weak_db.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_weak.upgrade() {
                                let model = Rc::new(slint::VecModel::from(items));
                                app.set_db_records(model.into());
                            }
                        });
                    }
                }
                DbCmd::Insert {
                    service,
                    runtime,
                    status,
                    latency,
                } => {
                    let insert_res = db.insert(&service, &runtime, &status, &latency);
                    let app_weak = app_weak_db.clone();
                    let is_ok = insert_res.is_ok();
                    let msg = match insert_res {
                        Ok(rec) => format!("Service '{}' (#{}) committed to SQLite.", rec.service, rec.id),
                        Err(e) => format!("Failed to insert record: {e}"),
                    };

                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(app) = app_weak.upgrade() {
                            let toast_status = if is_ok { "success" } else { "danger" };
                            let toast_title = if is_ok { "Record Inserted" } else { "Database Error" };
                            app.invoke_trigger_sample_toast(toast_status.into(), toast_title.into(), msg.into());
                        }
                    });

                    // Reload table automatically after write
                    if let Ok(records) = db.get_all() {
                        let items: Vec<TableRowData> = records
                            .into_iter()
                            .map(|r| TableRowData {
                                id: r.id.to_string().into(),
                                service: r.service.into(),
                                runtime: r.runtime.into(),
                                status: r.status.into(),
                                latency: r.latency.into(),
                            })
                            .collect();

                        let app_weak = app_weak_db.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_weak.upgrade() {
                                let model = Rc::new(slint::VecModel::from(items));
                                app.set_db_records(model.into());
                            }
                        });
                    }
                }
                DbCmd::Update {
                    id,
                    service,
                    runtime,
                    status,
                    latency,
                } => {
                    let update_res = db.update(id, &service, &runtime, &status, &latency);
                    let app_weak = app_weak_db.clone();
                    let is_ok = update_res.is_ok();
                    let msg = match update_res {
                        Ok(true) => format!("Service record #{} successfully updated in SQLite.", id),
                        Ok(false) => format!("Record #{} not found in SQLite.", id),
                        Err(e) => format!("Failed to update record: {e}"),
                    };

                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(app) = app_weak.upgrade() {
                            let toast_status = if is_ok { "success" } else { "danger" };
                            let toast_title = if is_ok { "Record Updated" } else { "Database Error" };
                            app.invoke_trigger_sample_toast(toast_status.into(), toast_title.into(), msg.into());
                        }
                    });

                    // Reload table automatically after update
                    if let Ok(records) = db.get_all() {
                        let items: Vec<TableRowData> = records
                            .into_iter()
                            .map(|r| TableRowData {
                                id: r.id.to_string().into(),
                                service: r.service.into(),
                                runtime: r.runtime.into(),
                                status: r.status.into(),
                                latency: r.latency.into(),
                            })
                            .collect();

                        let app_weak = app_weak_db.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_weak.upgrade() {
                                let model = Rc::new(slint::VecModel::from(items));
                                app.set_db_records(model.into());
                            }
                        });
                    }
                }
                DbCmd::Delete { id } => {
                    let delete_res = db.delete(id);
                    let app_weak = app_weak_db.clone();
                    let is_ok = delete_res.is_ok();
                    let msg = match delete_res {
                        Ok(true) => format!("Service record #{} deleted permanently.", id),
                        Ok(false) => format!("Record #{} not found in SQLite.", id),
                        Err(e) => format!("Failed to delete record: {e}"),
                    };

                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(app) = app_weak.upgrade() {
                            let toast_status = if is_ok { "success" } else { "danger" };
                            let toast_title = if is_ok { "Record Deleted" } else { "Database Error" };
                            app.invoke_trigger_sample_toast(toast_status.into(), toast_title.into(), msg.into());
                        }
                    });

                    // Reload table automatically after delete
                    if let Ok(records) = db.get_all() {
                        let items: Vec<TableRowData> = records
                            .into_iter()
                            .map(|r| TableRowData {
                                id: r.id.to_string().into(),
                                service: r.service.into(),
                                runtime: r.runtime.into(),
                                status: r.status.into(),
                                latency: r.latency.into(),
                            })
                            .collect();

                        let app_weak = app_weak_db.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_weak.upgrade() {
                                let len = items.len() as i32;
                                let model = Rc::new(slint::VecModel::from(items));
                                app.set_db_records(model.into());
                                if app.get_selected_record_idx() >= len {
                                    app.set_selected_record_idx(std::cmp::max(0, len - 1));
                                }
                            }
                        });
                    }
                }
                DbCmd::SetPreference { key, value } => {
                    let _ = db.set_preference(&key, &value);
                }
            }
        }
    });

    // 8. Connect CRUD UI Callbacks to Background SQLite Worker
    let db_tx_add = db_tx.clone();
    app.on_db_add_record(move |srv, run, st, lat| {
        let _ = db_tx_add.send(DbCmd::Insert {
            service: srv.to_string(),
            runtime: run.to_string(),
            status: st.to_string(),
            latency: lat.to_string(),
        });
    });

    let db_tx_upd = db_tx.clone();
    app.on_db_update_record(move |id_str, srv, run, st, lat| {
        if let Ok(id) = id_str.parse::<i64>() {
            let _ = db_tx_upd.send(DbCmd::Update {
                id,
                service: srv.to_string(),
                runtime: run.to_string(),
                status: st.to_string(),
                latency: lat.to_string(),
            });
        }
    });

    let db_tx_del = db_tx.clone();
    let app_weak_del = app.as_weak();
    app.on_db_delete_selected(move |idx| {
        if let Some(app) = app_weak_del.upgrade() {
            let records = app.get_db_records();
            if let Some(row) = records.row_data(idx as usize) {
                if let Ok(id) = row.id.parse::<i64>() {
                    let _ = db_tx_del.send(DbCmd::Delete { id });
                }
            }
        }
    });

    let db_tx_refresh = db_tx.clone();
    app.on_db_refresh(move || {
        let _ = db_tx_refresh.send(DbCmd::LoadAll);
    });

    // 9. Initial Load from SQLite on Startup
    let _ = db_tx.send(DbCmd::LoadAll);

    // Restore User Preferences from SQLite (Persistent Sidebar & UI State)
    if let Ok(db_init) = DatabaseManager::new("desktop_crud.db") {
        if let Ok(Some(collapsed)) = db_init.get_preference("sidebar_collapsed") {
            if collapsed == "true" {
                app.set_sidebar_collapsed(true);
            }
        }
    }

    let db_tx_pref = db_tx.clone();
    app.on_sidebar_toggle_collapsed(move |collapsed| {
        let _ = db_tx_pref.send(DbCmd::SetPreference {
            key: "sidebar_collapsed".into(),
            value: collapsed.to_string(),
        });
    });

    let db_tx_theme = db_tx.clone();
    let app_weak_theme = app.as_weak();
    app.on_theme_toggle_dark_mode(move |is_dark| {
        let _ = db_tx_theme.send(DbCmd::SetPreference {
            key: "theme_mode".into(),
            value: if is_dark { "dark".into() } else { "light".into() },
        });
        if let Some(app) = app_weak_theme.upgrade() {
            app.set_theme_mode_status(
                if is_dark {
                    "Saved: Slate Dark Theme (Persistent)".into()
                } else {
                    "Saved: Clean Light Theme (Persistent)".into()
                }
            );
        }
    });

    let db_tx_reset = db_tx.clone();
    let app_weak_reset = app.as_weak();
    app.on_theme_reset_os(move || {
        let os_dark = match dark_light::detect() {
            dark_light::Mode::Dark => true,
            dark_light::Mode::Light => false,
            dark_light::Mode::Default => false,
        };
        let _ = db_tx_reset.send(DbCmd::SetPreference {
            key: "theme_mode".into(),
            value: "system".into(),
        });
        if let Some(app) = app_weak_reset.upgrade() {
            app.global::<Palette>().set_dark_mode(os_dark);
            app.set_theme_mode_status(
                format!("Reset to OS Auto-detection ({})", if os_dark { "Dark" } else { "Light" }).into(),
            );
        }
    });

    // 10. Sample 360° Hardware Diagnostics & GPU Telemetry
    let initial_diag = SystemDiagnostics::query();
    apply_hardware_diagnostics(&app, &initial_diag);

    let app_weak_hw = app.as_weak();
    app.on_refresh_hardware_diagnostics(move || {
        let diag = SystemDiagnostics::query();
        if let Some(app) = app_weak_hw.upgrade() {
            apply_hardware_diagnostics(&app, &diag);
            app.invoke_trigger_sample_toast(
                "success".into(),
                "Hardware Resampled".into(),
                "Hardware telemetry and GPU status refreshed from system registry.".into(),
            );
        }
    });

    // 11. In-Process File Explorer TreeView Engine & Directory Browser
    let tree_manager = Arc::new(Mutex::new(FileTreeManager::new(".")));
    let initial_tree_nodes = tree_manager.lock().unwrap().generate_visible_nodes();
    let slint_tree_nodes = map_file_nodes(&initial_tree_nodes);
    app.set_tree_items(Rc::new(slint::VecModel::from(slint_tree_nodes)).into());
    app.set_tree_current_path(tree_manager.lock().unwrap().current_root_display().into());

    let tree_mgr_expand = tree_manager.clone();
    let app_weak_expand = app.as_weak();
    app.on_tree_toggle_expand(move |node_id, _curr_state| {
        let mut mgr = tree_mgr_expand.lock().unwrap();
        mgr.toggle_expand(node_id.as_str());
        let visible = mgr.generate_visible_nodes();
        if let Some(app) = app_weak_expand.upgrade() {
            let mapped = map_file_nodes(&visible);
            app.set_tree_items(Rc::new(slint::VecModel::from(mapped)).into());
        }
    });

    let app_weak_sel = app.as_weak();
    app.on_tree_node_selected(move |id, label, is_folder| {
        if let Some(app) = app_weak_sel.upgrade() {
            app.set_tree_selected_id(id.clone());
            app.set_tree_selected_label(label.clone());
            app.set_tree_selected_is_folder(is_folder);
            app.invoke_trigger_sample_toast(
                "info".into(),
                if is_folder { "Folder Selected".into() } else { "File Selected".into() },
                format!("Inspecting node: {}", label).into(),
            );
        }
    });

    let tree_mgr_refresh = tree_manager.clone();
    let app_weak_refresh = app.as_weak();
    app.on_tree_refresh_scan(move || {
        let mut mgr = tree_mgr_refresh.lock().unwrap();
        let cur_path = mgr.current_root_display();
        let _ = mgr.set_root(&cur_path);
        let visible = mgr.generate_visible_nodes();
        if let Some(app) = app_weak_refresh.upgrade() {
            let mapped = map_file_nodes(&visible);
            app.set_tree_items(Rc::new(slint::VecModel::from(mapped)).into());
            app.set_tree_current_path(mgr.current_root_display().into());
            app.invoke_trigger_sample_toast(
                "success".into(),
                "Filesystem Rescanned".into(),
                format!("Indexed {} files and folders in-process.", visible.len()).into(),
            );
        }
    });

    // Native in-process Folder Picker Dialog (runs asynchronously to maintain 120 FPS UI loop)
    let app_weak_browse = app.as_weak();
    let tree_mgr_browse = tree_manager.clone();
    app.on_tree_browse_folder(move || {
        let app_weak = app_weak_browse.clone();
        let tree_mgr = tree_mgr_browse.clone();
        std::thread::spawn(move || {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                let path_str = folder.to_string_lossy().to_string();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak.upgrade() {
                        let mut mgr = tree_mgr.lock().unwrap();
                        if let Ok(()) = mgr.set_root(&folder) {
                            let visible = mgr.generate_visible_nodes();
                            let mapped = map_file_nodes(&visible);
                            app.set_tree_items(Rc::new(slint::VecModel::from(mapped)).into());
                            app.set_tree_current_path(path_str.clone().into());
                            app.invoke_trigger_sample_toast(
                                "success".into(),
                                "Directory Loaded".into(),
                                format!("Loaded {} files and folders.", visible.len()).into(),
                            );
                        }
                    }
                });
            }
        });
    });

    // Load directory from manual path input
    let app_weak_load = app.as_weak();
    let tree_mgr_load = tree_manager.clone();
    app.on_tree_load_path(move |raw_path| {
        let trimmed = raw_path.trim();
        if trimmed.is_empty() { return; }
        let mut mgr = tree_mgr_load.lock().unwrap();
        match mgr.set_root(trimmed) {
            Ok(()) => {
                let visible = mgr.generate_visible_nodes();
                let mapped = map_file_nodes(&visible);
                if let Some(app) = app_weak_load.upgrade() {
                    app.set_tree_items(Rc::new(slint::VecModel::from(mapped)).into());
                    app.set_tree_current_path(mgr.current_root_display().into());
                    app.invoke_trigger_sample_toast(
                        "success".into(),
                        "Directory Loaded".into(),
                        format!("Indexed {} items in selected directory.", visible.len()).into(),
                    );
                }
            }
            Err(err) => {
                if let Some(app) = app_weak_load.upgrade() {
                    app.invoke_trigger_sample_toast(
                        "danger".into(),
                        "Invalid Directory".into(),
                        err.to_string().into(),
                    );
                }
            }
        }
    });

    // Quick jump directory shortcuts
    let app_weak_jump = app.as_weak();
    let tree_mgr_jump = tree_manager.clone();
    app.on_tree_quick_jump(move |target| {
        let target_str = target.as_str();
        let dest = match target_str {
            "desktop" => get_desktop_dir().ok(),
            "home" => std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).map(PathBuf::from).ok(),
            _ => Some(PathBuf::from(".")),
        };

        if let Some(dest_path) = dest {
            let mut mgr = tree_mgr_jump.lock().unwrap();
            if let Ok(()) = mgr.set_root(&dest_path) {
                let visible = mgr.generate_visible_nodes();
                let mapped = map_file_nodes(&visible);
                if let Some(app) = app_weak_jump.upgrade() {
                    app.set_tree_items(Rc::new(slint::VecModel::from(mapped)).into());
                    app.set_tree_current_path(mgr.current_root_display().into());
                    app.invoke_trigger_sample_toast(
                        "info".into(),
                        "Switched Directory".into(),
                        format!("Switched to {} ({} items).", target_str, visible.len()).into(),
                    );
                }
            }
        }
    });

    // 12. Native Desktop Shortcut Integration
    let app_name = "Native Desktop Design System";
    let shortcut_status = check_shortcut(app_name);
    match shortcut_status {
        ShortcutStatus::Installed(p) => {
            app.set_shortcut_installed(true);
            app.set_shortcut_path(p.to_string_lossy().to_string().into());
        }
        ShortcutStatus::NotInstalled => {
            app.set_shortcut_installed(false);
            app.set_shortcut_path("".into());
        }
    }

    let app_weak_create_sc = app.as_weak();
    app.on_create_desktop_shortcut(move || {
        let app_name = "Native Desktop Design System";
        match create_shortcut(
            app_name,
            None,
            None,
            Some("High-performance Native Rust Desktop Design System & Component Library"),
        ) {
            Ok(ShortcutResult::Created(p)) => {
                if let Some(app) = app_weak_create_sc.upgrade() {
                    app.set_shortcut_installed(true);
                    app.set_shortcut_path(p.to_string_lossy().to_string().into());
                    app.invoke_trigger_sample_toast(
                        "success".into(),
                        "Desktop Shortcut Created".into(),
                        "Launcher successfully placed on your Desktop.".into(),
                    );
                }
            }
            Ok(ShortcutResult::AlreadyExists(p)) => {
                if let Some(app) = app_weak_create_sc.upgrade() {
                    app.set_shortcut_installed(true);
                    app.set_shortcut_path(p.to_string_lossy().to_string().into());
                    app.invoke_trigger_sample_toast(
                        "info".into(),
                        "Shortcut Already Exists".into(),
                        "Desktop launcher is already present and valid.".into(),
                    );
                }
            }
            Err(err) => {
                if let Some(app) = app_weak_create_sc.upgrade() {
                    app.invoke_trigger_sample_toast(
                        "danger".into(),
                        "Shortcut Creation Failed".into(),
                        err.into(),
                    );
                }
            }
            _ => {}
        }
    });

    let app_weak_remove_sc = app.as_weak();
    app.on_remove_desktop_shortcut(move || {
        let app_name = "Native Desktop Design System";
        match remove_shortcut(app_name) {
            Ok(ShortcutResult::Removed(_p)) => {
                if let Some(app) = app_weak_remove_sc.upgrade() {
                    app.set_shortcut_installed(false);
                    app.set_shortcut_path("".into());
                    app.invoke_trigger_sample_toast(
                        "info".into(),
                        "Desktop Shortcut Removed".into(),
                        "Launcher cleanly removed from your Desktop.".into(),
                    );
                }
            }
            Ok(ShortcutResult::NotFound) => {
                if let Some(app) = app_weak_remove_sc.upgrade() {
                    app.set_shortcut_installed(false);
                    app.set_shortcut_path("".into());
                    app.invoke_trigger_sample_toast(
                        "info".into(),
                        "No Shortcut Found".into(),
                        "Desktop shortcut was not found.".into(),
                    );
                }
            }
            Err(err) => {
                if let Some(app) = app_weak_remove_sc.upgrade() {
                    app.invoke_trigger_sample_toast(
                        "danger".into(),
                        "Removal Failed".into(),
                        err.into(),
                    );
                }
            }
            _ => {}
        }
    });

    let app_weak_check_sc = app.as_weak();
    app.on_check_desktop_shortcut(move || {
        let app_name = "Native Desktop Design System";
        match check_shortcut(app_name) {
            ShortcutStatus::Installed(p) => {
                if let Some(app) = app_weak_check_sc.upgrade() {
                    app.set_shortcut_installed(true);
                    app.set_shortcut_path(p.to_string_lossy().to_string().into());
                }
            }
            ShortcutStatus::NotInstalled => {
                if let Some(app) = app_weak_check_sc.upgrade() {
                    app.set_shortcut_installed(false);
                    app.set_shortcut_path("".into());
                }
            }
        }
    });

    // 12. Connect Command Palette Spotlight Actions
    let db_tx_cmd = db_tx.clone();
    let app_weak_cmd = app.as_weak();
    app.on_command_palette_action(move |action_id| {
        if let Some(app) = app_weak_cmd.upgrade() {
            match action_id.as_str() {
                "toggle-theme" => {
                    let cur_dark = app.global::<Palette>().get_dark_mode();
                    let new_dark = !cur_dark;
                    app.global::<Palette>().set_dark_mode(new_dark);
                    let _ = db_tx_cmd.send(DbCmd::SetPreference {
                        key: "theme_mode".into(),
                        value: if new_dark { "dark".into() } else { "light".into() },
                    });
                    app.set_theme_mode_status(
                        if new_dark {
                            "Saved: Slate Dark Theme (Persistent)".into()
                        } else {
                            "Saved: Clean Light Theme (Persistent)".into()
                        },
                    );
                    app.invoke_trigger_sample_toast(
                        "success".into(),
                        "Theme Mode Updated".into(),
                        format!("Switched to {}", if new_dark { "Slate Dark" } else { "Clean Light" }).into(),
                    );
                }
                "open-pos" => {
                    app.invoke_trigger_sample_toast(
                        "info".into(),
                        "POS Retail Terminal".into(),
                        "Run 'cargo run --example pos_terminal' to launch the retail matrix.".into(),
                    );
                }
                "open-devforge" => {
                    app.invoke_trigger_sample_toast(
                        "info".into(),
                        "DevForge Studio".into(),
                        "Run 'cargo run --example dev_forge' to launch the developer utility studio.".into(),
                    );
                }
                "open-treeview" => {
                    app.set_selected_tab(0);
                    app.set_selected_tier_tab(2);
                    app.invoke_trigger_sample_toast(
                        "info".into(),
                        "Explorer Spotlight".into(),
                        "Navigated to TreeView File Explorer in Tier 3 Organisms.".into(),
                    );
                }
                "refresh-telemetry" => {
                    app.invoke_refresh_hardware_diagnostics();
                    app.invoke_trigger_sample_toast(
                        "success".into(),
                        "Hardware Telemetry".into(),
                        "Direct in-process hardware telemetry refreshed.".into(),
                    );
                }
                "create-shortcut" => {
                    app.invoke_create_desktop_shortcut();
                }
                other => {
                    app.invoke_trigger_sample_toast(
                        "info".into(),
                        "Action Dispatched".into(),
                        format!("Executed command: {other}").into(),
                    );
                }
            }
        }
    });

    // 13. Connect Accent Color Palette Persistence
    let db_tx_accent = db_tx.clone();
    let app_weak_accent = app.as_weak();
    app.on_accent_color_changed(move |hex_slint| {
        let hex_str = hex_slint.to_string();
        if let Some(col) = parse_hex_color(&hex_str) {
            if let Some(app) = app_weak_accent.upgrade() {
                app.global::<Palette>().set_accent(col);
                app.set_demo_color(col);
                app.set_demo_color_hex(hex_str.clone().into());
            }
        }
        let _ = db_tx_accent.send(DbCmd::SetPreference {
            key: "accent_color".into(),
            value: hex_str,
        });
    });

    // 14. Connect Live Command Palette Search Filter
    let all_commands = vec![
        CommandItem { id: "toggle-theme".into(), title: "Toggle Dark / Light Theme".into(), category: "Preferences".into(), shortcut: "Ctrl+T".into(), icon: "theme".into() },
        CommandItem { id: "open-pos".into(), title: "Launch POS Retail Terminal".into(), category: "Examples".into(), shortcut: "F8".into(), icon: "pos".into() },
        CommandItem { id: "open-devforge".into(), title: "Launch DevForge Developer Studio".into(), category: "Examples".into(), shortcut: "F9".into(), icon: "devforge".into() },
        CommandItem { id: "open-treeview".into(), title: "Scan Local Workspace Filesystem".into(), category: "Explorer".into(), shortcut: "Ctrl+E".into(), icon: "explorer".into() },
        CommandItem { id: "refresh-telemetry".into(), title: "Refresh 360° Hardware Telemetry".into(), category: "System".into(), shortcut: "F5".into(), icon: "telemetry".into() },
        CommandItem { id: "create-shortcut".into(), title: "Create Native Desktop Shortcut".into(), category: "System".into(), shortcut: "".into(), icon: "shortcut".into() },
    ];

    let app_weak_search = app.as_weak();
    let commands_ref = all_commands.clone();
    app.on_command_palette_search(move |query_slint| {
        let q = query_slint.trim().to_lowercase();
        let filtered: Vec<CommandItem> = if q.is_empty() {
            commands_ref.clone()
        } else {
            commands_ref
                .iter()
                .filter(|c| {
                    c.title.to_lowercase().contains(&q)
                        || c.category.to_lowercase().contains(&q)
                        || c.shortcut.to_lowercase().contains(&q)
                        || c.id.to_lowercase().contains(&q)
                })
                .cloned()
                .collect()
        };

        if let Some(app) = app_weak_search.upgrade() {
            let model = Rc::new(slint::VecModel::from(filtered));
            app.set_palette_commands(model.into());
        }
    });

    // Keep active timers alive for application lifespan
    let _timer_ref = (&timer, &toast_timer);

    // 14. Start the native desktop event loop
    app.run()
}

fn parse_hex_color(hex: &str) -> Option<slint::Color> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(slint::Color::from_argb_u8(255, r, g, b))
    } else {
        None
    }
}

fn map_file_nodes(nodes: &[FileNode]) -> Vec<TreeNodeData> {
    nodes
        .iter()
        .map(|n| TreeNodeData {
            id: n.id.clone().into(),
            label: n.label.clone().into(),
            icon: n.icon.clone().into(),
            level: n.level,
            is_folder: n.is_folder,
            expanded: n.expanded,
            has_children: n.has_children,
            badge: n.badge.clone().into(),
        })
        .collect()
}

fn apply_hardware_diagnostics(app: &AppWindow, diag: &SystemDiagnostics) {
    app.set_hw_os_name(format!("{} (Build {})", diag.os_name, diag.kernel_version).into());
    app.set_hw_host_name(diag.host_name.clone().into());
    app.set_hw_uptime(diag.uptime_str.clone().into());
    app.set_hw_app_memory(format!("{:.1} MB", diag.app_memory_mb).into());
    app.set_hw_cpu_brand(diag.cpu_brand.clone().into());
    app.set_hw_cpu_cores(diag.cpu_cores as i32);
    app.set_hw_cpu_usage(diag.cpu_usage_pct);
    app.set_hw_cpu_freq(format!("{:.1} GHz", (diag.cpu_frequency_mhz as f64) / 1000.0).into());
    app.set_hw_ram_total(format!("{:.1} GB", diag.ram_total_gb).into());
    app.set_hw_ram_used(format!("{:.1} GB", diag.ram_used_gb).into());
    app.set_hw_ram_free(format!("{:.1} GB", diag.ram_free_gb).into());
    app.set_hw_ram_usage(diag.ram_usage_pct);
    app.set_hw_gpu_name(diag.gpu_name.clone().into());
    app.set_hw_gpu_secondary(diag.gpu_secondary.clone().into());
    app.set_hw_graphics_backend(diag.graphics_backend.clone().into());
}
