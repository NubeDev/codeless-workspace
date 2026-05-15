// Wire types matching `hackline-proto` and the REST surface in
// `hackline/SCOPE.md` §5. Kept hand-written for now; once the gateway
// emits TS via specta we can replace these with the generated module
// (the consumer-facing `hackline-ts` package already exists for the
// connection-lifecycle subset, see `hackline/clients/hackline-ts/`).

export type DeviceClass = "linux" | "constrained";

export interface Device {
  id: number;
  zid: string;
  label: string | null;
  class: DeviceClass;
  customer_id: number | null;
  online: boolean;
  last_seen_ts: string | null;
  created_at: string;
}

export interface DeviceHealth {
  online: boolean;
  last_seen_ts: string | null;
  latency_ms_p50: number | null;
}

export interface AgentInfo {
  zid: string;
  version: string;
  allowed_ports: number[];
  uptime_s: number;
}

export type TunnelKind = "http" | "tcp" | "ssh";

export interface Tunnel {
  id: number;
  device_id: number;
  kind: TunnelKind;
  local_port: number;
  public_hostname: string | null;
  public_port: number | null;
  created_at: string;
}

export type CmdStatus = "pending" | "delivered" | "acked" | "expired";

export interface CmdOutboxRow {
  cmd_id: string;
  device_id: number;
  topic: string;
  status: CmdStatus;
  enqueued_at: string;
  expires_at: string;
  delivered_at: string | null;
  acked_at: string | null;
  result: "accepted" | "rejected" | "failed" | "done" | null;
  detail: string | null;
}

export interface AuditEntry {
  id: number;
  ts: string;
  actor: string;
  action: string;
  target: string | null;
  detail: Record<string, unknown> | null;
}

export type UserRole = "owner" | "admin" | "operator" | "viewer";

export interface User {
  id: number;
  name: string;
  role: UserRole;
  device_scope: string | null;
  tunnel_scope: string | null;
  expires_at: string | null;
  created_at: string;
}

export interface MintedToken {
  token: string;
  expires_at: string | null;
}

export interface Page<T> {
  entries: T[];
  next_cursor: string | null;
}

export interface ClaimStatus {
  claimed: boolean;
  can_claim: boolean;
}

// SSE control-plane event kinds (SCOPE.md §5.4).
export type GatewayEvent =
  | { kind: "device.online"; data: { device_id: number; zid: string; at: string } }
  | { kind: "device.offline"; data: { device_id: number; zid: string; at: string; reason: string } }
  | {
      kind: "tunnel.opened";
      data: { tunnel_id: number; device_id: number; request_id: string; peer: string | null };
    }
  | {
      kind: "tunnel.closed";
      data: {
        tunnel_id: number;
        request_id: string;
        bytes_up: number;
        bytes_down: number;
        duration_ms: number;
      };
    }
  | { kind: "cmd.queued"; data: { cmd_id: string; device_id: number; topic: string } }
  | { kind: "cmd.delivered"; data: { cmd_id: string; device_id: number; at: string } }
  | {
      kind: "cmd.acked";
      data: { cmd_id: string; device_id: number; result: string; at: string };
    }
  | { kind: "cmd.expired"; data: { cmd_id: string; device_id: number } }
  | { kind: "audit.entry"; data: AuditEntry };
