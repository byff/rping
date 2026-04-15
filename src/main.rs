// Prevent console window in addition to Slint window in Windows release builds.
#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod config;
mod ping;
mod gui;
mod excel;
mod utils;

use std::cell::RefCell;
use std::rc::Rc;
use slint::Weak;

slint::include_modules!();

fn main() {
    // Init logging
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).format_timestamp_millis().try_init();

    log::info!("PingTest starting (Slint GUI)...");

    // Create main window
    let window = MainWindow::new()
        .expect("Failed to create MainWindow");

    // Create window weak reference wrapped in RefCell for interior mutability
    let window_rc: Rc<RefCell<Option<Weak<MainWindow>>>> =
        Rc::new(RefCell::new(Some(window.as_weak())));

    // Create app state with window reference
    let app = Rc::new(RefCell::new(gui::app::PingTestApp::new(window_rc.clone())));

    // Start the timer with the window reference
    app.borrow_mut().start_timer();

    // Set up callbacks
    {
        let app_clone = app.clone();
        window.on_start_ping(move || {
            app_clone.borrow_mut().start_ping();
        });
    }
    {
        let app_clone = app.clone();
        window.on_stop_ping(move || {
            app_clone.borrow_mut().stop_ping();
        });
    }
    {
        let app_clone = app.clone();
        window.on_refresh_results(move || {
            app_clone.borrow_mut().refresh_ping();
        });
    }
    {
        let app_clone = app.clone();
        window.on_import_ips(move || {
            app_clone.borrow_mut().import_file();
        });
    }
    {
        let app_clone = app.clone();
        window.on_export_results(move || {
            app_clone.borrow_mut().export_results();
        });
    }
    {
        let app_clone = app.clone();
        window.on_insert_to_source(move || {
            app_clone.borrow_mut().export_to_source_excel();
        });
    }

    window.on_open_settings(move || {});
    window.on_open_about(move || {});

    {
        let app_clone = app.clone();
        window.on_close_settings(move || {
            app_clone.borrow_mut().close_settings();
        });
    }
    {
        let app_clone = app.clone();
        window.on_save_settings(move || {
            app_clone.borrow_mut().save_settings_from_window();
        });
    }
    {
        let app_clone = app.clone();
        window.on_reset_settings(move || {
            app_clone.borrow_mut().reset_settings();
        });
    }

    window.on_ip_warning_continue(move || {
        // User confirmed IP warning - restart ping
    });
    window.on_ip_warning_cancel(move || {});

    {
        let app_clone = app.clone();
        window.on_close_about(move || {});
    }

    {
        let app_clone = app.clone();
        window.on_sort_table(move |column| {
            app_clone.borrow_mut().sort_table(column);
        });
    }

    // Run the event loop
    window.run().expect("Failed to run window");
}
