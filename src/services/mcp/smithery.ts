/**
 * Smithery MCP Registry client (https://registry.smithery.ai/servers).
 * Remote/hosted servers install via mcp-remote → Anya stdio MCP runtime.
 */

import type { McpServerConfig } from "@/types/setting";
import {
  MCP_REMOTE_PACKAGE,
  mcpInstallId as stableMcpInstallId,
  sanitizeMcpInstallId,
} from "@/services/mcp/remote";

const REGISTRY_BASE = "https://registry.smithery.ai";
/** Connections REST API — https://smithery.ai/docs/use/connect */
const CONNECTIONS_BASE = "https://smithery.run";
/** Platform API (namespaces, tokens, etc.) — https://api.smithery.ai */
const PLATFORM_API_BASE = "https://api.smithery.ai";

let cachedNamespace: { apiKey: string; name: string } | null = null;

/**
 * The Connections API requires a namespace the caller already owns — it does not
 * auto-create arbitrary names. Reuse the account's first existing namespace, or
 * create one (server-generated name) if the account has none yet.
 * https://smithery.ai/docs/api-reference/namespaces
 */
export async function resolveSmitheryNamespace(apiKey: string): Promise<string> {
  const key = apiKey.trim();
  if (!key) throw new Error("Smithery API key is required");
  if (cachedNamespace?.apiKey === key) return cachedNamespace.name;

  const headers = { Authorization: `Bearer ${key}` };
  const listResponse = await fetch(`${PLATFORM_API_BASE}/namespaces`, { headers });
  if (listResponse.ok) {
    const data = (await listResponse.json().catch(() => ({}))) as {
      namespaces?: Array<{ name?: string }>;
    };
    const existing = data.namespaces?.find((ns) => ns.name?.trim())?.name?.trim();
    if (existing) {
      cachedNamespace = { apiKey: key, name: existing };
      return existing;
    }
  }

  const createResponse = await fetch(`${PLATFORM_API_BASE}/namespaces`, {
    method: "POST",
    headers,
  });
  const created = (await createResponse.json().catch(() => ({}))) as {
    name?: string;
    error?: string;
  };
  if (!createResponse.ok || !created.name) {
    throw new Error(created.error || `Smithery namespace error (${createResponse.status})`);
  }
  cachedNamespace = { apiKey: key, name: created.name };
  return created.name;
}

export type SmitheryMcpServerSummary = {
  id: string;
  qualifiedName: string;
  namespace?: string;
  slug?: string;
  displayName: string;
  description: string;
  verified?: boolean;
  useCount?: number;
  remote?: boolean;
  isDeployed?: boolean;
  iconUrl?: string;
  homepage?: string;
};

export type SmitheryMcpConnection = {
  type?: string;
  deploymentUrl?: string;
  bundleUrl?: string;
  runtime?: string;
  configSchema?: {
    type?: string;
    required?: string[];
    properties?: Record<
      string,
      {
        type?: string;
        title?: string;
        description?: string;
        default?: string | number | boolean;
      }
    >;
  };
};

export type SmitheryMcpServerDetail = SmitheryMcpServerSummary & {
  deploymentUrl?: string | null;
  connections?: SmitheryMcpConnection[];
};

export type SmitheryMcpServersPage = {
  servers: SmitheryMcpServerSummary[];
  pagination: {
    currentPage: number;
    pageSize: number;
    totalPages: number;
    totalCount: number;
  };
};

export function mcpInstallId(
  server: Pick<SmitheryMcpServerSummary, "qualifiedName" | "slug" | "displayName" | "id">,
): string {
  return stableMcpInstallId(server);
}

/** Format use counts like Smithery UI: 63171 → "63.17k uses". */
export function formatSmitheryUses(count: number): string {
  const n = Math.max(0, Math.floor(count));
  let compact: string;
  if (n >= 1_000_000) {
    const m = n / 1_000_000;
    compact = `${m >= 10 ? m.toFixed(1) : m.toFixed(2).replace(/0+$/, "").replace(/\.$/, "")}M`;
  } else if (n >= 1_000) {
    const k = n / 1_000;
    compact = `${k >= 100 ? k.toFixed(0) : k.toFixed(2).replace(/0+$/, "").replace(/\.$/, "")}k`;
  } else {
    compact = String(n);
  }
  return `${compact} uses`;
}

