import { isMobileDevice, isTauri } from "./tauri";

export interface BiometricOutcome {
  /** True when the user may proceed (biometric pass, or explicit fallback). */
  ok: boolean;
  /** Message to show when `ok` is false. */
  message?: string;
  /**
   * True when the platform could not offer a biometric prompt, so the unlock
   * was confirmed with a plain dialog instead. The vault is still "hidden";
   * it is just not gated by hardware.
   */
  degraded: boolean;
}

/**
 * Ask the OS for a biometric check before opening the vault session.
 *
 * The vault itself is guarded by a session lock in the Rust backend; this
 * prompt is the user-facing gate. On devices without enrolled biometrics the
 * caller gets `degraded: true` and is expected to confirm with a plain dialog
 * so the user can never be locked out of their own library.
 */
export async function authenticateForVault(reason = "Unlock the vault"): Promise<BiometricOutcome> {
  if (!isTauri() || !isMobileDevice()) {
    return { ok: true, degraded: true };
  }
  try {
    const biometric = await import("@tauri-apps/plugin-biometric");
    const status = await biometric.checkStatus();
    if (!status.isAvailable) {
      return { ok: true, degraded: true };
    }
    await biometric.authenticate(reason, {
      title: "Unlock ArcHive Vault",
      subtitle: reason,
      cancelTitle: "Cancel",
      // Let the OS allow PIN/pattern when no fingerprint is enrolled yet.
      confirmationRequired: false,
    });
    return { ok: true, degraded: false };
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    // No enrolled fingerprint/face (or no sensor that the OS will use): fall
    // back to the degraded confirmation instead of locking the user out of
    // their own library.
    if (/enrol|not available|no biometric|unavailable/i.test(message)) {
      return { ok: true, degraded: true };
    }
    // Cancelling the prompt is not an error worth a scary banner.
    const cancelled = /cancel|dismiss|user cancel/i.test(message);
    return {
      ok: false,
      degraded: false,
      message: cancelled ? undefined : message,
    };
  }
}
