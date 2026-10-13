# Patches generated Android project: cleartext HTTP + media storage permissions.
# NOTE: WebView userAgent + bundle.android.minSdkVersion now live declaratively
# in src-tauri/tauri.android.conf.json (auto-merged by the Tauri CLI). Only
# manifest/gradle entries with no config equivalent are patched here.
param(
    [string]$GradlePath = (Join-Path $PSScriptRoot "..\src-tauri\gen\android\app\build.gradle.kts"),
    [string]$ManifestPath = (Join-Path $PSScriptRoot "..\src-tauri\gen\android\app\src\main\AndroidManifest.xml")
)

if (-not (Test-Path $GradlePath)) {
    Write-Warning "Android project not found. Run: bun run tauri android init"
    exit 1
}

$content = Get-Content $GradlePath -Raw
# The generated defaultConfig sets "false" and only the debug build type sets
# "true", so test for a remaining "false" (not for any "true"): otherwise
# release APKs keep cleartext off and the WebView cannot reach the on-device
# 127.0.0.1 media server or a Remote LAN host.
if ($content -match 'manifestPlaceholders\["usesCleartextTraffic"\] = "false"') {
    $content = $content -replace 'manifestPlaceholders\["usesCleartextTraffic"\] = "false"', 'manifestPlaceholders["usesCleartextTraffic"] = "true"'
    Set-Content -Path $GradlePath -Value $content -NoNewline
    Write-Host "Patched $GradlePath for cleartext HTTP (loopback media server + Remote LAN)."
} else {
    Write-Host "Android LAN cleartext patch already applied."
}

if (Test-Path $ManifestPath) {
    $manifest = Get-Content $ManifestPath -Raw
    if ($manifest -notmatch 'READ_MEDIA_VIDEO') {
        $manifest = $manifest -replace '(<uses-permission android:name="android.permission.INTERNET" />)', @'
$1
    <uses-permission android:name="android.permission.CHANGE_WIFI_MULTICAST_STATE" />
    <uses-permission android:name="android.permission.READ_MEDIA_VIDEO" />
    <uses-permission android:name="android.permission.READ_MEDIA_IMAGES" />
    <uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" android:maxSdkVersion="32" />
'@
        Set-Content -Path $ManifestPath -Value $manifest -NoNewline
        Write-Host "Patched $ManifestPath with mDNS and media storage permissions."
    } elseif ($manifest -notmatch 'CHANGE_WIFI_MULTICAST_STATE') {
        $manifest = $manifest -replace '(<uses-permission android:name="android.permission.INTERNET" />)', @'
$1
    <uses-permission android:name="android.permission.CHANGE_WIFI_MULTICAST_STATE" />
'@
        Set-Content -Path $ManifestPath -Value $manifest -NoNewline
        Write-Host "Patched $ManifestPath with CHANGE_WIFI_MULTICAST_STATE for mDNS."
    } else {
        Write-Host "Android storage permissions already present."
    }

    # All files access: lets ArcHive delete library files in shared storage
    # (e.g. Download/ArcHive) it did not create in this install. Granted by the
    # user in system settings; WRITE_EXTERNAL_STORAGE covers Android 10 and older.
    if ($manifest -notmatch 'MANAGE_EXTERNAL_STORAGE') {
        $manifest = $manifest -replace '(<uses-permission android:name="android.permission.INTERNET" />)', @'
$1
    <uses-permission android:name="android.permission.MANAGE_EXTERNAL_STORAGE" />
    <uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE" android:maxSdkVersion="29" />
'@
        Set-Content -Path $ManifestPath -Value $manifest -NoNewline
        Write-Host "Patched $ManifestPath with all-files access permissions."
    } else {
        Write-Host "Android all-files access permission already present."
    }

    # Enable Activity PiP so HTML video.requestPictureInPicture can float.
    if ($manifest -notmatch 'supportsPictureInPicture') {
        if ($manifest -match 'android:configChanges="[^"]*"') {
            $manifest = $manifest -replace '(android:configChanges="[^"]*")', '$1 android:supportsPictureInPicture="true" android:resizeableActivity="true"'
        } else {
            $manifest = $manifest -replace '(<activity\b[^>]*)(>)', '$1 android:supportsPictureInPicture="true" android:resizeableActivity="true"$2'
        }
        Set-Content -Path $ManifestPath -Value $manifest -NoNewline
        Write-Host "Patched $ManifestPath for Picture-in-Picture."
    } else {
        Write-Host "Android Picture-in-Picture already enabled."
    }

    # Foreground service + WorkManager resume for background downloads
    if ($manifest -notmatch 'FOREGROUND_SERVICE_DATA_SYNC') {
        $manifest = $manifest -replace '(<uses-permission android:name="android.permission.INTERNET" />)', @'
$1
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE_DATA_SYNC" />
    <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
'@
        Set-Content -Path $ManifestPath -Value $manifest -NoNewline
        Write-Host "Patched $ManifestPath with foreground service permissions."
    }

    if ($manifest -notmatch 'DownloadForegroundService') {
        if ($manifest -match '</application>') {
            $serviceBlock = @'
    <service
        android:name="com.archhive.app.DownloadForegroundService"
        android:exported="false"
        android:foregroundServiceType="dataSync" />
'@
            $manifest = $manifest -replace '</application>', "$serviceBlock`r`n  </application>"
            Set-Content -Path $ManifestPath -Value $manifest -NoNewline
            Write-Host "Registered DownloadForegroundService in AndroidManifest."
        }
    }
}