export async function searchSmitheryMcpServers(
  query: string,
  options: { page?: number; pageSize?: number; remoteOnly?: boolean } = {},
): Promise<SmitheryMcpServersPage> {
  const params = new URLSearchParams();
  const q = query.trim();
  if (q) params.set("q", q);
  params.set("page", String(options.page ?? 1));
  params.set("pageSize", String(options.pageSize ?? 20));
  // Prefer hosted/remote servers — Anya connects them via mcp-remote.
  if (options.remoteOnly !== false) params.set("remote", "1");

  const urls = [
    `${REGISTRY_BASE}/servers?${params.toString()}`,
    // Smithery has intermittently rejected the `remote=1` filter. Retry the
    // public endpoint without it so catalog browsing remains available.
    `${REGISTRY_BASE}/servers?${new URLSearchParams(
      [...params].filter(([key]) => key !== "remote"),
    ).toString()}`,
  ];
  let response: Response | undefined;
  let lastStatus = 0;
  for (const url of urls) {
    const candidate = await fetch(url);
    if (candidate.ok) {
      response = candidate;
      break;
    }
    lastStatus = candidate.status;
  }
  if (!response) {
    throw new Error(
      lastStatus
        ? `Smithery registry is unavailable (HTTP ${lastStatus}). Try again later.`
        : "Smithery registry could not be reached. Check your network connection.",
    );
  }
  const data = (await response.json()) as SmitheryMcpServersPage;
  const servers = Array.isArray(data.servers) ? data.servers : [];
  return {
    // Registry search ranks by relevance; prefer download/use popularity by default.
    servers: sortSmitheryMcpByDownloads(servers),
    pagination: data.pagination ?? {
      currentPage: options.page ?? 1,
      pageSize: options.pageSize ?? 20,
      totalPages: 1,
      totalCount: 0,
    },
  };
}

export function sortSmitheryMcpByDownloads(
  servers: SmitheryMcpServerSummary[],
): SmitheryMcpServerSummary[] {
  return [...servers].sort((a, b) => (b.useCount ?? 0) - (a.useCount ?? 0));
}

export async function getSmitheryMcpServer(
  qualifiedName: string,
): Promise<SmitheryMcpServerDetail> {
  const response = await fetch(
    `${REGISTRY_BASE}/servers/${qualifiedName
      .split("/")
      .map((part) => encodeURIComponent(part))
      .join("/")}`,
  );
  if (!response.ok) {
    throw new Error(`Smithery server not found (${response.status})`);
  }
  return (await response.json()) as SmitheryMcpServerDetail;
}

export function resolveSmitheryDeploymentUrl(detail: SmitheryMcpServerDetail): string | null {
  const http = (detail.connections ?? []).find(
    (conn) => (conn.type ?? "").toLowerCase() === "http" && Boolean(conn.deploymentUrl),
  );
  if (http?.deploymentUrl) return http.deploymentUrl.trim();
  if (detail.deploymentUrl) return detail.deploymentUrl.trim();
  if (detail.remote || detail.isDeployed) {
    return `https://server.smithery.ai/${detail.qualifiedName}`;
  }
  return null;
}

/** Append Smithery `api_key` so hosted MCP skips fragile local browser OAuth. */
export function withSmitheryApiKey(url: string, apiKey: string): string {
  const key = apiKey.trim();
  if (!key) return url;
  // Connect proxy uses Authorization: Bearer (injected at spawn), not query api_key.
  if (isSmitheryConnectProxyUrl(url)) return url;
  const lower = url.toLowerCase();
  if (!lower.includes("smithery.ai") && !lower.includes("run.tools")) return url;
  if (/[?&]api_key=/i.test(url)) return url;
  const sep = url.includes("?") ? "&" : "?";
  return `${url}${sep}api_key=${encodeURIComponent(key)}`;
}

