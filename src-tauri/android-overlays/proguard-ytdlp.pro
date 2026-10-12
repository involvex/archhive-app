# ArcHive Android — ProGuard / R8 keep rules
# Applied into gen/android/app/proguard-rules.pro by scripts/patch-android-ytdlp.ps1

# Custom Tauri plugin registered via JNI reflection
-keep class com.archhive.app.YtDlpPlugin { *; }
-keepclassmembers class com.archhive.app.YtDlpPlugin {
    <init>(android.app.Activity);
    *;
}
-keep class com.archhive.app.DownloadForegroundService { *; }
-keep class com.archhive.app.CookieBrowserPlugin { *; }
-keep class com.archhive.app.CookieBrowserPlugin$OpenArgs { *; }
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
# ...and the libraries it drives reflectively during YoutubeDL.init /
# FFmpeg.init (zip unpacking, JSON). R8 renaming these crashed release builds.
-keep class org.apache.commons.compress.** { *; }
-keep class org.apache.commons.io.** { *; }
-keep class com.fasterxml.jackson.** { *; }
-keepattributes Signature,*Annotation*,InnerClasses,EnclosingMethod
-dontwarn org.apache.commons.compress.**
-dontwarn org.apache.commons.io.**
-dontwarn com.fasterxml.jackson.**
