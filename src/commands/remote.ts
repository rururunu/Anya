import { invoke } from "@tauri-apps/api/core";

export interface PairedDevice {
  deviceId: string;
  credential: string;
  deviceName?: string | null;
  pairedAtEpochMs: number;
  lastSeenEpochMs: number;
}

export interface PairingSessionInfo {
  token: string;
  pairingCode: string;
  host: string;
  hosts: string[];
  port: number;
  scheme: string;
  lanHosts?: string[];
  lanPort?: number;
  qrPayload: string;
  qrDataUrl: string;
  expiresAtEpochMs: number;
}

export interface TunnelPrefs {
  cloudflaredEnabled: boolean;
  cloudflaredToken?: string | null;
  cloudflaredHostname?: string | null;
  cloudflaredBinary: string;
  useQuickTunnel: boolean;
}

export interface GatewayStatus {
  running: boolean;
  port: number;
  connectedClients: number;
  pairingActive: boolean;
  pairing: PairingSessionInfo | null;
  devices: PairedDevice[];
}

export function remoteGatewayStatus(): Promise<GatewayStatus> {
  return invoke("remote_gateway_status");
}

export function remoteGatewayStart(port?: number): Promise<GatewayStatus> {
  return invoke("remote_gateway_start", { port: port ?? null });
}

export function remoteGatewayStop(): Promise<GatewayStatus> {
  return invoke("remote_gateway_stop");
}

export function remoteCreatePairing(): Promise<PairingSessionInfo> {
  return invoke("remote_create_pairing");
}

export function remoteGetTunnelPrefs(): Promise<TunnelPrefs> {
  return invoke("remote_get_tunnel_prefs");
}

export function remoteSetTunnelPrefs(prefs: TunnelPrefs): Promise<void> {
  return invoke("remote_set_tunnel_prefs", { prefs });
}

export function remoteListDevices(): Promise<PairedDevice[]> {
  return invoke("remote_list_devices");
}

export function remoteRevokeDevice(deviceId: string): Promise<GatewayStatus> {
  return invoke("remote_revoke_device", { deviceId });
}

export interface RemoteSessionCompose {
  chatMode: "ask" | "agent" | "plan";
  toolApprovalMode: "ask" | "auto" | "alwaysAllow";
  chatModel: string;
  chatModelProvider: string;
  chatModelLabel?: string | null;
}

export function remoteSyncSessionCompose(
  sessionId: string,
  compose: RemoteSessionCompose,
): Promise<RemoteSessionCompose> {
  return invoke("remote_sync_session_compose", { sessionId, compose });
}

export function remoteGetSessionCompose(sessionId: string): Promise<RemoteSessionCompose> {
  return invoke("remote_get_session_compose", { sessionId });
}
