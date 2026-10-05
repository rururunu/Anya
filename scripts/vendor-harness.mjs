// Import the local official checkout once; application builds use the snapshot.
import { cp, mkdir, readFile, readdir, realpath, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { execFileSync } from 'node:child_process';

const checkout = resolve(process.argv[2] ?? '../deepseek-harness');
const destination = resolve('src-tauri/harness/vendor');
const commit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: checkout, encoding: 'utf8' }).trim();
const graph = {};
async function locate(name, from) {
  for (let dir = from; ; dir = dirname(dir)) {
    const candidate = join(dir, 'node_modules', name);
    if (existsSync(join(candidate, 'package.json'))) return realpath(candidate);
    if (dirname(dir) === dir) throw new Error(`Missing production dependency ${name} from ${from}`);
  }
}
async function visit(name, from, optional = false, exact) {
  let source;
  try { source = exact ?? await locate(name, from); } catch (e) { if (optional) return; throw e; }
  const pkg = JSON.parse(await readFile(join(source, 'package.json'), 'utf8'));
  const compatible = (items, value) => !items || (items.includes(value) || items.includes('any')) && !items.includes('!' + value);
  if (!compatible(pkg.os, process.platform) || !compatible(pkg.cpu, process.arch)) {
    if (optional) return;
    throw new Error(`Unsupported required package ${name}`);
  }
  const id = name.replace(/[@/]/g, '_') + '-' + pkg.version;
  if (graph[id]) return id;
  graph[id] = { name, version: pkg.version, dependencies: {}, official: source.startsWith(checkout) && !source.includes('.pnpm') };
  const target = join(destination, 'packages', id);
  await mkdir(target, { recursive: true });
  for (const entry of await readdir(source)) {
    if (entry === 'node_modules' || entry === '.git') continue;
    await cp(join(source, entry), join(target, entry), { recursive: true, dereference: true });
  }
  for (const dep of Object.keys(pkg.dependencies ?? {})) graph[id].dependencies[dep] = await visit(dep, source);
  for (const dep of Object.keys(pkg.optionalDependencies ?? {})) {
    const child = await visit(dep, source, true);
    if (child) graph[id].dependencies[dep] = child;
  }
  return id;
}
const roots = {};
roots['@deepseek-ai/dsh'] = await visit('@deepseek-ai/dsh', checkout, false, join(checkout, 'apps/cli'));
roots['@deepseek-ai/dsh-sdk-client'] = await visit('@deepseek-ai/dsh-sdk-client', checkout, false, join(checkout, 'packages/sdk/client'));
await writeFile(join(destination, 'manifest.json'), JSON.stringify({ commit, platform: process.platform, arch: process.arch, roots, packages: graph }, null, 2) + '\n');
await cp(join(checkout, 'LICENSE'), join(destination, 'LICENSE'));
await cp(join(checkout, 'THIRD_PARTY_NOTICES.md'), join(destination, 'THIRD_PARTY_NOTICES.md'));
console.log(`Imported official harness ${commit}: ${Object.keys(graph).length} production packages, including original source and built assets.`);
