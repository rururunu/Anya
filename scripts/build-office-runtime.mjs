import { build } from "esbuild";
import { existsSync, readFileSync } from "node:fs";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { readdir } from "node:fs/promises";
import { builtinModules, createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { bundleKit } from "./office/bundle-kit.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const output = join(root, "src-tauri/resources/office");
const require = createRequire(join(root, "package.json"));

async function officeOutputsFresh() {
  const engine = ["win32", "darwin"].includes(process.platform)
    ? `@deepseek-ai/libreoffice-kit-${process.platform}-${process.arch}`
    : "@deepseek-ai/libreoffice-kit-wasm";
  const runtimeOut = join(output, "runtime.mjs");
  const files = [
    "cli.mjs",
    "kit/worker.mjs",
    "kit/BUILD.json",
    "THIRD_PARTY_LICENSES.txt",
    process.platform === "win32" ? "node.exe" : "node",
    `kit/node_modules/${engine}`,
  ];
  if (!existsSync(runtimeOut) || files.some((file) => !existsSync(join(output, file))))
    return false;

  const packageVersion = (name) =>
    JSON.parse(readFileSync(require.resolve(`${name}/package.json`), "utf8")).version;
  try {
    const recorded = JSON.parse(await readFile(join(output, "kit/BUILD.json"), "utf8"));
    if (
      recorded.api !== packageVersion("@deepseek-ai/libreoffice-kit") ||
      recorded.engine !== engine ||
      recorded.engineVersion !== packageVersion(engine) ||
      recorded.node !== process.version ||
      recorded.platform !== process.platform ||
      recorded.arch !== process.arch
    )
      return false;
  } catch {
    return false;
  }
  const builtAt = (await stat(runtimeOut)).mtimeMs;
  for (const file of [
    "scripts/build-office-runtime.mjs",
    "scripts/office/runtime.mjs",
    "scripts/office/cli.mjs",
    "scripts/office/kit-worker.mjs",
    "scripts/office/bundle-kit.mjs",
  ]) {
    if ((await stat(join(root, file))).mtimeMs > builtAt) return false;
  }
  return true;
}

if (await officeOutputsFresh()) {
  console.log("Office runtime is up to date; skipping bundle.");
  process.exit(0);
}
await mkdir(output, { recursive: true });
const built = await build({
  entryPoints: [join(root, "scripts/office/runtime.mjs")],
  outfile: join(output, "runtime.mjs"),
  bundle: true,
  platform: "node",
  format: "esm",
  target: "es2022",
  minify: true,
  metafile: true,
  plugins: [
    {
      name: "deno-node-builtins",
      setup(plugin) {
        plugin.onResolve({ filter: /.*/ }, ({ path }) =>
          builtinModules.includes(path)
            ? { path: path.startsWith("node:") ? path : `node:${path}`, external: true }
            : undefined,
        );
      },
    },
  ],
  legalComments: "eof",
  banner: {
    js: 'import { createRequire as __anyaCreateRequire } from "node:module"; const require = __anyaCreateRequire(import.meta.url);',
  },
});
// Node built-ins must keep node: specifiers: Deno never resolves bare "fs" as npm.
const bundle = await readFile(join(output, "runtime.mjs"), "utf8");
await writeFile(
  join(output, "runtime.mjs"),
  bundle.replace(
    /require\("(assert|buffer|child_process|crypto|events|fs|http|https|os|path|stream|timers|url|util|zlib|string_decoder)"\)/g,
    'require("node:$1")',
  ),
);
await writeFile(join(output, "cli.mjs"), await readFile(join(root, "scripts/office/cli.mjs")));
// Include notices for all bundled packages, including transitive dependencies.
const packageRoots = new Map();
for (const input of Object.keys(built.metafile.inputs)) {
  let folder = dirname(join(root, input));
  while (folder !== dirname(folder)) {
    try {
      const metadata = JSON.parse(await readFile(join(folder, "package.json"), "utf8"));
      if (metadata.name && folder.includes("node_modules")) {
        packageRoots.set(folder, metadata);
        break;
      }
    } catch {
      /* continue toward package root */
    }
    folder = dirname(folder);
  }
}
const notices = [];
for (const [packageRoot, metadata] of packageRoots) {
  let license = "";
  for (const filename of (await readdir(packageRoot)).filter((name) => /^licen[cs]e/i.test(name))) {
    try {
      license = await readFile(join(packageRoot, filename), "utf8");
      break;
    } catch {
      /* try next */
    }
  }
  notices.push(
    `${metadata.name} ${metadata.version}\nLicense: ${metadata.license ?? "see retained source notices"}\n${license || JSON.stringify({ author: metadata.author, repository: metadata.repository })}`,
  );
}
const kitNotices = await bundleKit(root, output);
await writeFile(
  join(output, "THIRD_PARTY_LICENSES.txt"),
  notices.join("\n\n") + "\n\n" + kitNotices,
);
console.log("Office authoring runtime and LibreOffice Kit bundled for offline execution.");