/** Point mcp-remote at the Smithery Connect proxy so tool calls use vaulted OAuth credentials. */
export function withSmitheryConnectProxyArgs(
  args: string[],
  namespace: string,
  connectionId: string,
): string[] {
  const next = [...args];
  const pkgIdx = next.findIndex((arg) => {
    const base = arg.split("@")[0] ?? arg;
    return base.toLowerCase() === "mcp-remote";
  });
  if (pkgIdx < 0) return next;
  const proxy = smitheryConnectProxyUrl(namespace, connectionId);
  const urlIdx = next.findIndex((arg, i) => i > pkgIdx && /^https?:\/\//i.test(arg));
  if (urlIdx >= 0) {
    next[urlIdx] = proxy;
  } else {
    next.splice(pkgIdx + 1, 0, proxy);
  }
  // Drop any previously persisted Authorization headers (secrets belong at spawn-time).
  for (let i = next.length - 2; i >= 0; i -= 1) {
    if (next[i] === "--header" && /^authorization\s*:/i.test(next[i + 1] ?? "")) {
      next.splice(i, 2);
    }
  }
  return next;
}

/** Stable Smithery connection id for an installed server (Connections API path segment). */
export function smitheryConnectionId(server: Pick<McpServerConfig, "id">): string {
  return sanitizeMcpInstallId(server.id);
}

/** Official Smithery Connect MCP proxy — routes through credential vault (not upstream Arcade). */
export function smitheryConnectProxyUrl(namespace: string, connectionId: string): string {
  return `https://api.smithery.ai/connect/${encodeURIComponent(namespace)}/${encodeURIComponent(connectionId)}/mcp`;
}

export function isSmitheryConnectProxyUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    return (
      parsed.hostname.toLowerCase() === "api.smithery.ai" &&
      /^\/connect\/[^/]+\/[^/]+\/mcp\/?$/i.test(parsed.pathname)
    );
  } catch {
    return false;
  }
}

export type SmitheryConnectionStatus = {
  state: "connected" | "auth_required" | "input_required" | "error" | string;
  setupUrl?: string;
  message?: string;
};

export type SmitheryConnectionUpsertResult = SmitheryConnectionStatus & {
  namespace: string;
  connectionId: string;
};

/** Delete a Smithery Connection so the next upsert can re-enter OAuth (setupUrl). */
export async function deleteSmitheryConnection(
  server: Pick<McpServerConfig, "id">,
  apiKey: string,
): Promise<void> {
  const key = apiKey.trim();
  if (!key) throw new Error("Smithery API key is required");
  const namespace = await resolveSmitheryNamespace(key);
  const connectionId = smitheryConnectionId(server);
  const response = await fetch(
    `${CONNECTIONS_BASE}/${encodeURIComponent(namespace)}/${encodeURIComponent(connectionId)}`,
    {
      method: "DELETE",
      headers: { Authorization: `Bearer ${key}` },
    },
  );
  if (response.ok || response.status === 404) return;
  const data = await response.json().catch(() => ({}) as Record<string, unknown>);
  const message = (data as { message?: string }).message || (data as { error?: string }).error;
  throw new Error(message || `Smithery delete connection error (${response.status})`);
}

/**
 * Create/update a Smithery Connection for this server via the official Connections API
 * and return its status — including the authoritative `setupUrl` for OAuth/config, so we
 * never have to guess which URL from mcp-remote's logs is the real one.
 *
 * Prefer registry `server` (qualifiedName) over raw `mcpUrl` so Smithery vaults upstream
 * OAuth (Gmail/Arcade) correctly — see https://smithery.ai/docs/use/connect
 */
