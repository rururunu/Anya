import test from 'node:test';
import assert from 'node:assert/strict';
import { summarize } from './deepseek-metrics-report.mjs';

test('quality labels are distinct from transport success and unknown cache stays unknown', () => {
  const report = summarize([
    { kind: 'model_call_start', call_id: 'a', request_id: 'r', model: 'deepseek' },
    { kind: 'reasoning_policy', call_id: 'a', task_class: 'simple_question', selected_effort: 'low' },
    { kind: 'wire_attempt', call_id: 'a', first_sse_ms: 4 },
    { kind: 'model_call', call_id: 'a', request_id: 'r', model: 'deepseek', succeeded: true, duration_ms: 10, cache_hit_ratio: null },
  ], [{ request_id: 'r', success: false }]);
  assert.equal(report.groups[0].transport_success_rate, 1);
  assert.equal(report.groups[0].task_success_rate, 0);
  assert.equal(report.groups[0].mean_cache_hit_ratio, null);
  assert.equal(report.groups[0].median_first_sse_ms, 4);
});

test('a multi-call task is judged once per effort, explicit zero cache is retained', () => {
  const records = ['a', 'b'].flatMap(call_id => [
    { kind: 'model_call_start', call_id, request_id: 'r', model: 'deepseek' },
    { kind: 'reasoning_policy', call_id, task_class: 'configured_task', selected_effort: 'high' },
    { kind: 'model_call', call_id, request_id: 'r', model: 'deepseek', succeeded: true, cache_hit_ratio: 0, retry_count: 1 },
  ]);
  const group = summarize(records, [{ request_id: 'r', success: true }]).groups[0];
  assert.equal(group.calls, 2);
  assert.equal(group.judged_tasks, 1);
  assert.equal(group.mean_cache_hit_ratio, 0);
  assert.equal(group.retries, 2);
  assert.throws(() => summarize(records, [{ request_id: 'r', success: 'yes' }]));
});
