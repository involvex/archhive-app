import type { MouseEvent, ReactNode } from "react";
import { openExternal } from "@/lib/external";
import { cn } from "@/lib/utils";

interface ExtLinkProps {
  href: string;
  className?: string;
  children: ReactNode;
  title?: string;
}

/**
 * External link that opens in the system browser.
 *
 * Keeps a real `href` (copy link / accessibility) but intercepts the click:
 * inside Tauri the webview can't open new windows, so the URL goes through
 * the opener plugin instead of a dead `target="_blank"` navigation.
 */
export function ExtLink({ href, className, children, title }: ExtLinkProps) {
  function onClick(e: MouseEvent<HTMLAnchorElement>) {
    e.preventDefault();
    void openExternal(href).catch(console.error);
  }

  return (
    <a
      href={href}
      onClick={onClick}
      rel="noreferrer"
      title={title}
      className={cn("text-[var(--color-primary)] hover:underline", className)}
    >
      {children}
    </a>
  );
}
