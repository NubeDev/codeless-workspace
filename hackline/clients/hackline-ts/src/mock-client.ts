import type { ApiClient } from "./client";
import type {
  AgentInfo,
  AuditEntry,
  ClaimStatus,
  CmdOutboxRow,
  CmdStatus,
  Device,
  DeviceHealth,
  GatewayEvent,
  MintedToken,
  Page,
  Tunnel,
  TunnelKind,
  User,
  UserRole,
} from "./types";

// In-memory fixtures so the UI is usable before the gateway is up.
// Selected by `?mock=1` (see `main.tsx`). Numbers and strings are
// representative, not exhaustive — enough surface to exercise every
// page's empty/non-empty paths.
export class MockApiClient implements ApiClient {
  baseUrl = "mock://hackline";
  private devices: Device[] = [
    {
      id: 1,
      zid: "01HG3K2P0Z7Q1XV6E2RT3NB7M0",
      label: "rack-7-a",
      class: "linux",
      customer_id: null,
      online: true,
      last_seen_ts: new Date(Date.now() - 12_000).toISOString(),
      created_at: new Date(Date.now() - 86_400_000 * 14).toISOString(),
    },
    {
      id: 2,
      zid: "01HG3K2P5Z7Q1XV6E2RT3NB7N1",
      label: "sensor-rack-7",
      class: "constrained",
      customer_id: null,
      online: false,
      last_seen_ts: new Date(Date.now() - 3_600_000).toISOString(),
      created_at: new Date(Date.now() - 86_400_000 * 7).toISOString(),
    },
  ];
  private tunnels: Tunnel[] = [
    {
      id: 1,
      device_id: 1,
      kind: "http",
      local_port: 8080,
      public_hostname: "device-1.cloud.example.com",
      public_port: null,
      created_at: new Date(Date.now() - 3_600_000).toISOString(),
    },
  ];
  private cmd: CmdOutboxRow[] = [
    {
      cmd_id: "01HG3K2P0Z7Q1XV6E2RT3NB7CD",
      device_id: 1,
      topic: "block.install",
      status: "acked",
      enqueued_at: new Date(Date.now() - 600_000).toISOString(),
      expires_at: new Date(Date.now() + 600_000).toISOString(),
      delivered_at: new Date(Date.now() - 590_000).toISOString(),
      acked_at: new Date(Date.now() - 580_000).toISOString(),
      result: "done",
      detail: null,
    },
  ];
  private audit: AuditEntry[] = [
    {
      id: 1,
      ts: new Date(Date.now() - 60_000).toISOString(),
      actor: "owner",
      action: "device.create",
      target: "device:1",
      detail: { zid: "01HG3K2P0Z7Q1XV6E2RT3NB7M0" },
    },
  ];
  private users: User[] = [
    {
      id: 1,
      name: "owner",
      role: "owner",
      device_scope: null,
      tunnel_scope: null,
      expires_at: null,
      created_at: new Date(Date.now() - 86_400_000 * 30).toISOString(),
    },
  ];

  hasToken() {
    return true;
  }

  health = async () => ({ ok: true as const });
  claimStatus = async (): Promise<ClaimStatus> => ({ claimed: true, can_claim: false });
  claim = async (_: { token: string; owner: string }) => ({
    token: "hk_mock_owner_token",
    owner: _.owner,
  });

  listDevices = async () => [...this.devices];
  getDevice = async (id: number) => {
    const d = this.devices.find((x) => x.id === id);
    if (!d) throw new Error("not found");
    return d;
  };
  createDevice = async (input: { zid: string; label?: string | null }) => {
    const d: Device = {
      id: this.devices.length + 1,
      zid: input.zid,
      label: input.label ?? null,
      class: "linux",
      customer_id: null,
      online: false,
      last_seen_ts: null,
      created_at: new Date().toISOString(),
    };
    this.devices.push(d);
    return d;
  };
  deleteDevice = async (id: number) => {
    this.devices = this.devices.filter((x) => x.id !== id);
  };
  getDeviceInfo = async (id: number): Promise<AgentInfo> => ({
    zid: this.devices.find((x) => x.id === id)?.zid ?? "",
    version: "0.1.0-mock",
    allowed_ports: [22, 8080],
    uptime_s: 86_400,
  });
  getDeviceHealth = async (id: number): Promise<DeviceHealth> => {
    const d = this.devices.find((x) => x.id === id);
    return {
      online: d?.online ?? false,
      last_seen_ts: d?.last_seen_ts ?? null,
      latency_ms_p50: d?.online ? 42 : null,
    };
  };

