import { h, render, type Component, type VNode } from "vue";
import { readMermaidSource } from "@/services/chat/mermaidDiagram";

const mountedContainers = new Map<HTMLElement, VNode>();

let mermaidCardComponent: Component | null = null;
let mermaidCardLoading: Promise<Component> | null = null;

async function loadMermaidCard(): Promise<Component> {
  if (mermaidCardComponent) return mermaidCardComponent;
  if (!mermaidCardLoading) {
    mermaidCardLoading = import("./MermaidCard.vue").then((mod) => {
      mermaidCardComponent = mod.default;
      return mod.default;
    });
  }
  return mermaidCardLoading;
}

/**
 * Hydrates `[data-mermaid-block]` placeholders emitted by the markdown renderer.
 */
export function hydrateMermaidBlocks(root: HTMLElement) {
  for (const node of mountedContainers.keys()) {
    if (!node.isConnected) {
      render(null, node);
      mountedContainers.delete(node);
    }
  }

  const nodes = [...root.querySelectorAll<HTMLElement>("[data-mermaid-block], .mermaid-block")];
  if (nodes.length === 0) return;

  void loadMermaidCard()
    .then((MermaidCard) => {
      for (const node of nodes) {
        if (!node.isConnected) continue;
        const source = readMermaidSource(node);
        if (!source.trim()) continue;
        const vnode = h(MermaidCard, { source });
        mountedContainers.set(node, vnode);
        render(vnode, node);
      }
    })
    .catch((error) => {
      console.error("mermaid card load failed:", error);
    });
}

export function disposeMermaidBlocks(root: HTMLElement) {
  for (const node of [...mountedContainers.keys()]) {
    if (root.contains(node) || !node.isConnected) {
      render(null, node);
      mountedContainers.delete(node);
    }
  }
}
