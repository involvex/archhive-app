import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** Card grid shared by the libraries and browse pages: 2 columns on a
 * portrait phone (readable thumbnails and titles), more on wider screens. */
export const CARD_GRID =
  "grid grid-cols-2 gap-2.5 sm:grid-cols-3 md:grid-cols-4 md:gap-3 lg:grid-cols-5 xl:grid-cols-6";