  listTunnels = async () => [...this.tunnels];
  createTunnel = async (input: {
    device_id: number;
    kind: TunnelKind;
    local_port: number;
    public_hostname?: string | null;
    public_port?: number | null;
  }) => {
    const t: Tunnel = {
      id: this.tunnels.length + 1,
      device_id: input.device_id,
      kind: input.kind,
      local_port: input.local_port,
      public_hostname: input.public_hostname ?? null,
      public_port: input.public_port ?? null,
      created_at: new Date().toISOString(),
    };
    this.tunnels.push(t);
    return t;
  };
  deleteTunnel = async (id: number) => {
    this.tunnels = this.tunnels.filter((x) => x.id !== id);
  };

  sendCmd = async (input: {
    device_id: number;
    topic: string;
    payload: unknown;
    expires_in_s?: number;
  }) => {
    const cmd_id = `01HG${Math.random().toString(36).slice(2, 10).toUpperCase()}`;
    this.cmd.unshift({
      cmd_id,
      device_id: input.device_id,
      topic: input.topic,
      status: "pending",
      enqueued_at: new Date().toISOString(),
      expires_at: new Date(Date.now() + (input.expires_in_s ?? 600) * 1000).toISOString(),
      delivered_at: null,
      acked_at: null,
      result: null,
      detail: null,
    });
    return { cmd_id };
  };
  listCmd = async (input: {
    device_id: number;
    status?: CmdStatus;
  }): Promise<Page<CmdOutboxRow>> => {
    const entries = this.cmd.filter(
      (c) => c.device_id === input.device_id && (!input.status || c.status === input.status),
    );
    return { entries, next_cursor: null };
  };
  cancelCmd = async (cmd_id: string) => {
    this.cmd = this.cmd.filter((c) => c.cmd_id !== cmd_id);
  };

  listAudit = async (): Promise<Page<AuditEntry>> => ({
    entries: [...this.audit],
    next_cursor: null,
  });

  listUsers = async () => [...this.users];
  createUser = async (input: {
    name: string;
    role: UserRole;
    device_scope?: string | null;
    tunnel_scope?: string | null;
    expires_in_s?: number;
  }) => {
    const u: User = {
      id: this.users.length + 1,
      name: input.name,
      role: input.role,
      device_scope: input.device_scope ?? null,
      tunnel_scope: input.tunnel_scope ?? null,
      expires_at: input.expires_in_s
        ? new Date(Date.now() + input.expires_in_s * 1000).toISOString()
        : null,
      created_at: new Date().toISOString(),
    };
    this.users.push(u);
    return u;
  };
  deleteUser = async (id: number) => {
    this.users = this.users.filter((x) => x.id !== id);
  };
  mintToken = async (_user_id: number): Promise<MintedToken> => ({
    token: `hk_mock_${Math.random().toString(36).slice(2, 18)}`,
    expires_at: null,
  });

  subscribeEvents(listener: (event: GatewayEvent) => void): () => void {
    // Synthetic ticker so the live-events page shows movement in mock mode.
    let n = 0;
    const handle = window.setInterval(() => {
      n += 1;
      listener({
        kind: "tunnel.opened",
        data: {
          tunnel_id: 1,
          device_id: 1,
          request_id: `req-${n}`,
          peer: "203.0.113.5",
        },
      });
    }, 4000);
    return () => window.clearInterval(handle);
  }
}
