import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import { getAppRuntime } from "@/lib/runtime";

/**
 * Ask the Android shell for an immersive window (system bars hidden, optional
 * landscape lock). Fire-and-forget; a no-op outside the mobile app.
 */
async function setNativeImmersive(enabled: boolean, landscape: boolean): Promise<void> {
  if (getAppRuntime() !== "mobile-tauri") return;
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("set_player_fullscreen", { enabled, landscape });
  } catch (e) {
    console.warn("[fullscreen] immersive mode unavailable", e);
  }
}

/** True when the HTML Fullscreen API actually works here (not Android's WebView). */
function nativeFullscreenWorks(): boolean {
  return (
    getAppRuntime() !== "mobile-tauri" &&
    typeof document !== "undefined" &&
    Boolean(document.fullscreenEnabled)
  );
}

/**
 * Video fullscreen that also works in the Android app. Tauri's Android
 * WebView cancels HTML5 fullscreen (wry's onShowCustomView), so there the
 * player container is expanded to cover the screen ("app fullscreen") and
 * the shell hides the system bars and rotates to landscape for wide video.
 * Elsewhere the standard Fullscreen API is used.
 *
 * The Android back button exits app fullscreen (a history entry is pushed on
 * enter and consumed on exit).
 */
export function usePlayerFullscreen(containerRef: RefObject<HTMLElement | null>) {
  const [appFullscreen, setAppFullscreen] = useState(false);
  const [nativeFullscreen, setNativeFullscreen] = useState(false);
  const pushedHistory = useRef(false);

  useEffect(() => {
    function onChange() {
      setNativeFullscreen(
        document.fullscreenElement != null && document.fullscreenElement === containerRef.current,
      );
    }
    document.addEventListener("fullscreenchange", onChange);
    return () => document.removeEventListener("fullscreenchange", onChange);
  }, [containerRef]);

  const exitApp = useCallback((fromPopState = false) => {
    setAppFullscreen(false);
    void setNativeImmersive(false, false);
    if (pushedHistory.current && !fromPopState) {
      pushedHistory.current = false;
      history.back();
    }
    pushedHistory.current = false;
  }, []);

  const enter = useCallback(() => {
    if (nativeFullscreenWorks() && containerRef.current?.requestFullscreen) {
      void containerRef.current.requestFullscreen().catch(() => {});
      return;
    }
    const video = containerRef.current?.querySelector("video");
    const landscape =
      !video || !video.videoWidth || !video.videoHeight || video.videoWidth >= video.videoHeight;
    setAppFullscreen(true);
    void setNativeImmersive(true, landscape);
    history.pushState({ ...(history.state ?? {}), playerFullscreen: true }, "");
    pushedHistory.current = true;
  }, [containerRef]);

  const toggle = useCallback(() => {
    if (document.fullscreenElement) {
      void document.exitFullscreen().catch(() => {});
    } else if (appFullscreen) {
      exitApp();
    } else {
      enter();
    }
  }, [appFullscreen, enter, exitApp]);

  // Back button / Escape leave app fullscreen.
  useEffect(() => {
    if (!appFullscreen) return;
    function onPop() {
      exitApp(true);
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.stopPropagation();
        exitApp();
      }
    }
    window.addEventListener("popstate", onPop);
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("popstate", onPop);
      window.removeEventListener("keydown", onKey, true);
    };
  }, [appFullscreen, exitApp]);

  // Never leave the phone stuck immersive / landscape when the player closes.
  useEffect(
    () => () => {
      if (pushedHistory.current) {
        void setNativeImmersive(false, false);
      }
    },
    [],
  );

  return {
    isFullscreen: appFullscreen || nativeFullscreen,
    /** App-level fullscreen is active: style the container to cover the screen. */
    appFullscreen,
    /** Hide the browser's own fullscreen button where it cannot work. */
    hideNativeButton: !nativeFullscreenWorks(),
    toggle,
  };
}

/** Classes for a player container in app fullscreen. */
export const APP_FULLSCREEN_CLASS =
  "fixed inset-0 z-[400] m-0 flex items-center justify-center bg-black [&_video]:h-full! [&_video]:max-h-none! [&_video]:w-full! [&_video]:rounded-none! [&_video]:aspect-auto!";
