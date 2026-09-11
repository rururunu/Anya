import { createWriteStream, existsSync, mkdirSync, rmSync } from "node:fs";
import { chmod, copyFile, mkdtemp, readdir, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { Readable } from "node:stream";

const RIPGREP_VERSION = "14.1.1";
const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const binariesDir = join(root, "src-tauri", "binaries");

const HOST_TARGETS = {
  "win32-x64": {
    triple: "x86_64-pc-windows-msvc",
    archive: `ripgrep-${RIPGREP_VERSION}-x86_64-pc-windows-msvc.zip`,
    exe: "rg.exe",
  },
  "win32-arm64": {
    triple: "aarch64-pc-windows-msvc",
    archive: `ripgrep-${RIPGREP_VERSION}-aarch64-pc-windows-msvc.zip`,
    exe: "rg.exe",
  },
  "darwin-arm64": {
    triple: "aarch64-apple-darwin",
    archive: `ripgrep-${RIPGREP_VERSION}-aarch64-apple-darwin.tar.gz`,
    exe: "rg",
  },
  "darwin-x64": {
    triple: "x86_64-apple-darwin",
    archive: `ripgrep-${RIPGREP_VERSION}-x86_64-apple-darwin.tar.gz`,
    exe: "rg",
  },
  "linux-x64": {
    triple: "x86_64-unknown-linux-musl",
    archive: `ripgrep-${RIPGREP_VERSION}-x86_64-unknown-linux-musl.tar.gz`,
    exe: "rg",
  },
  "linux-arm64": {
    triple: "aarch64-unknown-linux-gnu",
    archive: `ripgrep-${RIPGREP_VERSION}-aarch64-unknown-linux-gnu.tar.gz`,
    exe: "rg",
  },
};

function hostKey() {
  const platform = process.platform;
  const arch = process.arch === "arm64" ? "arm64" : process.arch === "x64" ? "x64" : process.arch;
  return `${platform}-${arch}`;
}

function destName(spec) {
  const suffix = spec.exe.endsWith(".exe") ? ".exe" : "";
  return `rg-${spec.triple}${suffix}`;
}

async function alreadyPresent(dest) {
  try {
    const info = await stat(dest);
    return info.isFile() && info.size > 100_000;
  } catch {
    return false;
  }
}

async function download(url, dest) {
  const response = await fetch(url);
  if (!response.ok || !response.body) {
    throw new Error(`download failed ${response.status} ${url}`);
  }
  await pipeline(Readable.fromWeb(response.body), createWriteStream(dest));
}

function extract(archive, outDir) {
  const result = spawnSync("tar", ["-xf", archive, "-C", outDir], { stdio: "inherit" });
  if (result.status !== 0) {
    throw new Error(`tar extract failed for ${archive}`);
  }
}

async function findExtractedBinary(dir, exeName) {
  const stack = [dir];
  while (stack.length) {
    const current = stack.pop();
    const entries = await readdir(current, { withFileTypes: true });
    for (const entry of entries) {
      const path = join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(path);
      } else if (entry.name === exeName) {
        return path;
      }
    }
  }
  return null;
}

async function fetchOne(spec, force) {
  mkdirSync(binariesDir, { recursive: true });
  const dest = join(binariesDir, destName(spec));
  if (!force && (await alreadyPresent(dest))) {
    console.log(`[fetch-rg] using cached ${destName(spec)}`);
    return dest;
  }
  const url = `https://github.com/BurntSushi/ripgrep/releases/download/${RIPGREP_VERSION}/${spec.archive}`;
  const tmp = await mkdtemp(join(tmpdir(), "anya-rg-"));
  const archivePath = join(tmp, spec.archive);
  try {
    console.log(`[fetch-rg] downloading ${spec.archive}`);
    await download(url, archivePath);
    extract(archivePath, tmp);
    const found = await findExtractedBinary(tmp, spec.exe);
    if (!found) {
      throw new Error(`rg binary not found in ${spec.archive}`);
    }
    await copyFile(found, dest);
    if (!dest.endsWith(".exe")) {
      await chmod(dest, 0o755);
    }
    console.log(`[fetch-rg] wrote ${destName(spec)}`);
    return dest;
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

const force = process.argv.includes("--force");
const all = process.argv.includes("--all");
const key = hostKey();
const host = HOST_TARGETS[key];

if (!all && !host) {
  console.warn(`[fetch-rg] no ripgrep build mapped for ${key}; search_files will use PATH rg or the WalkDir fallback`);
  process.exit(0);
}

const specs = all ? Object.values(HOST_TARGETS) : [host];
for (const spec of specs) {
  try {
    await fetchOne(spec, force);
  } catch (error) {
    if (all) {
      console.warn(`[fetch-rg] skip ${spec.triple}: ${error.message}`);
      continue;
    }
    console.error(`[fetch-rg] ${error.message}`);
    process.exit(1);
  }
}
