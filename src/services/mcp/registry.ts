import type { McpServerConfig } from "@/types/setting";

const REGISTRY_BASE = "https://registry.modelcontextprotocol.io/v0.1";

export type RegistryPackage = {
  registryType?: string;
  identifier?: string;
  version?: string;
  runtimeHint?: string;
  transport?: { type?: string };
  runtimeArguments?: Array<{ value?: string; type?: string; name?: string }>;
  packageArguments?: Array<{
    name?: string;
    type?: string;
    value?: string;
    default?: string;
    isRequired?: boolean;
    description?: string;
  }>;
  environmentVariables?: Array<{
    name: string;
    description?: string;
    isRequired?: boolean;
    isSecret?: boolean;
    value?: string;
    default?: string;
  }>;
};

export type RegistryServer = {
  name: string;
  title?: string;
  description?: string;
  version?: string;
  websiteUrl?: string;
  repository?: { url?: string };
  packages?: RegistryPackage[];
};

export type CatalogEntry = {
  name: string;
  title: string;
  description: string;
  version?: string;
  websiteUrl?: string;
  package: RegistryPackage;
  install: McpServerConfig;
  requiredEnv: Array<{ name: string; description?: string; secret?: boolean }>;
  source: "registry" | "curated";
};

type ListResponse = {
  servers?: Array<{ server?: RegistryServer }>;
  metadata?: { nextCursor?: string; count?: number };
};

function curatedNpm(
  id: string,
  title: string,
  description: string,
  npmPackage: string,
  extraArgs: string[] = [],
  requiredEnv: CatalogEntry["requiredEnv"] = [],
): CatalogEntry {
  return {
    name: `curated/${id}`,
    title,
    description,
    package: {
      registryType: "npm",
      identifier: npmPackage,
      runtimeHint: "npx",
      transport: { type: "stdio" },
    },
    install: {
      id,
      title,
      description,
      command: "npx",
      args: ["-y", npmPackage, ...extraArgs],
      env: [],
      enabled: false,
    },
    requiredEnv,
    source: "curated",
  };
}

function curatedPypi(
  id: string,
  title: string,
  description: string,
  pypiPackage: string,
  extraArgs: string[] = [],
  requiredEnv: CatalogEntry["requiredEnv"] = [],
): CatalogEntry {
  return {
    name: `curated/${id}`,
    title,
    description,
    package: {
      registryType: "pypi",
      identifier: pypiPackage,
      runtimeHint: "uvx",
      transport: { type: "stdio" },
    },
    install: {
      id,
      title,
      description,
      command: "uvx",
      args: [pypiPackage, ...extraArgs],
      env: [],
      enabled: false,
    },
    requiredEnv,
    source: "curated",
  };
}

