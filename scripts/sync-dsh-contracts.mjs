// Development-only importer. No harness runtime is shipped or started by Anya.
import { mkdir, readFile, writeFile, copyFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
const root = resolve(process.argv[2] ?? '../deepseek-harness');
const output = resolve('src-tauri/prompts/dsh');
await mkdir(output, { recursive: true });
const tools = new Map();
const sections = [];
const systemSource = await readFile(join(root, 'packages/core/system-prompt/src/index.ts'), 'utf8');
const orderBlock = systemSource.match(/const SECTION_ORDERS = \{([\s\S]*?)\}/)?.[1];
if (!orderBlock) throw new Error('Missing upstream section order');
const orders = Object.fromEntries([...orderBlock.matchAll(/([A-Z_]+):\s*(-?\d+)/g)].map(([, name, value]) => [name, Number(value)]));
const identity = systemSource.match(/name: 'harness:identity',[\s\S]*?text: '([^']+)'/)?.[1];
if (!identity) throw new Error('Missing upstream identity');
const noop = () => () => {};
const ctx = {
  tools: { register: tool => { tools.set(tool.name, tool); return noop(); }, get: name => tools.get(name) },
  systemPrompt: { section: section => { sections.push(section); return noop(); }, getSectionOrder: name => orders[name] ?? 0, context: noop, tools: noop },
  fs: {}, skills: {}, agents: {}, subagents: { getProvider: () => ({ name: 'spawn', inheritsParentContext: false, capabilities: { depthLimit: true } }), resolveMaxDepth: () => 1 }, jobs: { attachController: noop, events: { subscribe: noop } }, shell: {}, shellEnv: {}, web: {}, subprocess: {},
  sessionProjections: { register: noop }, on: noop, effect: noop,
  get: () => undefined,
  inject: (_names, callback) => { callback(ctx); return noop(); }, fiber: { state: 1 },
};
const load = async path => {
  const filename = join(root, 'packages', path, 'src/index.ts');
  if (!path.startsWith('shell/')) return import(pathToFileURL(filename).href);
  const require = createRequire(filename);
  const ts = createRequire(join(root, 'package.json'))('typescript');
  const source = (await readFile(filename, 'utf8')).replace(/import \{ FiberState \} from '@deepseek-ai\/cordis'/, 'const FiberState = { ACTIVE: 1 }');
  const js = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText;
  const resolved = js.replace(/from (['"])([^'"]+)\1/g, (_match, _quote, spec) => `from ${JSON.stringify(spec.startsWith('node:') ? spec : pathToFileURL(require.resolve(spec)).href)}`);
  return import('data:text/javascript;base64,' + Buffer.from(resolved).toString('base64'));
};
const fs = await load('fs/tool-fs');
fs.apply(ctx, { readLimit: 2000, readMaxLineLength: 2000, readMaxBytes: 51200, readStreamMinSize: 1024 * 1024 });
for (const [path, config] of [
  ['fs/tool-fs-search', { sampleOverCapGlobResults: false }], ['todo/tool-todo', { allowParallelInProgress: false }],
  ['interaction/tool-ask-user', {}], ['skill/tool-skill', {}], ['jobs/tool-jobs', {}],
  ['web/tool-web', {}], ['shell/tool-pwsh', {}], ['shell/tool-bash', {}],
  ['subagent/tool-subagent', { provider: 'spawn', enableRunInBackground: false, maxDepth: 1 }],
  ['deliverables/tool-present', {}],
]) {
  try { const mod = await load(path); mod.apply(ctx, mod.Config ? mod.Config(config) : config); } catch (e) { console.error(`${path}: ${e.message}`); throw e; }
}
const schemas = [...tools.values()].map(({ name, description, parameters }) => ({ type: 'function', function: { name, description, parameters } }));
// The Plan controller is a Cordis service; extract its static contract without constructing a host.
const planSource = await readFile(join(root, 'packages/plan/plan-mode/src/index.ts'), 'utf8');
const ts = createRequire(join(root, 'package.json'))('typescript');
const planAst = ts.createSourceFile('plan.ts', planSource, ts.ScriptTarget.Latest, true);
const literal = node => {
  if (ts.isStringLiteral(node)) return node.text;
  if (ts.isBinaryExpression(node) && node.operatorToken.kind === ts.SyntaxKind.PlusToken) return literal(node.left) + literal(node.right);
  throw new Error('Unexpected plan contract expression');
};
let exitDescription;
function visit(node) { if (ts.isVariableDeclaration(node) && node.name.getText(planAst) === 'EXIT_DESCRIPTION') exitDescription = literal(node.initializer); ts.forEachChild(node, visit); }
visit(planAst);
if (!exitDescription) throw new Error('Missing upstream EXIT_DESCRIPTION');
schemas.push({ type: 'function', function: { name: 'exit_plan_mode', description: exitDescription, parameters: { type: 'object', properties: { plan: { type: 'string', description: 'The complete plan, as markdown, starting with a # heading that names it.' } }, required: ['plan'] } } });
const deliverySource = await readFile(join(root, 'packages/client/ui-deliverables/src/index.ts'), 'utf8');
const deliveryAst = ts.createSourceFile('deliverables.ts', deliverySource, ts.ScriptTarget.Latest, true);
let deliveryPrompt;
function visitDelivery(node) { if (ts.isVariableDeclaration(node) && node.name.getText(deliveryAst) === 'FILE_REFERENCE_PROMPT') deliveryPrompt = literal(node.initializer); ts.forEachChild(node, visitDelivery); }
visitDelivery(deliveryAst);
if (!deliveryPrompt) throw new Error('Missing upstream FILE_REFERENCE_PROMPT');
sections.push({ name: 'ui:deliverable-file-references', order: orders.DELIVERABLE_FILE_REFERENCES, text: deliveryPrompt });
const prompt = sections.map(s => typeof s.text === 'function' ? s.text({}) : s.text).filter(Boolean).join('\n\n');
await writeFile(join(output, 'tools.json'), JSON.stringify(schemas, null, 2) + '\n');
await writeFile(join(output, 'tools.md'), prompt + '\n');
await writeFile(join(output, 'sections.json'), JSON.stringify(sections.map(s => ({ name: s.name, order: s.order, text: typeof s.text === 'function' ? s.text({}) : s.text })).filter(s => s.text), null, 2) + '\n');
await writeFile(join(output, 'identity.md'), identity + '\n');
await writeFile(join(output, 'SOURCE.json'), JSON.stringify({ repository: 'https://github.com/deepseek-ai/deepseek-harness', commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), tools: schemas.map(t => t.function.name) }, null, 2) + '\n');
await copyFile(join(root, 'LICENSE'), join(output, 'LICENSE'));
console.log(schemas.map(s => s.function.name).join(', '));
