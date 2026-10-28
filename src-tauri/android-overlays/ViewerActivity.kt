package com.archhive.app

import android.os.Bundle

/**
 * Hosts ArcHive in-app viewer windows (secondary Tauri WebviewWindows).
 *
 * Tauri's Android multiwindow support launches one activity per extra
 * window; the activity name is passed via WebviewWindowBuilder's
 * `activity_name("ViewerActivity")` (see src-tauri/src/viewer.rs).
 * Activity Embedding split rules live in res/xml/main_split_config.xml
 * and are registered by SplitInitializer at app startup.
 */
class ViewerActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
    }
}
