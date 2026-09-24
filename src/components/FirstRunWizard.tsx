import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { api } from "@/lib/api/client";
import { useSettingsStore } from "@/lib/stores/settings";
import { getAppRuntime } from "@/lib/runtime";
import { isTauri } from "@/lib/tauri";
import type { AppSettings, EngineMode } from "@/lib/types";
import { WizardLayout } from "@/components/WizardLayout";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { FolderOpen, MonitorSmartphone, Cookie, Wrench, PartyPopper } from "lucide-react";

const TOTAL_STEPS = 5;

const STEP_TITLES = [
  "Welcome to ArcHive",
  "Library folder",
  "Download engine",
  "Connect your devices",
  "Media tools & demo",
];

const ENGINE_OPTIONS: { mode: EngineMode; title: string; desc: string }[] = [
  {
    mode: "local",
    title: "Local",
    desc: "Full power on this device: yt-dlp, gallery-dl and ffmpeg run here.",
  },
  {
    mode: "remote_lan",
    title: "Remote LAN",
    desc: "Use a desktop ArcHive as the engine. Best for phones and tablets.",
  },
  {
    mode: "standalone",
    title: "Standalone",
    desc: "On-device only (YouTube + direct links). No desktop needed.",
  },
];

/** Persist a settings patch both locally and on the backend (runtime-aware). */
async function persistWizardSettings(partial: Partial<AppSettings>): Promise<void> {
  const store = useSettingsStore.getState();
  store.updateSettings(partial);
  if (!isTauri()) return;
  const merged = { ...useSettingsStore.getState().settings, ...partial };
  try {
    if (getAppRuntime() === "mobile-tauri") {
      await api.saveDeviceSettings(merged);
    } else {
      await api.saveSettings(merged);
    }
  } catch {
    /* backend unreachable — local store keeps the choice */
  }
}

export function FirstRunWizardGate() {
  const hydrated = useSettingsStore((s) => s.hydrated);
  const wizardCompleted = useSettingsStore((s) => s.settings.wizard_completed);
  const [open, setOpen] = useState(true);

  if (!hydrated || wizardCompleted || !open) return null;
  return <FirstRunWizard onDone={() => setOpen(false)} />;
}

