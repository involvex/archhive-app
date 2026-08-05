use crate::models::AppSettings;
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime, Window,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub struct TrayHotkeyState {
    registered: Mutex<Option<Shortcut>>,
    was_minimized: Mutex<bool>,
}

impl TrayHotkeyState {
    pub fn new() -> Self {
        Self {
            registered: Mutex::new(None),
            was_minimized: Mutex::new(false),
        }
    }
}

pub fn tray_settings_changed(prev: &AppSettings, next: &AppSettings) -> bool {
    prev.close_to_tray != next.close_to_tray
        || prev.minimize_to_tray != next.minimize_to_tray
        || prev.tray_hotkey != next.tray_hotkey
}

const MAIN_WINDOW: &str = "main";
const MENU_SHOW: &str = "tray_show";
const MENU_SETTINGS: &str = "tray_settings";
const MENU_QUIT: &str = "tray_quit";

pub fn setup<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let icon = app
        .default_window_icon()
        .ok_or_else(|| "missing default window icon".to_string())?
        .clone();

    let show_item = MenuItem::with_id(app, MENU_SHOW, "Show ArcHive", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let settings_item = MenuItem::with_id(app, MENU_SETTINGS, "Settings", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let quit_item =
        MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>).map_err(|e| e.to_string())?;
    let menu = Menu::with_items(app, &[&show_item, &settings_item, &quit_item])
        .map_err(|e| e.to_string())?;

    TrayIconBuilder::new()
        .icon(icon)
        .tooltip("ArcHive")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_SHOW => show_main_window(app, false),
            MENU_SETTINGS => show_main_window(app, true),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn sync_from_settings<R: Runtime>(app: &AppHandle<R>, settings: &AppSettings) {
    if let Err(e) = sync_hotkey(app, settings) {
        eprintln!("tray hotkey sync failed: {e}");
    }
}

pub fn on_window_event<R: Runtime>(window: &Window<R>, event: &tauri::WindowEvent) {
    // Only process events for the main window — secondary webview windows
    // (Chaturbate bridge, etc.) must not trigger tray behavior.
    if window.label() != MAIN_WINDOW {
        return;
    }

    let app = window.app_handle();
    let Ok(settings) = app.state::<Arc<crate::state::AppState>>().get_settings() else {
        return;
    };
    let hotkey_state = app.state::<Arc<TrayHotkeyState>>();

    match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            if settings.close_to_tray {
                api.prevent_close();
                hide_main_window(app);
            }
        }
        tauri::WindowEvent::Resized(_) => {
            if !settings.minimize_to_tray {
                return;
            }
            let is_minimized = window.is_minimized().unwrap_or(false);
            let mut was_minimized = hotkey_state.was_minimized.lock();
            if is_minimized && !*was_minimized {
                // Only hide on the transition non-minimized → minimized.
                // Resized fires for DPI changes, virtual desktop switches,
                // taskbar show/hide, and other non-user events where
                // is_minimized() may transiently return true.
                hide_main_window(app);
                let _ = window.unminimize();
            }
            *was_minimized = is_minimized;
        }
        _ => {}
    }
}

fn sync_hotkey<R: Runtime>(app: &AppHandle<R>, settings: &AppSettings) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    let state = app.state::<Arc<TrayHotkeyState>>();

    if let Some(prev) = state.registered.lock().take() {
        let _ = shortcuts.unregister(prev);
    }

    let hotkey = settings
        .tray_hotkey
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    let Some(hotkey) = hotkey else {
        return Ok(());
    };

    let shortcut = hotkey
        .parse::<Shortcut>()
        .map_err(|e| format!("invalid hotkey '{hotkey}': {e}"))?;

    if shortcuts.is_registered(shortcut) {
        let _ = shortcuts.unregister(shortcut);
    }

    shortcuts
        .on_shortcut(shortcut, |app, _shortcut, event| {
            use tauri_plugin_global_shortcut::ShortcutState;
            if event.state == ShortcutState::Pressed {
                toggle_main_window(app);
            }
        })
        .map_err(|e| e.to_string())?;

    *state.registered.lock() = Some(shortcut);
    Ok(())
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>, open_settings: bool) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.set_skip_taskbar(false);
    if open_settings {
        let _ = window.emit("app-navigate", "/settings");
    }
}

fn hide_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let _ = window.hide();
    let _ = window.set_skip_taskbar(true);
}

fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    if window.is_visible().unwrap_or(true) {
        hide_main_window(app);
    } else {
        show_main_window(app, false);
    }
}
