import { useEffect, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { EmptyState, ErrorBox, PageBody, PageHeader } from "@/components/PageChrome";
import { useApi } from "@/lib/api";
import type { Device } from "@/lib/api";
import { navigate } from "@/lib/route";
import { relTime, shortId } from "@/lib/utils";

export function DevicesPage() {
  const api = useApi();
  const [devices, setDevices] = useState<Device[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [creating, setCreating] = useState(false);
  const [zid, setZid] = useState("");
  const [label, setLabel] = useState("");

  const refresh = async () => {
    try {
      setDevices(await api.listDevices());
      setError(null);
    } catch (e) {
      setError(e);
    }
  };

  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const onCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!zid.trim()) return;
    try {
      await api.createDevice({ zid: zid.trim(), label: label.trim() || null });
      setZid("");
      setLabel("");
      setCreating(false);
      void refresh();
    } catch (err) {
      setError(err);
    }
  };

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Devices"
        description="Every box on the fabric. Click a row for tunnel + message-plane detail."
        actions={
          <Button size="sm" onClick={() => setCreating((v) => !v)}>
            {creating ? "Cancel" : "Add device"}
          </Button>
        }
      />
      <PageBody>
        {creating ? (
          <form
            onSubmit={onCreate}
            className="mb-4 flex flex-wrap items-end gap-2 rounded-lg border bg-card p-3"
          >
            <div className="flex flex-col gap-1">
              <label className="text-[11px] text-muted-foreground">ZID</label>
              <Input
                value={zid}
                onChange={(e) => setZid(e.target.value)}
                placeholder="01HG3K2P0Z…"
                className="w-[28ch] font-mono"
              />
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-[11px] text-muted-foreground">Label</label>
              <Input
                value={label}
                onChange={(e) => setLabel(e.target.value)}
                placeholder="rack-7-a"
                className="w-48"
              />
            </div>
            <Button type="submit" size="sm">
              Create
            </Button>
          </form>
        ) : null}

        {error ? <ErrorBox error={error} /> : null}

        {devices == null ? (
          <div className="text-xs text-muted-foreground">loading…</div>
        ) : devices.length === 0 ? (
          <EmptyState
            title="No devices yet"
            description="Run hackline-agent on a device, or create a constrained-class device with the button above."
          />
        ) : (
          <div className="overflow-hidden rounded-lg border">
            <table className="w-full text-sm">
              <thead className="bg-muted/40 text-xs text-muted-foreground">
                <tr>
                  <th className="px-3 py-2 text-left font-medium">Status</th>
                  <th className="px-3 py-2 text-left font-medium">Label</th>
                  <th className="px-3 py-2 text-left font-medium">ZID</th>
                  <th className="px-3 py-2 text-left font-medium">Class</th>
                  <th className="px-3 py-2 text-left font-medium">Last seen</th>
                </tr>
              </thead>
              <tbody>
                {devices.map((d) => (
                  <tr
                    key={d.id}
                    className="cursor-pointer border-t hover:bg-accent/40"
                    onClick={() => navigate({ name: "device", id: d.id })}
                  >
                    <td className="px-3 py-2">
                      <Badge variant={d.online ? "ok" : "err"}>
                        {d.online ? "online" : "offline"}
                      </Badge>
                    </td>
                    <td className="px-3 py-2">{d.label ?? <span className="text-muted-foreground">—</span>}</td>
                    <td className="px-3 py-2 font-mono text-xs">{shortId(d.zid, 14)}</td>
                    <td className="px-3 py-2 text-xs text-muted-foreground">{d.class}</td>
                    <td className="px-3 py-2 text-xs text-muted-foreground">
                      {relTime(d.last_seen_ts)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </PageBody>
    </div>
  );
}