export function FirstRunWizard({ onDone }: { onDone: () => void }) {
  const navigate = useNavigate();
  const settings = useSettingsStore((s) => s.settings);
  const [step, setStep] = useState(0);
  const [libraryPath, setLibraryPath] = useState(settings.library_path);
  const [engineMode, setEngineMode] = useState<EngineMode>(settings.engine_mode);
  const [resolvingDefault, setResolvingDefault] = useState(false);
  const [toolsStatus, setToolsStatus] = useState("");
  const [checkingTools, setCheckingTools] = useState(false);
  const [demoStatus, setDemoStatus] = useState("");
  const [loadingDemo, setLoadingDemo] = useState(false);
  const runtime = getAppRuntime();

  async function finish() {
    await persistWizardSettings({
      library_path: libraryPath,
      engine_mode: engineMode,
      wizard_completed: true,
    });
    onDone();
  }

  function skip() {
    void persistWizardSettings({ wizard_completed: true }).then(onDone);
  }

  async function resolveDefaultLibraryDir() {
    setResolvingDefault(true);
    try {
      const dir = await api.defaultLibraryDir();
      setLibraryPath(dir);
    } catch (e) {
      setToolsStatus(e instanceof Error ? e.message : "Could not resolve default folder");
    } finally {
      setResolvingDefault(false);
    }
  }

  async function checkMediaTools() {
    setCheckingTools(true);
    setToolsStatus("Checking media tools…");
    try {
      const [ffmpeg, versions] = await Promise.all([
        api.ffmpegStatus(),
        api.binaryVersions().catch(() => null),
      ]);
      const parts: string[] = [];
      parts.push(ffmpeg.ffmpeg_available ? "ffmpeg OK" : "ffmpeg missing");
      parts.push(ffmpeg.ffprobe_available ? "ffprobe OK" : "ffprobe missing");
      if (versions?.ytdlp_version) parts.push(`yt-dlp ${versions.ytdlp_version}`);
      else parts.push("yt-dlp missing");
      setToolsStatus(parts.join(" · "));
    } catch (e) {
      setToolsStatus(e instanceof Error ? e.message : "Tool check failed");
    } finally {
      setCheckingTools(false);
    }
  }

  async function loadDemo() {
    setLoadingDemo(true);
    setDemoStatus("Queueing demo download…");
    try {
      const msg = await api.loadDemoScene();
      setDemoStatus(msg);
    } catch (e) {
      setDemoStatus(e instanceof Error ? e.message : "Demo download failed");
    } finally {
      setLoadingDemo(false);
    }
  }

  function openSettings(tab: string) {
    void finish().then(() => {
      void navigate({ to: "/settings", search: { tab } });
    });
  }

  const canNext = step === 1 ? libraryPath.trim().length > 0 : true;
  const isLast = step === TOTAL_STEPS - 1;

  return (
    <WizardLayout
      step={step}
      totalSteps={TOTAL_STEPS}
      title={STEP_TITLES[step]}
      subtitle="First-time setup — you can change everything later in Settings."
      onBack={() => setStep((s) => Math.max(0, s - 1))}
      onNext={() => {
        if (isLast) {
          void finish();
          return;
        }
        if (step === 1) {
          void persistWizardSettings({ library_path: libraryPath });
        }
        if (step === 2) {
          void persistWizardSettings({ engine_mode: engineMode });
        }
        setStep((s) => Math.min(TOTAL_STEPS - 1, s + 1));
      }}
      onSkip={skip}
      nextLabel={isLast ? "Finish" : "Next"}
      canNext={canNext}
      isLast={isLast}
    >
      {step === 0 && (
        <div className="space-y-3 text-sm">
          <p className="flex items-center gap-2 text-base font-medium">
            <PartyPopper className="h-5 w-5 text-[var(--color-primary)]" />
            Browse, download and organize media from 15+ sites.
          </p>
          <p className="text-[var(--color-muted-foreground)]">
            This wizard takes about a minute: pick a library folder, choose how downloads run (
            {runtime === "mobile-tauri" ? "on this phone" : "on this device"}), connect your other
            devices, and verify the media tools.
          </p>
          <ul className="list-disc space-y-1 pl-5 text-[var(--color-muted-foreground)]">
            <li>Library folder — where your downloads live</li>
            <li>Engine mode — local, remote LAN, or standalone</li>
            <li>Device pairing — phone + desktop over your Wi-Fi</li>
            <li>Cookies — for sites that need a login</li>
            <li>Media tools check — ffmpeg, yt-dlp, gallery-dl</li>
          </ul>
        </div>
      )}

      {step === 1 && (
        <div className="space-y-3 text-sm">
          <p className="flex items-center gap-2 font-medium">
            <FolderOpen className="h-4 w-4 text-[var(--color-primary)]" />
            Where should downloads be saved?
          </p>
          <Input
            value={libraryPath}
            onChange={(e) => setLibraryPath(e.target.value)}
            placeholder="/path/to/ArcHive"
            aria-label="Library folder"
          />
          <div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => void resolveDefaultLibraryDir()}
              disabled={resolvingDefault}
            >
              {resolvingDefault ? "Resolving…" : "Use default folder"}
            </Button>
          </div>
          <p className="text-xs text-[var(--color-muted-foreground)]">
            {runtime === "mobile-tauri"
              ? "On Android this is an app-private folder. You can change it later in Settings → Library."
              : "Desktop default is your Videos/ArcHive folder. Thumbnails are stored next to the videos."}
          </p>
        </div>
      )}

      {step === 2 && (
        <div className="space-y-2 text-sm">
          {ENGINE_OPTIONS.map((opt) => (
            <button
              key={opt.mode}
              type="button"
              onClick={() => setEngineMode(opt.mode)}
              aria-pressed={engineMode === opt.mode}
              className={`w-full rounded-md border px-3 py-2 text-left transition-colors ${
                engineMode === opt.mode
                  ? "border-[var(--color-primary)] bg-[var(--color-primary)]/10"
                  : "border-[var(--color-border)] hover:bg-[var(--color-muted)]"
              }`}
            >
              <p className="font-medium">{opt.title}</p>
              <p className="text-xs text-[var(--color-muted-foreground)]">{opt.desc}</p>
            </button>
          ))}
        </div>
      )}

      {step === 3 && (
        <div className="space-y-3 text-sm">
          <p className="flex items-center gap-2 font-medium">
            <MonitorSmartphone className="h-4 w-4 text-[var(--color-primary)]" />
            Pair your phone and desktop over LAN
          </p>
          <p className="text-[var(--color-muted-foreground)]">
            Enable the LAN server on your desktop, then scan the QR code from the mobile app. The
            server streams your library and accepts download jobs from your phone.
          </p>
          <p className="flex items-start gap-2 text-[var(--color-muted-foreground)]">
            <Cookie className="mt-0.5 h-4 w-4 shrink-0" />
            Sites that need a login (cookies) are configured per-site in Settings → Engine →
            Cookies.
          </p>
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" onClick={() => openSettings("engine")}>
              Open Engine settings
            </Button>
            <Button variant="outline" size="sm" onClick={() => openSettings("lan")}>
              Open LAN settings
            </Button>
          </div>
        </div>
      )}

      {step === 4 && (
        <div className="space-y-3 text-sm">
          <p className="flex items-center gap-2 font-medium">
            <Wrench className="h-4 w-4 text-[var(--color-primary)]" />
            Verify downloads will work
          </p>
          <div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => void checkMediaTools()}
              disabled={checkingTools}
            >
              {checkingTools ? "Checking…" : "Check media tools"}
            </Button>
          </div>
          {toolsStatus && (
            <p className="text-xs text-[var(--color-muted-foreground)]">{toolsStatus}</p>
          )}
          <div className="border-t border-[var(--color-border)] pt-3">
            <p className="font-medium">Try it out</p>
            <p className="text-xs text-[var(--color-muted-foreground)]">
              Queue a small public-domain test video to see the full download → library pipeline in
              action.
            </p>
            <div className="mt-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => void loadDemo()}
                disabled={loadingDemo}
              >
                {loadingDemo ? "Queueing…" : "Load demo scene"}
              </Button>
            </div>
            {demoStatus && (
              <p className="mt-1 text-xs text-[var(--color-muted-foreground)]">{demoStatus}</p>
            )}
          </div>
        </div>
      )}
    </WizardLayout>
  );
}
