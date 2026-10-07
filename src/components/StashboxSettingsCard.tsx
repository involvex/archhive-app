import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { StashBoxEndpoint } from "@/lib/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ExtLink } from "@/components/ExtLink";

const STASHDB_DEFAULT = "https://stashdb.org/graphql";

/**
 * Stash-box endpoint management (Settings → Metadata).
 * API keys are stored encrypted in the host vault and never leave it —
 * the list API only exposes `has_key`. Keys are NOT included in backups.
 */
export function StashboxSettingsCard() {
  const [endpoints, setEndpoints] = useState<StashBoxEndpoint[]>([]);
  const [loading, setLoading] = useState(true);
  const [name, setName] = useState("StashDB");
  const [endpoint, setEndpoint] = useState(STASHDB_DEFAULT);
  const [apiKey, setApiKey] = useState("");
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);
  const [testingId, setTestingId] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setEndpoints(await api.listStashboxEndpoints());
    } catch (e) {
      setStatus(e instanceof Error ? e.message : "Failed to load endpoints");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void refresh();
  }, [refresh]);

  async function save() {
    if (busy) return;
    if (!name.trim() || !endpoint.trim()) {
      setStatus("Name and endpoint URL are required.");
      return;
    }
    setBusy(true);
    setStatus("");
    try {
      await api.saveStashboxEndpoint({
        name: name.trim(),
        endpoint: endpoint.trim(),
        apiKey: apiKey.trim() || undefined,
      });
      setApiKey("");
      setStatus(
        apiKey.trim()
          ? "Endpoint saved with API key (encrypted in vault)."
          : "Endpoint saved. Add an API key to enable queries.",
      );
      await refresh();
    } catch (e) {
      setStatus(e instanceof Error ? e.message : "Save failed");
    } finally {
      setBusy(false);
    }
  }

  async function test(id: string) {
    if (testingId) return;
    setTestingId(id);
    setStatus("");
    try {
      const account = await api.testStashboxEndpoint(id);
      setStatus(`Connected as ${account}.`);
    } catch (e) {
      setStatus(e instanceof Error ? e.message : "Connection test failed");
    } finally {
      setTestingId(null);
    }
  }

  async function remove(id: string) {
    if (!window.confirm("Remove this stash-box endpoint and its stored API key?")) return;
    try {
      await api.deleteStashboxEndpoint(id);
      await refresh();
    } catch (e) {
      setStatus(e instanceof Error ? e.message : "Delete failed");
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">Stash-Box Metadata</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <p className="text-xs text-[var(--color-muted-foreground)]">
          Enrich library scenes from community databases (StashDB, FansDB, JAVStash…). Matching uses
          your files&apos; perceptual hashes — nothing is uploaded except fingerprints you
          explicitly submit. Get a StashDB API key from{" "}
          <ExtLink href="https://stashdb.org/settings?tab=api" className="underline">
            stashdb.org → Settings → API
          </ExtLink>
          . Keys stay encrypted on this device and are never exported in backups.
        </p>
        {loading ? (
          <p className="text-xs text-[var(--color-muted-foreground)]">Loading endpoints…</p>
        ) : endpoints.length === 0 ? (
          <p className="text-xs text-[var(--color-muted-foreground)]">
            No endpoints yet — add StashDB below to enable Enrich.
          </p>
        ) : (
          <ul className="space-y-2 text-sm">
            {endpoints.map((ep) => (
              <li key={ep.id} className="rounded-md border border-[var(--color-border)] p-3">
                <div className="flex items-center justify-between gap-2">
                  <span className="font-medium">{ep.name}</span>
                  <span
                    className={`rounded px-1.5 py-0.5 text-xs ${
                      ep.has_key
                        ? "bg-green-500/15 text-green-400"
                        : "bg-yellow-500/15 text-yellow-400"
                    }`}
                  >
                    {ep.has_key ? "key stored" : "no key"}
                  </span>
                </div>
                <p className="mt-1 break-all font-mono text-xs text-[var(--color-muted-foreground)]">
                  {ep.endpoint}
                </p>
                <div className="mt-2 flex flex-wrap gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => void test(ep.id)}
                    disabled={testingId !== null || !ep.has_key}
                  >
                    {testingId === ep.id ? "Testing…" : "Test"}
                  </Button>
                  <Button variant="outline" size="sm" onClick={() => void remove(ep.id)}>
                    Remove
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        )}
        <div className="space-y-2 rounded-md border border-[var(--color-border)] p-3">
          <p className="text-xs font-medium">Add endpoint</p>
          <Input
            placeholder="Name (e.g. StashDB)"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <Input
            placeholder="https://stashdb.org/graphql"
            value={endpoint}
            onChange={(e) => setEndpoint(e.target.value)}
            inputMode="url"
          />
          <Input
            placeholder="API key (stored encrypted, optional for now)"
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            autoComplete="off"
          />
          <Button onClick={() => void save()} disabled={busy}>
            {busy ? "Saving…" : "Save endpoint"}
          </Button>
        </div>
        {status && <p className="text-xs text-[var(--color-muted-foreground)]">{status}</p>}
      </CardContent>
    </Card>
  );
}
