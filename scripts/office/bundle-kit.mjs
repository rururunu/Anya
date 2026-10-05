import { cp, mkdir, readFile, writeFile, readdir, copyFile, realpath } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { existsSync } from "node:fs";
import { pathToFileURL } from "node:url";

/** Copy the complete selected engine and its production dependency graph.
 * Flat copied packages keep npm resolution without junctions or source paths.
 */
export async function bundleKit(root, output) {
  const modules = join(output, "kit/node_modules");
  const seen = new Map();
  const sourceRequire = createRequire(join(root, "package.json"));
  async function packageRoot(name, from) {
    let folder = from;
    while (true) {
      const candidate = join(folder, "node_modules", name);
      if (existsSync(join(candidate, "package.json"))) return await realpath(candidate);
      const parent = dirname(folder);
      if (parent === folder)
        throw new Error(`Required bundled Office dependency is missing: ${name}`);
      folder = parent;
    }
  }
  async function copyPackage(name, from, optional = false) {
    let source;
    try {
      source = await packageRoot(name, from);
    } catch (error) {
      if (optional) return;
      throw error;
    }
    const metadata = JSON.parse(await readFile(join(source, "package.json"), "utf8"));
    if (seen.has(name)) {
      if (seen.get(name).version !== metadata.version)
        throw new Error(`Conflicting Office dependency versions for ${name}`);
      return;
    }
    seen.set(name, metadata);
    const dest = join(modules, name);
    await mkdir(dest, { recursive: true });
    for (const entry of await readdir(source)) {
      if (entry === "node_modules") continue;
      await cp(join(source, entry), join(dest, entry), {
        recursive: true,
        dereference: true,
        force: true,
      });
    }
    for (const dependency of Object.keys(metadata.dependencies ?? {}))
      await copyPackage(dependency, source);
    for (const dependency of Object.keys(metadata.optionalDependencies ?? {})) {
      // The entry package requires exactly the matching engine, not all platforms.
      if (dependency.startsWith("@deepseek-ai/libreoffice-kit-")) continue;
      await copyPackage(dependency, source, true);
    }
  }
  await copyPackage("@deepseek-ai/libreoffice-kit", root);
  const kitRoot = await packageRoot("@deepseek-ai/libreoffice-kit", root);
  const kit = seen.get("@deepseek-ai/libreoffice-kit");
  const engine = ["win32", "darwin"].includes(process.platform)
    ? `@deepseek-ai/libreoffice-kit-${process.platform}-${process.arch}`
    : "@deepseek-ai/libreoffice-kit-wasm";
  if (!kit.optionalDependencies?.[engine])
    throw new Error(`LibreOffice Kit declares no engine for ${process.platform}/${process.arch}`);
  await copyPackage(engine, kitRoot);
  const { discoverRuntime } = await import(
    pathToFileURL(sourceRequire.resolve("@deepseek-ai/libreoffice-kit")).href
  );
  await discoverRuntime();
  await copyFile(
    process.execPath,
    join(output, process.platform === "win32" ? "node.exe" : "node"),
  );
  const nodeLicense = join(dirname(process.execPath), "LICENSE");
  if (!existsSync(nodeLicense) && process.version !== "v24.14.0")
    throw new Error("Provide the matching Node LICENSE alongside the build executable.");
  await copyFile(
    existsSync(nodeLicense) ? nodeLicense : join(root, "scripts/office/NODE-LICENSE.txt"),
    join(output, "NODE-LICENSE.txt"),
  );
  // Keep this module next to its dependency graph; do not bundle worker URLs or engine assets.
  await copyFile(join(root, "scripts/office/kit-worker.mjs"), join(output, "kit/worker.mjs"));
  await writeFile(join(output, "kit/package.json"), '{"private":true,"type":"module"}\n');
  await writeFile(
    join(output, "kit/BUILD.json"),
    JSON.stringify(
      {
        api: kit.version,
        engine,
        engineVersion: seen.get(engine).version,
        platform: process.platform,
        arch: process.arch,
        node: process.version,
        source: "https://github.com/deepseek-ai/dsh-libreoffice-kit",
      },
      null,
      2,
    ) + "\n",
  );
  const notices = [...seen.values()]
    .map(
      (metadata) =>
        `${metadata.name} ${metadata.version}: ${metadata.license ?? "see package notices"}`,
    )
    .join("\n");
  console.log(
    `Bundled LibreOffice Kit ${kit.version}, ${engine}; ${seen.size} production packages.`,
  );
  return (
    notices +
    "\nCorresponding source: https://github.com/deepseek-ai/dsh-libreoffice-kit\nEngine sources, recipes and third-party licenses are retained inside the selected engine package.\n"
  );
}
