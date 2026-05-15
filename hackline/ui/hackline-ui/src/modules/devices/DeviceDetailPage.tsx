import { useEffect, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState, ErrorBox, PageBody, PageHeader } from "@/components/PageChrome";
import { useApi } from "@/lib/api";
import type { AgentInfo, Device, DeviceHealth, Tunnel } from "@/lib/api";
import { navigate } from "@/lib/route";
import { relTime } from "@/lib/utils";

// Same cadence as `DevicesPage` so liveliness freshness feels
// consistent when the user navigates between list and detail.
// At 5 s the goal-27 1 s RTT cache misses on every refresh for
// this single device (which is fine: one Zenoh query every 5 s).
const HEALTH_POLL_MS = 5_000;

export function DeviceDetailPage({ id }: { id: number }) {
  const api = useApi();
  const [device, setDevice] = useState<Device | null>(null);
  const [info, setInfo] = useState<AgentInfo | null>(null);
  const [health, setHealth] = useState<DeviceHealth | null>(null);
  const [tunnels, setTunnels] = useState<Tunnel[]>([]);
  const [error, setError] = useState<unknown>(null);

  useEffect(() => {
    let cancelled = false;
    const pollHealth = () => {
      api
        .getDeviceHealth(id)
        .then((h) => !cancelled && setHealth(h))
        .catch(() => {});
    };
    (async () => {
      try {
        const [d, ts] = await Promise.all([api.getDevice(id), api.listTunnels()]);
        if (cancelled) return;
        setDevice(d);
        setTunnels(ts.filter((t) => t.device_id === id));
        // Info is best-effort and only meaningful on linux-class
        // devices (constrained agents don't speak the info topic).
        // Health is polled separately below; the once-on-mount
        // call here is the first tick so the badge fills in
        // immediately, not after `HEALTH_POLL_MS`.
        pollHealth();
        if (d.class === "linux") {
          api.getDeviceInfo(id).then((i) => !cancelled && setInfo(i)).catch(() => {});
        }
      } catch (e) {
        if (!cancelled) setError(e);
      }
    })();
    const intervalId = window.setInterval(pollHealth, HEALTH_POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, [api, id]);

  if (error) {
    return (
      <div className="flex h-full flex-col">
        <PageHeader title={`Device #${id}`} actions={<Button variant="outline" size="sm" onClick={() => navigate({ name: "devices" })}>Back</Button>} />
        <PageBody>
          <ErrorBox error={error} />
        </PageBody>
      </div>
    );
  }
  if (!device) {
    return (
      <div className="flex h-full flex-col">
        <PageHeader title={`Device #${id}`} />
        <PageBody>
          <div className="text-xs text-muted-foreground">loading…</div>
        </PageBody>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title={device.label ?? `Device #${device.id}`}
        description={device.zid}
        actions={
          <>
            {health == null ? (
              <Badge variant="outline">—</Badge>
            ) : (
              <Badge variant={health.online ? "ok" : "err"}>
                {health.online ? "online" : "offline"}
              </Badge>
            )}
            <Button variant="outline" size="sm" onClick={() => navigate({ name: "devices" })}>
              Back
            </Button>
          </>
        }
      />
      <PageBody className="space-y-4">
        <div className="grid gap-4 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle>Health</CardTitle>
            </CardHeader>
            <CardContent className="space-y-1 text-xs">
              {/* The first three rows all come from the same probe
                  so they're rendered atomically: either all from
                  `health` or all `—`. Mixing in `device.last_seen_at`
                  as a fallback would let `online` flip while
                  `last seen` lagged a tick — a visible UI lie.
                  `class` is a row property, not a probe result, so
                  it stays sourced from `device`. */}
              <Row label="online" value={health == null ? "—" : String(health.online)} />
              <Row
                label="last seen"
                value={health == null ? "—" : relTime(health.last_seen_at)}
              />
              <Row
                label="rtt"
                value={health?.rtt_ms != null ? `${health.rtt_ms} ms` : "—"}
              />
              <Row label="class" value={device.class} />
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>Agent info</CardTitle>
            </CardHeader>
            <CardContent className="space-y-1 text-xs">
              {device.class === "constrained" ? (
                <div className="text-muted-foreground">
                  Constrained-class device — no <code>hackline-agent</code>, no tunnel plane.
                </div>
              ) : info ? (
                <>
                  <Row label="version" value={info.version} />
                  <Row label="uptime" value={`${Math.round(info.uptime_s / 60)} min`} />
                  <Row label="allowed ports" value={info.allowed_ports.join(", ") || "—"} />
                </>
              ) : (
                <div className="text-muted-foreground">live query pending…</div>
              )}
            </CardContent>
          </Card>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>Tunnels</CardTitle>
          </CardHeader>
          <CardContent>
            {tunnels.length === 0 ? (
              <EmptyState
                title="No tunnels"
                description="Add a tunnel from the Tunnels page to expose a local port."
              />
            ) : (
              <table className="w-full text-sm">
                <thead className="text-xs text-muted-foreground">
                  <tr>
                    <th className="px-2 py-1 text-left font-medium">Kind</th>
                    <th className="px-2 py-1 text-left font-medium">Local port</th>
                    <th className="px-2 py-1 text-left font-medium">Public</th>
                    <th className="px-2 py-1 text-left font-medium">Created</th>
                  </tr>
                </thead>
                <tbody>
                  {tunnels.map((t) => (
                    <tr key={t.id} className="border-t">
                      <td className="px-2 py-1.5">{t.kind}</td>
                      <td className="px-2 py-1.5 font-mono text-xs">{t.local_port}</td>
                      <td className="px-2 py-1.5 font-mono text-xs">
                        {t.public_hostname ?? `:${t.public_port ?? "—"}`}
                      </td>
                      <td className="px-2 py-1.5 text-xs text-muted-foreground">
                        {relTime(t.created_at)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </CardContent>
        </Card>
      </PageBody>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-mono">{value}</span>
    </div>
  );
}
