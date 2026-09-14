# ArcHive Android — ProGuard / R8 keep rules
# Applied into gen/android/app/proguard-rules.pro by scripts/patch-android-ytdlp.ps1

# Custom Tauri plugin registered via JNI reflection
-keep class com.archhive.app.YtDlpPlugin { *; }
-keepclassmembers class com.archhive.app.YtDlpPlugin {
    <init>(android.app.Activity);
    *;
}

# Tauri plugin framework (annotations + Plugin base used by register_android_plugin)
-keep class app.tauri.** { *; }
-keep @app.tauri.annotation.TauriPlugin class * { *; }
-keepclassmembers class * {
    @app.tauri.annotation.Command *;
}

# youtubedl-android native/Python bridge
-keep class com.yausername.** { *; }
-dontwarn com.yausername.**
