mod tray;

pub use tray::{
    on_window_event, setup, sync_from_settings, tray_settings_changed, TrayHotkeyState,
};
