# Patches generated Android project for multi-window (in-app viewer) support.
# Adds ViewerActivity + SplitInitializer (androidx.startup), the Activity
# Embedding split rules (res/xml/main_split_config.xml), the
# PROPERTY_ACTIVITY_EMBEDDING_SPLITS_ENABLED manifest property, and the
# androidx.window / androidx.startup dependencies.
# Required for Tauri mobile multiwindow: https://v2.tauri.app/learn/mobile-multiwindow/
# Run after `tauri android init` / android:regen (gen/android is gitignored).
# Idempotent: safe to re-run via apply-android-patches.ps1.
param(
    [string]$GenAndroid = (Join-Path $PSScriptRoot "..\src-tauri\gen\android"),
    [string]$Overlays = (Join-Path $PSScriptRoot "..\src-tauri\android-overlays")
)

$ErrorActionPreference = "Stop"

$gen = $GenAndroid
$overlayDir = $Overlays
$manifestPath = Join-Path $gen "app\src\main\AndroidManifest.xml"
$gradlePath = Join-Path $gen "app\build.gradle.kts"
$ktDestDir = Join-Path $gen "app\src\main\java\com\archhive\app"
$xmlDestDir = Join-Path $gen "app\src\main\res\xml"

if (-not (Test-Path $gradlePath)) {
    Write-Warning "Android project not found at $gen. Run: bun run tauri android init"
    exit 1
}

# 1. Kotlin sources: ViewerActivity (secondary webview host). SplitInitializer no longer used.
$ktSources = @("ViewerActivity.kt")
foreach ($name in $ktSources) {
    $src = Join-Path $overlayDir $name
    if (-not (Test-Path $src)) {
        throw "Missing overlay: $src"
    }
    New-Item -ItemType Directory -Force -Path $ktDestDir | Out-Null
    Copy-Item -Path $src -Destination (Join-Path $ktDestDir $name) -Force
}
# Remove old SplitInitializer if exists
$splitInit = Join-Path $ktDestDir "SplitInitializer.kt"
if (Test-Path $splitInit) {
    Remove-Item $splitInit -Force
}
Write-Host "Installed ViewerActivity.kt"

# 2. Split rules resource (MainActivity | ViewerActivity pair).
$splitConfigSrc = Join-Path $overlayDir "main_split_config.xml"
if (-not (Test-Path $splitConfigSrc)) {
    throw "Missing overlay: $splitConfigSrc"
}
New-Item -ItemType Directory -Force -Path $xmlDestDir | Out-Null
Copy-Item -Path $splitConfigSrc -Destination (Join-Path $xmlDestDir "main_split_config.xml") -Force
Write-Host "Installed res/xml/main_split_config.xml"

# 2b. Patch MainActivity to initialize split rules.
$mainActivityPath = Join-Path $ktDestDir "MainActivity.kt"
if (Test-Path $mainActivityPath) {
    $mainAct = Get-Content $mainActivityPath -Raw
    if ($mainAct -notmatch 'RuleController') {
        # Add imports
        if ($mainAct -notmatch 'androidx.window.core.ExperimentalWindowApi') {
            $mainAct = $mainAct -replace '(import androidx.activity.enableEdgeToEdge)', "`$1`r`nimport androidx.window.core.ExperimentalWindowApi`r`nimport androidx.window.embedding.RuleController"
        }
        # Add annotation and initialization
        if ($mainAct -notmatch '@OptIn.*ExperimentalWindowApi') {
            $mainAct = $mainAct -replace '(class MainActivity)', "@OptIn(ExperimentalWindowApi::class)`r`n`$1"
        }
        # Add initialization in onCreate before super.onCreate
        $initCode = "    // Initialize Activity Embedding split rules for in-app viewer`r`n    RuleController.getInstance(this).apply {`r`n      setRules(RuleController.parseRules(this@MainActivity, R.xml.main_split_config))`r`n    }`r`n"
        $mainAct = $mainAct -replace '(override fun onCreate\(savedInstanceState: Bundle\?\) \{)', "`$1`r`n" + $initCode
        Set-Content -Path $mainActivityPath -Value $mainAct -NoNewline
        Write-Host "Patched MainActivity to initialize split rules."
    } else {
        Write-Host "MainActivity already initializes split rules."
    }
}

