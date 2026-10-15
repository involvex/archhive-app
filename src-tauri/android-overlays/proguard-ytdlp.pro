# ArcHive Android — ProGuard / R8 keep rules
# Applied into gen/android/app/proguard-rules.pro by scripts/patch-android-ytdlp.ps1

# Custom Tauri plugin registered via JNI reflection
-keep class com.archhive.app.YtDlpPlugin { *; }
-keepclassmembers class com.archhive.app.YtDlpPlugin {
    <init>(android.app.Activity);
    *;
}
-keep class com.archhive.app.DownloadForegroundService { *; }
-keep class com.archhive.app.PendingResumeWorker { *; }
-keepclassmembers class com.archhive.app.PendingResumeWorker {
    <init>(android.content.Context, androidx.work.WorkerParameters);
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

# Apache Commons Compress — youtubedl-android's ZipUtils unzips the Python
# runtime via commons-compress ZipFile, whose ExtraFieldUtils static registry
# reflectively instantiates ZipExtraField impls (AsiExtraField, ...). R8 must
# not strip or merge those classes/constructors: release boot otherwise dies
# with "class ...AsiExtraField is not a concrete class" (seen v0.6.0, OnePlus 7T).
-keep class org.apache.commons.compress.** { *; }
-dontwarn org.apache.commons.compress.**
