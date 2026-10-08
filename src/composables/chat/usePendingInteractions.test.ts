import { beforeEach, describe, expect, it, vi } from "vitest";
import { usePendingInteractions } from "./usePendingInteractions";
import type { PendingInteractionsSnapshot, ToolApprovalEvent } from "@/types/chat";
const mocks = vi.hoisted(() => ({ snapshot: vi.fn() }));
vi.mock("@/services/ipc", () => ({ getPendingInteractions: mocks.snapshot }));
const tool = (id: string, sequence = 1): ToolApprovalEvent => ({
  requestId: id,
  sessionId: "s",
  toolName: "pwsh",
  title: id,
  sequence,
});
const snapshot = (...tools: ToolApprovalEvent[]): PendingInteractionsSnapshot => ({
  askUser: [],
  pathPermission: [],
  toolApproval: tools,
});

describe("pending interactions", () => {
  beforeEach(() => mocks.snapshot.mockReset());
  it("does not let an older session snapshot undo a newer global recovery", async () => {
    let finish!: (value: PendingInteractionsSnapshot) => void;
    mocks.snapshot.mockReturnValueOnce(
      new Promise<PendingInteractionsSnapshot>((resolve) => {
        finish = resolve;
      }),
    );
    mocks.snapshot.mockResolvedValueOnce(snapshot(tool("fresh")));
    const queue = usePendingInteractions();
    queue.enqueue("s", { kind: "tool_approval", value: tool("old") });
    const earlier = queue.sync("s");
    await queue.sync("");
    finish(snapshot(tool("old")));
    await earlier;
    queue.enqueue("s", { kind: "tool_approval", value: tool("old") });
    expect(queue.queues.value.s.map((item) => item.value.requestId)).toEqual(["fresh"]);
  });
  it("recovers requests for background sessions in one snapshot", async () => {
    mocks.snapshot.mockResolvedValue(
      snapshot(tool("active"), { ...tool("background"), sessionId: "other" }),
    );
    const queue = usePendingInteractions();
    await queue.sync("");
    expect(queue.pendingInteractions.value.s.value.requestId).toBe("active");
    expect(queue.pendingInteractions.value.other.value.requestId).toBe("background");
  });
  it("queues requests and promotes the next when another window resolves the head", () => {
    const queue = usePendingInteractions();
    queue.enqueue("s", { kind: "tool_approval", value: tool("first") });
    queue.enqueue("s", { kind: "tool_approval", value: tool("second") });
    queue.enqueue("s", { kind: "tool_approval", value: tool("second") });
    expect(queue.queues.value.s).toHaveLength(2);
    queue.resolve("first", "s");
    expect(queue.pendingInteractions.value.s.value.requestId).toBe("second");
    queue.enqueue("s", { kind: "tool_approval", value: tool("first") });
    expect(queue.queues.value.s).toHaveLength(1);
  });
  it("restores backend order across interaction kinds", async () => {
    mocks.snapshot.mockResolvedValue({
      ...snapshot(tool("tool", 3)),
      askUser: [{ sessionId: "s", requestId: "ask", questions: [], sequence: 1 }],
      pathPermission: [
        {
          sessionId: "s",
          requestId: "path",
          path: "outside",
          operation: "read",
          toolName: "read",
          sequence: 2,
        },
      ],
    });
    const queue = usePendingInteractions();
    await queue.sync("s");
    expect(queue.queues.value.s.map((item) => item.value.requestId)).toEqual([
      "ask",
      "path",
      "tool",
    ]);
  });
  it("keeps live arrivals and does not resurrect a resolved request from an in-flight snapshot", async () => {
    let finish!: (value: PendingInteractionsSnapshot) => void;
    mocks.snapshot.mockReturnValue(
      new Promise<PendingInteractionsSnapshot>((resolve) => {
        finish = resolve;
      }),
    );
    const queue = usePendingInteractions();
    queue.enqueue("s", { kind: "tool_approval", value: tool("old") });
    const sync = queue.sync("s");
    queue.resolve("old", "s");
    queue.enqueue("s", { kind: "tool_approval", value: tool("live") });
    finish(snapshot(tool("old")));
    await sync;
    expect(queue.queues.value.s.map((item) => item.value.requestId)).toEqual(["live"]);
  });
});
