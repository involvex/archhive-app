import { Link } from "@tanstack/react-router";
import type { ArcHivePlugin } from "@/lib/plugins/types";

const plugin: ArcHivePlugin = {
  id: "example",
  register(ctx) {
    ctx.addBrowseSite({
      id: "example-tube",
      display_name: "Example Tube",
      base_url: "https://example.com",
      supported_kinds: ["search", "video"],
      requires_cookies: false,
    });

    ctx.addSettingsPanel({
      id: "example-settings",
      title: "Installed Plugins",
      tab: "engine",
      render: () => (
        <div className="space-y-2 text-sm text-[var(--color-muted-foreground)]">
          <p>
            <strong className="text-[var(--color-foreground)]">Example Plugin</strong> is active.
            Clone plugins into <code>plugins/</code> and run <code>bun run plugins:generate</code>.
          </p>
          <p>
            This demo site card uses <strong>Browse by URL</strong> for real downloads — open{" "}
            <Link to="/browse/by-url" className="text-[var(--color-primary)] underline">
              Custom URL
            </Link>
            .
          </p>
        </div>
      ),
    });
  },
};

export default plugin;
