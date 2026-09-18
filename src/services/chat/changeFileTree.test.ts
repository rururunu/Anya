import { describe, expect, it } from "vitest";
import { buildChangeFileTree, flattenChangeFileTree, visiblePathSegments } from "./changeFileTree";

describe("visiblePathSegments", () => {
  it("keeps the last shared folder when files diverge", () => {
    expect(
      visiblePathSegments(["C:/proj/src/components/chat/A.vue", "C:/proj/src/layouts/Main.vue"]),
    ).toEqual([
      ["src", "components", "chat", "A.vue"],
      ["src", "layouts", "Main.vue"],
    ]);
  });
});

describe("buildChangeFileTree", () => {
  it("groups files under shared folders", () => {
    const tree = buildChangeFileTree([
      { id: "2", path: "src/layouts/Main.vue" },
      { id: "1", path: "src/components/chat/A.vue" },
      { id: "3", path: "src/components/chat/B.vue" },
    ]);
    expect(tree).toMatchObject([
      {
        type: "dir",
        name: "src",
        children: [
          {
            type: "dir",
            name: "components",
            children: [
              {
                type: "dir",
                name: "chat",
                children: [
                  { type: "file", name: "A.vue", changeId: "1" },
                  { type: "file", name: "B.vue", changeId: "3" },
                ],
              },
            ],
          },
          {
            type: "dir",
            name: "layouts",
            children: [{ type: "file", name: "Main.vue", changeId: "2" }],
          },
        ],
      },
    ]);
  });
});

describe("flattenChangeFileTree", () => {
  it("hides children of collapsed folders", () => {
    const tree = buildChangeFileTree([
      { id: "1", path: "src/a.ts" },
      { id: "2", path: "src/b.ts" },
    ]);
    const rows = flattenChangeFileTree(tree, new Set(["src"]));
    expect(rows).toEqual([{ type: "dir", name: "src", path: "src", depth: 0, expanded: false }]);
  });
});