export const CURATED_MCP_CATALOG: CatalogEntry[] = [
  curatedNpm(
    "filesystem",
    "Filesystem",
    "官方文件系统 MCP：读写本地目录（参数为允许访问的路径）。",
    "@modelcontextprotocol/server-filesystem",
    ["."],
  ),
  curatedNpm(
    "memory",
    "Memory",
    "官方知识图谱记忆 MCP，适合跨会话记要点。",
    "@modelcontextprotocol/server-memory",
  ),
  curatedNpm(
    "sequential-thinking",
    "Sequential Thinking",
    "官方分步思考工具，帮助拆解复杂任务。",
    "@modelcontextprotocol/server-sequential-thinking",
  ),
  curatedNpm(
    "everything",
    "Everything",
    "官方示例 MCP，包含 prompts / resources / tools，便于联调。",
    "@modelcontextprotocol/server-everything",
  ),
  curatedPypi(
    "fetch",
    "Fetch",
    "官方网页抓取 MCP，拉取并转换 URL 内容（PyPI / uvx）。",
    "mcp-server-fetch",
  ),
  curatedPypi(
    "time",
    "Time",
    "官方时间工具：时区转换与当前时间（PyPI / uvx）。",
    "mcp-server-time",
  ),
  curatedNpm(
    "brave-search",
    "Brave Search",
    "Brave 网页搜索（需要 BRAVE_API_KEY）。",
    "@modelcontextprotocol/server-brave-search",
    [],
    [{ name: "BRAVE_API_KEY", description: "Brave Search API key", secret: true }],
  ),
  curatedNpm(
    "github",
    "GitHub",
    "官方 GitHub MCP（需要 GITHUB_PERSONAL_ACCESS_TOKEN）。",
    "@modelcontextprotocol/server-github",
    [],
    [
      {
        name: "GITHUB_PERSONAL_ACCESS_TOKEN",
        description: "GitHub personal access token",
        secret: true,
      },
    ],
  ),
  curatedNpm(
    "gitlab",
    "GitLab",
    "官方 GitLab MCP（需要 GITLAB_PERSONAL_ACCESS_TOKEN 等）。",
    "@modelcontextprotocol/server-gitlab",
    [],
    [
      {
        name: "GITLAB_PERSONAL_ACCESS_TOKEN",
        description: "GitLab personal access token",
        secret: true,
      },
    ],
  ),
  curatedNpm(
    "google-maps",
    "Google Maps",
    "官方 Google Maps MCP（需要 GOOGLE_MAPS_API_KEY）。",
    "@modelcontextprotocol/server-google-maps",
    [],
    [{ name: "GOOGLE_MAPS_API_KEY", description: "Google Maps API key", secret: true }],
  ),
  curatedNpm(
    "slack",
    "Slack",
    "官方 Slack MCP（需要 SLACK_BOT_TOKEN / SLACK_TEAM_ID）。",
    "@modelcontextprotocol/server-slack",
    [],
    [
      { name: "SLACK_BOT_TOKEN", description: "Slack bot token", secret: true },
      { name: "SLACK_TEAM_ID", description: "Slack team/workspace id" },
    ],
  ),
  curatedNpm(
    "postgres",
    "PostgreSQL",
    "官方 PostgreSQL 只读查询 MCP（需要 DATABASE_URL）。",
    "@modelcontextprotocol/server-postgres",
    [],
    [{ name: "DATABASE_URL", description: "Postgres connection string", secret: true }],
  ),
  curatedPypi(
    "sqlite",
    "SQLite",
    "官方 SQLite MCP，适合本地数据库探索（PyPI / uvx）。",
    "mcp-server-sqlite",
    ["."],
  ),
  curatedNpm(
    "puppeteer",
    "Puppeteer",
    "官方浏览器自动化 MCP（截图、点击、填表等）。",
    "@modelcontextprotocol/server-puppeteer",
  ),
  curatedNpm(
    "redis",
    "Redis",
    "官方 Redis MCP（需要 REDIS_URL）。",
    "@modelcontextprotocol/server-redis",
    [],
    [{ name: "REDIS_URL", description: "Redis connection URL", secret: true }],
  ),
  curatedPypi("git", "Git", "官方 Git 仓库操作 MCP（PyPI / uvx）。", "mcp-server-git"),
];