export async function upsertSmitheryConnection(
  server: Pick<McpServerConfig, "id" | "qualifiedName" | "title">,
  mcpUrl: string,
  apiKey: string,
): Promise<SmitheryConnectionUpsertResult> {
  const key = apiKey.trim();
  if (!key) throw new Error("Smithery API key is required");
  const namespace = await resolveSmitheryNamespace(key);
  const connectionId = smitheryConnectionId(server);
  const qualifiedName = server.qualifiedName?.trim();
  const body: Record<string, string> = {
    name: server.title?.trim() || connectionId,
  };
  // Registry servers: `server` lets Smithery own OAuth/credential vaulting.
  // Fallback to mcpUrl for custom/non-registry endpoints.
  if (qualifiedName) {
    body.server = qualifiedName;
  } else {
    body.mcpUrl = mcpUrl;
  }
  const response = await fetch(
    `${CONNECTIONS_BASE}/${encodeURIComponent(namespace)}/${encodeURIComponent(connectionId)}`,
    {
      method: "PUT",
      headers: {
        Authorization: `Bearer ${key}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(body),
    },
  );
  const data = await response.json().catch(() => ({}) as Record<string, unknown>);
  const status = (data as { status?: SmitheryConnectionStatus }).status;
  if (!response.ok && !status) {
    // Retry once with mcpUrl if `server` was rejected (older namespaces / unknown slug).
    if (qualifiedName && mcpUrl) {
      const fallback = await fetch(
        `${CONNECTIONS_BASE}/${encodeURIComponent(namespace)}/${encodeURIComponent(connectionId)}`,
        {
          method: "PUT",
          headers: {
            Authorization: `Bearer ${key}`,
            "Content-Type": "application/json",
          },
          body: JSON.stringify({ mcpUrl, name: body.name }),
        },
      );
      const fallbackData = await fallback.json().catch(() => ({}) as Record<string, unknown>);
      const fallbackStatus = (fallbackData as { status?: SmitheryConnectionStatus }).status;
      if (fallback.ok || fallbackStatus) {
        return {
          namespace,
          connectionId,
          state: fallbackStatus?.state ?? "error",
          setupUrl:
            fallbackStatus?.setupUrl ||
            (fallbackStatus as { authorizationUrl?: string } | undefined)?.authorizationUrl,
          message: fallbackStatus?.message,
        };
      }
    }
    const message = (data as { message?: string }).message || (data as { error?: string }).error;
    throw new Error(message || `Smithery connection error (${response.status})`);
  }
  return {
    namespace,
    connectionId,
    state: status?.state ?? "error",
    setupUrl:
      status?.setupUrl || (status as { authorizationUrl?: string } | undefined)?.authorizationUrl,
    message: status?.message,
  };
}

export function isSmitheryHostedServer(
  server: Pick<McpServerConfig, "args" | "source" | "homepage">,
): boolean {
  if (server.source === "smithery") return true;
  const hay = [...(server.args ?? []), server.homepage ?? ""].join(" ").toLowerCase();
  return hay.includes("smithery.ai") || hay.includes("run.tools");
}

export type SmitheryInstallPlan = {
  install: McpServerConfig;
  /** Env keys the user should fill (from configSchema.required). */
  requiredEnv: Array<{ name: string; description?: string; secret?: boolean }>;
  deploymentUrl: string;
};

/** Build an Anya stdio install that bridges to Smithery's hosted HTTP MCP. */
export function buildSmitheryMcpInstall(
  detail: SmitheryMcpServerDetail,
  options?: { apiKey?: string },
): SmitheryInstallPlan | null {
  const deploymentUrl = resolveSmitheryDeploymentUrl(detail);
  if (!deploymentUrl) return null;

  const id = mcpInstallId(detail);
  const http = (detail.connections ?? []).find(
    (conn) => (conn.type ?? "").toLowerCase() === "http",
  );
  const schema = http?.configSchema;
  const required = schema?.required ?? [];
  const requiredEnv = required.map((name) => {
    const prop = schema?.properties?.[name];
    return {
      name: name.toUpperCase().replace(/[^A-Z0-9_]+/g, "_"),
      description: prop?.description || prop?.title || name,
      secret: /key|token|secret|password/i.test(name),
    };
  });

  const remoteUrl = withSmitheryApiKey(deploymentUrl, options?.apiKey ?? "");

  return {
    deploymentUrl,
    requiredEnv,
    install: {
      id,
      title: detail.displayName || id,
      description: detail.description || "",
      command: "npx",
      args: ["-y", MCP_REMOTE_PACKAGE, remoteUrl],
      env: [],
      enabled: true,
      iconUrl: detail.iconUrl?.trim() || undefined,
      qualifiedName: detail.qualifiedName?.trim() || undefined,
      registryId: detail.id?.trim() || undefined,
      homepage: detail.homepage?.trim() || undefined,
      source: "smithery",
    },
  };
}
