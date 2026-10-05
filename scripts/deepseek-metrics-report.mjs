import { readFile, readdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const finite = n => typeof n === 'number' && Number.isFinite(n) && n >= 0;
const average = values => values.length ? values.reduce((a, b) => a + b, 0) / values.length : null;
const median = values => {
  const sorted = values.filter(finite).sort((a, b) => a - b);
  const n = sorted.length;
  return n ? (sorted[Math.floor(n / 2)] + sorted[Math.floor((n - 1) / 2)]) / 2 : null;
};

// HTTP success is separate from judged task success. Outcomes must be supplied
// by a real benchmark or reviewer; a finished stream is not evidence of quality.
export function summarize(records, outcomes = []) {
  const calls = new Map();
  for (const record of records) {
    if (!record.call_id) continue;
    const call = calls.get(record.call_id) ?? { attempts: [] };
    if (record.kind === 'model_call_start') call.start = record;
    if (record.kind === 'model_call') call.end = record;
    if (record.kind === 'reasoning_policy') call.policy = record;
    if (record.kind === 'wire_attempt') call.attempts.push(record);
    calls.set(record.call_id, call);
  }
  const judgments = new Map();
  for (const outcome of outcomes) {
    if (typeof outcome.success !== 'boolean' || (!outcome.call_id && !outcome.request_id)) {
      throw new Error('Each outcome requires call_id or request_id and a boolean success');
    }
    const key = outcome.call_id ? `call:${outcome.call_id}` : `request:${outcome.request_id}`;
    if (judgments.has(key)) throw new Error(`Duplicate outcome: ${key}`);
    judgments.set(key, outcome);
  }
  const groups = new Map();
  for (const call of calls.values()) {
    const record = call.end ?? call.start;
    if (!record) continue;
    const model = record.model ?? call.policy?.model ?? 'unknown';
    const taskClass = call.policy?.task_class ?? 'unknown';
    const effort = call.policy?.selected_effort ?? 'unknown';
    const mode = call.policy?.policy_mode ?? 'unknown';
    const key = JSON.stringify([model, taskClass, effort, mode]);
    const group = groups.get(key) ?? { model, task_class: taskClass, selected_effort: effort, policy_mode: mode, calls: [], judgments: new Map() };
    group.calls.push(call);
    const judgment = judgments.get(`call:${record.call_id}`) ?? judgments.get(`request:${record.request_id}`);
    if (judgment) group.judgments.set(judgment, judgment.success);
    groups.set(key, group);
  }
  return { groups: [...groups.values()].map(group => {
    const ends = group.calls.map(c => c.end).filter(Boolean);
    const observed = [...group.judgments.values()];
    const reportedCache = ends.filter(r => finite(r.cache_hit_ratio));
    const timings = group.calls.flatMap(c => c.attempts);
    return {
      model: group.model, task_class: group.task_class, selected_effort: group.selected_effort,
      policy_mode: group.policy_mode,
      calls: group.calls.length, ended_calls: ends.length,
      start_only_calls: group.calls.length - ends.length,
      transport_success_rate: average(ends.map(r => r.succeeded ? 1 : 0)),
      judged_tasks: observed.length, task_success_rate: average(observed.map(s => s ? 1 : 0)),
      cache_reported_calls: reportedCache.length,
      mean_cache_hit_ratio: average(reportedCache.map(r => r.cache_hit_ratio)),
      median_duration_ms: median(ends.map(r => r.duration_ms)),
      median_first_event_ms: median(ends.map(r => r.first_provider_event_ms)),
      median_first_sse_ms: median(timings.map(r => r.first_sse_ms)),
      median_first_content_ms: median(ends.map(r => r.first_content_ms)),
      median_reasoning_tokens: median(ends.map(r => r.reasoning_tokens)),
      median_completion_tokens: median(ends.map(r => r.completion_tokens)),
      retries: ends.filter(r => finite(r.retry_count)).reduce((sum, r) => sum + r.retry_count, 0),
    };
  }), quality_note: 'Compare the same judged task set across efforts. Missing quality labels and unreported API usage remain null. Transport success alone does not establish task success or savings.' };
}

async function jsonLines(path) {
  const text = await readFile(path, 'utf8');
  return text.split(/\r?\n/).filter(line => line.trim()).map((line, i) => {
    try { return JSON.parse(line); } catch { throw new Error(`${path}:${i + 1}: invalid JSON`); }
  });
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [input, outcomesPath] = process.argv.slice(2);
  if (!input) throw new Error('Usage: node scripts/deepseek-metrics-report.mjs <logs-directory> [outcomes.jsonl]');
  const names = (await readdir(input)).filter(name => /^deepseek-calls-.*\.jsonl$/.test(name)).sort();
  const records = (await Promise.all(names.map(name => jsonLines(resolve(input, name))))).flat();
  const outcomes = outcomesPath ? await jsonLines(outcomesPath) : [];
  process.stdout.write(`${JSON.stringify(summarize(records, outcomes), null, 2)}\n`);
}