function shortIdFromName(name: string) {
  const leaf = name.split("/").pop() ?? name;
  return leaf
    .replace(/^mcp-server-/, "")
    .replace(/^server-/, "")
    .replace(/[^a-zA-Z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 48)
    .toLowerCase();
}

function pickStdioPackage(server: RegistryServer): RegistryPackage | null {
  const packages = server.packages ?? [];
  // Anya only supports local stdio over npm / PyPI (npx / uvx). Skip remote,
  // OCI, and other transports so they never appear as "installable".
  const installable = packages.filter((pkg) => {
    if (!pkg.identifier) return false;
    if (pkg.registryType !== "npm" && pkg.registryType !== "pypi") return false;
    const transport = pkg.transport?.type ?? "stdio";
    return transport === "stdio";
  });
  return (
    installable.find((pkg) => pkg.registryType === "npm") ??
    installable.find((pkg) => pkg.registryType === "pypi") ??
    null
  );
}

export function packageToInstall(
  server: RegistryServer,
  pkg: RegistryPackage,
): { install: McpServerConfig; requiredEnv: CatalogEntry["requiredEnv"] } {
  const id = shortIdFromName(server.name);
  const requiredEnv =
    (pkg.environmentVariables ?? [])
      .filter((item) => item.isRequired)
      .map((item) => ({
        name: item.name,
        description: item.description,
        secret: item.isSecret,
      })) ?? [];

  const env: Array<[string, string]> = [];
  for (const item of pkg.environmentVariables ?? []) {
    const value = item.value ?? item.default;
    if (value) env.push([item.name, value]);
  }

  const title = server.title || server.name.split("/").pop() || server.name;
  const description = server.description || "";

  if (pkg.registryType === "pypi") {
    return {
      install: {
        id,
        title,
        description,
        command: pkg.runtimeHint || "uvx",
        args: [pkg.identifier!],
        env,
        enabled: true,
      },
      requiredEnv,
    };
  }

  const runtimeArgs = (pkg.runtimeArguments ?? [])
    .map((arg) => arg.value)
    .filter((value): value is string => Boolean(value));
  if (!runtimeArgs.includes("-y") && (pkg.runtimeHint || "npx") === "npx") {
    runtimeArgs.unshift("-y");
  }
  const packageArgs: string[] = [];
  for (const arg of pkg.packageArguments ?? []) {
    if (arg.type === "named" && arg.name) {
      const value = arg.value ?? arg.default;
      if (value != null && value !== "") {
        packageArgs.push(arg.name.startsWith("-") ? arg.name : `--${arg.name}`, String(value));
      }
    } else if (arg.type === "positional") {
      const value = arg.value ?? arg.default;
      if (value != null && value !== "") packageArgs.push(String(value));
    }
  }

  return {
    install: {
      id,
      title,
      description,
      command: pkg.runtimeHint || "npx",
      args: [...runtimeArgs, pkg.identifier!, ...packageArgs],
      env,
      enabled: true,
    },
    requiredEnv,
  };
}

export function toCatalogEntry(
  server: RegistryServer,
  source: "registry" | "curated" = "registry",
): CatalogEntry | null {
  const pkg = pickStdioPackage(server);
  if (!pkg) return null;
  const { install, requiredEnv } = packageToInstall(server, pkg);
  return {
    name: server.name,
    title: server.title || server.name.split("/").pop() || server.name,
    description: server.description || "",
    version: server.version,
    websiteUrl: server.websiteUrl || server.repository?.url,
    package: pkg,
    install,
    requiredEnv,
    source,
  };
}

export type RegistrySearchResult = {
  entries: CatalogEntry[];
  nextCursor?: string;
  /** Raw registry rows scanned while collecting stdio packages. */
  scanned: number;
};

/**
 * Official registry pages are dominated by remote HTTP servers.
 * We page through results until we collect enough installable stdio packages.
 */
export async function searchMcpRegistry(
  query: string,
  options: {
    desired?: number;
    pageSize?: number;
    maxPages?: number;
    cursor?: string;
  } = {},
): Promise<RegistrySearchResult> {
  const desired = options.desired ?? 60;
  const pageSize = options.pageSize ?? 100;
  const maxPages = options.maxPages ?? 12;
  const trimmed = query.trim();

  const entries: CatalogEntry[] = [];
  const seen = new Set<string>();
  let cursor = options.cursor;
  let scanned = 0;
  let pages = 0;

  while (entries.length < desired && pages < maxPages) {
    const params = new URLSearchParams({
      limit: String(pageSize),
      version: "latest",
    });
    if (trimmed) params.set("search", trimmed);
    if (cursor) params.set("cursor", cursor);

    const response = await fetch(`${REGISTRY_BASE}/servers?${params.toString()}`, {
      headers: { Accept: "application/json" },
    });
    if (!response.ok) {
      throw new Error(`MCP Registry HTTP ${response.status}`);
    }
    const data = (await response.json()) as ListResponse;
    const batch = data.servers ?? [];
    scanned += batch.length;
    pages += 1;

    for (const item of batch) {
      const server = item.server;
      if (!server) continue;
      const entry = toCatalogEntry(server, "registry");
      if (!entry || seen.has(entry.name)) continue;
      seen.add(entry.name);
      entries.push(entry);
      if (entries.length >= desired) break;
    }

    cursor = data.metadata?.nextCursor;
    if (!cursor || batch.length === 0) {
      cursor = undefined;
      break;
    }
  }

  return { entries, nextCursor: cursor, scanned };
}

export function filterCurated(query: string): CatalogEntry[] {
  const q = query.trim().toLowerCase();
  if (!q) return CURATED_MCP_CATALOG;
  return CURATED_MCP_CATALOG.filter((entry) => {
    const hay =
      `${entry.title} ${entry.name} ${entry.description} ${entry.install.id}`.toLowerCase();
    return hay.includes(q);
  });
}

export type McpRuntimeSupport = {
  npm: boolean;
  pypi: boolean;
  nodePath?: string | null;
  npxCliPath?: string | null;
  uvxPath?: string | null;
};

export function entryRuntimeKind(entry: CatalogEntry): "npm" | "pypi" | "other" {
  const registryType = (entry.package.registryType ?? "").toLowerCase();
  const command = (entry.install.command ?? "").toLowerCase();
  if (registryType === "npm" || command === "npx" || command === "npm") return "npm";
  if (registryType === "pypi" || command === "uvx" || command === "uv") return "pypi";
  return "other";
}

export function isEntryInstallable(entry: CatalogEntry, support: McpRuntimeSupport): boolean {
  // Next version: guide users through required API keys / env vars.
  if ((entry.requiredEnv ?? []).length > 0) return false;
  switch (entryRuntimeKind(entry)) {
    case "npm":
      return support.npm;
    case "pypi":
      return support.pypi;
    default:
      return false;
  }
}

export function filterInstallable(
  entries: CatalogEntry[],
  support: McpRuntimeSupport,
): CatalogEntry[] {
  return entries.filter((entry) => isEntryInstallable(entry, support));
}
