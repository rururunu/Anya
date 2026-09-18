export type ChangeFileTreeNode =
  | { type: "dir"; name: string; path: string; children: ChangeFileTreeNode[] }
  | { type: "file"; name: string; path: string; changeId: string };

export type ChangeFileTreeRow =
  | { type: "dir"; name: string; path: string; depth: number; expanded: boolean }
  | { type: "file"; name: string; path: string; changeId: string; depth: number };

function normalizePath(path: string) {
  return path.replace(/\\/g, "/").replace(/\/+$/, "");
}

function splitPath(path: string) {
  return normalizePath(path)
    .split("/")
    .filter((part) => part.length > 0 && part !== ".");
}

/** Drop unique leading folders, keep the last shared directory. */
export function visiblePathSegments(paths: string[]): string[][] {
  const parts = paths.map(splitPath);
  if (!parts.length) return parts;
  if (parts.length === 1) {
    const only = parts[0]!;
    return [only.slice(Math.max(0, only.length - 3))];
  }
  let depth = 0;
  while (
    parts.every((item) => item.length > depth + 1) &&
    parts.every((item) => item[depth] === parts[0]![depth]) &&
    parts.every((item) => item.length > depth + 2 && item[depth + 1] === parts[0]![depth + 1])
  ) {
    depth += 1;
  }
  return parts.map((item) => item.slice(depth));
}

function insert(nodes: ChangeFileTreeNode[], parts: string[], changeId: string, prefix: string) {
  const name = parts[0];
  if (!name) return;
  const path = prefix ? `${prefix}/${name}` : name;
  if (parts.length === 1) {
    nodes.push({ type: "file", name, path, changeId });
    return;
  }
  let dir = nodes.find((node): node is Extract<ChangeFileTreeNode, { type: "dir" }> => {
    return node.type === "dir" && node.name === name;
  });
  if (!dir) {
    dir = { type: "dir", name, path, children: [] };
    nodes.push(dir);
  }
  insert(dir.children, parts.slice(1), changeId, path);
}

function sortTree(nodes: ChangeFileTreeNode[]) {
  nodes.sort((left, right) => {
    if (left.type !== right.type) return left.type === "dir" ? -1 : 1;
    return left.name.localeCompare(right.name);
  });
  for (const node of nodes) {
    if (node.type === "dir") sortTree(node.children);
  }
}

/** Folder tree of changed files, grouped like a review explorer. */
export function buildChangeFileTree(
  files: Array<{ id: string; path: string }>,
): ChangeFileTreeNode[] {
  const segments = visiblePathSegments(files.map((file) => file.path));
  const root: ChangeFileTreeNode[] = [];
  files.forEach((file, index) => {
    const parts = segments[index];
    if (!parts?.length) return;
    insert(root, parts, file.id, "");
  });
  sortTree(root);
  return root;
}

export function flattenChangeFileTree(
  nodes: ChangeFileTreeNode[],
  collapsed: Set<string>,
  depth = 0,
): ChangeFileTreeRow[] {
  const rows: ChangeFileTreeRow[] = [];
  for (const node of nodes) {
    if (node.type === "dir") {
      const expanded = !collapsed.has(node.path);
      rows.push({ type: "dir", name: node.name, path: node.path, depth, expanded });
      if (expanded) rows.push(...flattenChangeFileTree(node.children, collapsed, depth + 1));
      continue;
    }
    rows.push({
      type: "file",
      name: node.name,
      path: node.path,
      changeId: node.changeId,
      depth,
    });
  }
  return rows;
}
