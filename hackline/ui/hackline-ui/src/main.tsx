import { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";

import "./styles/globals.css";

import { App } from "./App";
import {
  ApiProvider,
  HttpApiClient,
  MockApiClient,
  readBaseUrl,
  readToken,
  type ApiClient,
} from "./lib/api";
import { ClaimScreen } from "./modules/claim/ClaimScreen";

// Boot order matches codeless-ui's browser shell:
//   1. `?mock=1` short-circuits to MockApiClient (UI-only dev).
//   2. Otherwise probe /v1/health on the configured base URL.
//      - unreachable -> honest "cannot reach gateway" screen
//      - reachable + unclaimed -> ClaimScreen (writes the owner token)
//      - reachable + claimed but no token -> Settings is the only
//        useful page; we render a token-prompt screen.
//      - reachable + claimed + token -> full App.

type Mode =
  | { kind: "probing" }
  | { kind: "down"; baseUrl: string }
  | { kind: "claim" }
  | { kind: "no-token" }
  | { kind: "ready" }
  | { kind: "mock" };

function isMockRequested(): boolean {
  return new URLSearchParams(window.location.search).get("mock") === "1";
}

function buildHttp(): HttpApiClient {
  return new HttpApiClient({ baseUrl: readBaseUrl(), token: readToken() });
}

function Root() {
  const [baseUrl] = useState(readBaseUrl);
  const [client, setClient] = useState<ApiClient | null>(() =>
    isMockRequested() ? new MockApiClient() : null,
  );
  const [mode, setMode] = useState<Mode>(() =>
    isMockRequested() ? { kind: "mock" } : { kind: "probing" },
  );

  useEffect(() => {
    if (mode.kind !== "probing") return;
    let cancelled = false;
    (async () => {
      const probe = new HttpApiClient({ baseUrl, token: readToken() });
      try {
        await probe.health();
      } catch {
        if (!cancelled) setMode({ kind: "down", baseUrl });
        return;
      }
      let claim;
      try {
        claim = await probe.claimStatus();
      } catch {
        // Older gateway revs might not expose claim status; treat as
        // "claimed" so we don't block the operator.
        claim = { claimed: true, can_claim: false };
      }
      if (cancelled) return;
      if (!claim.claimed && claim.can_claim) {
        setClient(probe);
        setMode({ kind: "claim" });
        return;
      }
      const c = buildHttp();
      setClient(c);
      setMode(c.hasToken() ? { kind: "ready" } : { kind: "no-token" });
    })();
    return () => {
      cancelled = true;
    };
  }, [mode, baseUrl]);

  if (mode.kind === "probing" || !client) return null;
  if (mode.kind === "down") {
    return <ServerDown baseUrl={mode.baseUrl} onRetry={() => setMode({ kind: "probing" })} />;
  }
  if (mode.kind === "claim") {
    return (
      <ApiProvider client={client}>
        <ClaimScreen onDone={() => window.location.reload()} />
      </ApiProvider>
    );
  }
  if (mode.kind === "no-token") {
    return <NoToken />;
  }
  return (
    <ApiProvider client={client}>
      <App />
      {mode.kind === "mock" ? <MockBadge /> : null}
    </ApiProvider>
  );
}

function ServerDown({ baseUrl, onRetry }: { baseUrl: string; onRetry: () => void }) {
  return (
    <div className="flex min-h-screen items-center justify-center p-6">
      <div className="max-w-lg space-y-3">
        <h1 className="text-base font-semibold">cannot reach hackline gateway</h1>
        <p className="text-xs text-muted-foreground">The UI tried:</p>
        <pre className="rounded bg-muted px-2 py-1 text-xs">{baseUrl}</pre>
        <p className="text-xs text-muted-foreground">
          Start the gateway, or append <code>?mock=1</code> to the URL to run
          the UI without a backend.
        </p>
        <button
          onClick={onRetry}
          className="rounded-md border px-3 py-1 text-xs hover:bg-accent"
        >
          retry
        </button>
      </div>
    </div>
  );
}

function NoToken() {
  return (
    <div className="flex min-h-screen items-center justify-center p-6">
      <div className="max-w-md space-y-3">
        <h1 className="text-base font-semibold">No bearer token configured</h1>
        <p className="text-xs text-muted-foreground">
          Open Settings (or visit <code>#/settings</code>) and paste your
          bearer token. Mint one with <code>hackline users tokens</code>.
        </p>
        <a
          href="#/settings"
          onClick={() => setTimeout(() => window.location.reload(), 50)}
          className="inline-block rounded-md border px-3 py-1 text-xs hover:bg-accent"
        >
          open settings
        </a>
      </div>
    </div>
  );
}

function MockBadge() {
  return (
    <div className="pointer-events-none fixed bottom-2 right-2 rounded-md border border-warn/40 bg-[color:var(--warn)]/15 px-2 py-1 font-mono text-[11px] text-[color:var(--warn)]">
      mock mode · ?mock=1
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <Root />,
);