# 3. Gradle: androidx.window (Activity Embedding) + androidx.startup.
$gradle = Get-Content $gradlePath -Raw
$needsWindow = $gradle -notmatch 'androidx\.window:window'
$needsStartup = $gradle -notmatch 'androidx\.startup:startup-runtime'
if ($needsWindow -or $needsStartup) {
    $anchor = 'dependencies {'
    $idx = $gradle.IndexOf($anchor)
    if ($idx -lt 0) {
        throw "Could not find dependencies block in $gradlePath"
    }
    $lines = @()
    if ($needsWindow) {
        $lines += '    // Multi-window (in-app viewer): Activity Embedding + androidx.startup'
        $lines += '    implementation("androidx.window:window:1.5.0")'
    }
    if ($needsStartup) {
        $lines += '    implementation("androidx.startup:startup-runtime:1.2.0")'
    }
    $insert = "dependencies {`r`n" + ($lines -join "`r`n")
    $gradle = $gradle.Substring(0, $idx) + $insert + $gradle.Substring($idx + $anchor.Length)
    Set-Content -Path $gradlePath -Value $gradle -NoNewline
    Write-Host "Patched build.gradle.kts with multiwindow dependencies."
} else {
    Write-Host "Multiwindow gradle dependencies already present."
}

# 4. Manifest: tools namespace + embedding property + ViewerActivity.
if (-not (Test-Path $manifestPath)) {
    throw "Missing manifest: $manifestPath"
}
$manifest = Get-Content $manifestPath -Raw
$manifestChanged = $false

# 4a. xmlns:tools namespace (required for tools:node="merge" below).
if ($manifest -notmatch 'xmlns:tools=') {
    $manifest = $manifest -replace '(<manifest\b[^>]*?)\s*>', "`$1`r`n          xmlns:tools=`"http://schemas.android.com/tools`">"
    $manifestChanged = $true
    Write-Host "Added xmlns:tools namespace to AndroidManifest."
} else {
    Write-Host "xmlns:tools namespace already present."
}

# 4b. Enable Activity Embedding on the <application> tag.
if ($manifest -notmatch 'PROPERTY_ACTIVITY_EMBEDDING_SPLITS_ENABLED') {
    $appIdx = $manifest.IndexOf('<application')
    if ($appIdx -lt 0) {
        throw "Could not find <application> in $manifestPath"
    }
    $tagEnd = $manifest.IndexOf('>', $appIdx)
    if ($tagEnd -lt 0) {
        throw "Could not find end of <application> tag in $manifestPath"
    }
    $property = "`r`n        <property`r`n            android:name=`"android.window.PROPERTY_ACTIVITY_EMBEDDING_SPLITS_ENABLED`"`r`n            android:value=`"true`" />"
    $manifest = $manifest.Substring(0, $tagEnd + 1) + $property + $manifest.Substring($tagEnd + 1)
    $manifestChanged = $true
    Write-Host "Added activity-embedding property to AndroidManifest."
} else {
    Write-Host "Activity-embedding property already present."
}

# 4c. ViewerActivity declaration (before </application>).
$appClose = '</application>'
$closeIdx = $manifest.IndexOf($appClose)
if ($closeIdx -lt 0) {
    throw "Could not find </application> in $manifestPath"
}

$preClose = $manifest.Substring(0, $closeIdx)
$postClose = $manifest.Substring($closeIdx)

if ($preClose -notmatch 'android:name="\.ViewerActivity"') {
    $preClose += @'

        <!-- In-app viewer window (secondary Tauri WebviewWindow). -->
        <activity
            android:name=".ViewerActivity"
            android:exported="true" />
'@
    $manifestChanged = $true
    Write-Host "Added .ViewerActivity declaration to AndroidManifest."
} else {
    Write-Host "ViewerActivity declaration already present."
}

# Remove broken InitializationProvider if present (we initialize in MainActivity now)
if ($preClose -match 'androidx\.startup\.InitializationProvider') {
    # Remove the provider block
    $pattern = '(?s)\s*<!--\s*Registers SplitInitializer[^\>]*?-->\s*<provider[^>]*androidx\.startup\.InitializationProvider[^>]*>.*?</provider>'
    $newPreClose = $preClose -replace $pattern, ''
    if ($newPreClose -ne $preClose) {
        $preClose = $newPreClose
        $manifestChanged = $true
        Write-Host "Removed InitializationProvider (initialized in MainActivity)."
    }
}

$manifest = $preClose + $postClose
if ($manifestChanged) {
    Set-Content -Path $manifestPath -Value $manifest -NoNewline
}

Write-Host "Android multiwindow patch complete."
