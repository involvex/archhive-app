/**
 * Resolve the auto-advance target when playback ends (Q24).
 * Returns null unless advancing is enabled and a next scene exists.
 */
export function getAutoAdvanceTarget<T>(
  scenes: T[] | undefined,
  currentIndex: number | undefined | null,
  autoAdvanceNext: boolean,
): { scene: T; index: number } | null {
  if (!autoAdvanceNext || !scenes || currentIndex == null) return null;
  if (currentIndex < scenes.length - 1) {
    return { scene: scenes[currentIndex + 1], index: currentIndex + 1 };
  }
  return null;
}
