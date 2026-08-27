// @vitest-environment node
import { describe, expect, it } from "vitest";
import { JSDOM } from "jsdom";
import {
  FIND_CURRENT_CLASS,
  FIND_HIT_CLASS,
  activityMatchesQuery,
  applyFindHits,
  clearFindHits,
  jsonIncludesQuery,
  paintCurrentFindHit,
  textIncludesQuery,
} from "./conversationFind";

function rootWith(html: string) {
  const dom = new JSDOM(`<!doctype html><html><body>${html}</body></html>`);
  return dom.window.document.body.firstElementChild as HTMLElement;
}

describe("conversationFind", () => {
  it("wraps case-insensitive matches in document order", () => {
    const root = rootWith("<p>Foo bar foo</p>");
    const hits = applyFindHits(root, "foo");
    expect(hits).toHaveLength(2);
    expect(hits[0]?.textContent).toBe("Foo");
    expect(hits[1]?.textContent).toBe("foo");
    expect(hits.every((hit) => hit.classList.contains(FIND_HIT_CLASS))).toBe(true);
  });

  it("paints the current hit and can clear marks", () => {
    const root = rootWith("<p>alpha alpha</p>");
    const hits = applyFindHits(root, "alpha");
    paintCurrentFindHit(hits, 1);
    expect(hits[0]?.classList.contains(FIND_CURRENT_CLASS)).toBe(false);
    expect(hits[1]?.classList.contains(FIND_CURRENT_CLASS)).toBe(true);
    clearFindHits(root);
    expect(root.querySelectorAll("mark")).toHaveLength(0);
    expect(root.textContent).toBe("alpha alpha");
  });

  it("skips empty queries and message action chrome", () => {
    const root = rootWith('<p>hello</p><button class="message-actions">hello</button>');
    expect(applyFindHits(root, "   ")).toHaveLength(0);
    const hits = applyFindHits(root, "hello");
    expect(hits).toHaveLength(1);
    expect(hits[0]?.closest(".message-actions")).toBeNull();
  });

  it("matches activity text that is hidden when the card is collapsed", () => {
    expect(
      textIncludesQuery("src/main/java/com/demo/VirtualThreadsDemo.java", "VirtualThreads"),
    ).toBe(true);
    expect(jsonIncludesQuery({ command: "mvn test" }, "mvn")).toBe(true);
    expect(
      activityMatchesQuery(
        {
          id: "1",
          toolName: "exec",
          title: "Run",
          kind: "shell",
          detail: "hidden output TOKEN",
          success: true,
          status: "done",
        },
        "token",
      ),
    ).toBe(true);
  });
});
