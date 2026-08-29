import { access, mkdir, readFile, writeFile, readdir } from "node:fs/promises";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Generate `release/latest.json` for GitHub Releases + Tauri Updater.
 *
 * Usage:
 *   node scripts/generate-latest-json.mjs --tag v0.2.1
 *   node scripts/generate-latest-json.mjs --tag v0.2.14 --notes "Release notes"
 *
 * Writes `release/latest.json` (required GitHub asset name) and
 * `release/Anya_<version>_latest_<notes-slug>.json`.
 *
 * Requires a signed release build:
 *   src-tauri/target/release/bundle/msi/{productName}_<version>_x64.msi
 *   src-tauri/target/release/bundle/msi/{productName}_<version>_x64.msi.sig
 */
const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const msiDir = join(root, "src-tauri", "target", "release", "bundle", "msi");
const outDir = join(root, "release");
const outFile = join(outDir, "latest.json");
const defaultRepo = "rururunu/Anya";

function parseArgs(argv) {
  const options = {
    tag: "",
    notes: "",
    repo: process.env.GITHUB_REPOSITORY || defaultRepo,
    pubDate: new Date().toISOString(),
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--tag") options.tag = argv[++i] ?? "";
    else if (arg === "--notes") options.notes = argv[++i] ?? "";
    else if (arg === "--repo") options.repo = argv[++i] ?? defaultRepo;
    else if (arg === "--pub-date") options.pubDate = argv[++i] ?? options.pubDate;
  }

  return options;
}

async function readJson(path) {
  const raw = await readFile(path, "utf8");
  return JSON.parse(raw);
}

async function findMsiPair(version, productName) {
  const files = await readdir(msiDir);
  const msiName = `${productName}_${version}_x64.msi`;
  const sigName = `${msiName}.sig`;

  if (!files.includes(msiName)) {
    throw new Error(`MSI not found: ${join(msiDir, msiName)}. Run pnpm tauri:build with signing first.`);
  }
  if (!files.includes(sigName)) {
    throw new Error(
      `Signature not found: ${join(msiDir, sigName)}. Set TAURI_SIGNING_PRIVATE_KEY before build.`,
    );
  }

  return { msiName, sigName };
}

const options = parseArgs(process.argv.slice(2));
if (!options.tag) {
  console.error("[generate-latest-json] missing --tag (example: --tag v0.2.1)");
  process.exit(1);
}

const tag = options.tag.startsWith("v") ? options.tag : `v${options.tag}`;
const version = tag.replace(/^v/, "");

try {
  await access(msiDir);
} catch {
  console.error(`[generate-latest-json] bundle directory not found: ${msiDir}`);
  process.exit(1);
}

const config = await readJson(join(root, "src-tauri", "tauri.conf.json"));
if (config.version !== version) {
  console.warn(
    `[generate-latest-json] warning: tauri.conf.json version is ${config.version}, tag is ${version}`,
  );
}

const productName = typeof config.productName === "string" ? config.productName : "Anya";
const { msiName, sigName } = await findMsiPair(version, productName);
const signature = (await readFile(join(msiDir, sigName), "utf8")).trim();
const downloadUrl = `https://github.com/${options.repo}/releases/download/${tag}/${msiName}`;

const latest = {
  version,
  notes: options.notes,
  pub_date: options.pubDate,
  platforms: {
    "windows-x86_64": {
      url: downloadUrl,
      signature,
    },
  },
};

await mkdir(outDir, { recursive: true });
await writeFile(outFile, `${JSON.stringify(latest, null, 2)}\n`, "utf8");

const notesSlug = String(options.notes)
  .replace(/[\\/:*?"<>|]/g, "")
  .replace(/\s+/g, "")
  .slice(0, 32);
const namedFile = notesSlug
  ? join(outDir, `Anya_${version}_latest_${notesSlug}.json`)
  : join(outDir, `Anya_${version}_latest.json`);
await writeFile(namedFile, `${JSON.stringify(latest, null, 2)}\n`, "utf8");

console.log(`[generate-latest-json] wrote ${outFile}`);
console.log(`[generate-latest-json] wrote ${namedFile}`);
console.log(`[generate-latest-json] upload to GitHub Release ${tag}:`);
console.log(`  - ${msiName}`);
console.log(`  - ${sigName}`);
console.log(`  - latest.json`);
