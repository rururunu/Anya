import { h, render, type Component, type VNode } from "vue";
import { parseChartSpec } from "@/services/chat/chartSpec";

const mountedContainers = new Map<HTMLElement, VNode>();

let chartCardComponent: Component | null = null;
let chartCardLoading: Promise<Component> | null = null;

async function loadChartCard(): Promise<Component> {
  if (chartCardComponent) return chartCardComponent;
  if (!chartCardLoading) {
    // Keep ECharts out of the overlay input-mode boot graph; only load when a
    // chart fence actually needs hydration.
    chartCardLoading = import("./ChartCard.vue").then((mod) => {
      chartCardComponent = mod.default;
      return mod.default;
    });
  }
  return chartCardLoading;
}

/**
 * Hydrates `[data-chart-spec]` placeholders emitted by the markdown renderer.
 *
 * Containers are managed via a module-level registry: placeholders replaced by
 * a v-html update (no longer connected) are unmounted first so their ECharts
 * instance is disposed, then every live placeholder gets a ChartCard mounted.
 * Rendering into an already-mounted container patches the existing component,
 * so the same instance is reused and updated via setOption.
 */
export function hydrateChartBlocks(root: HTMLElement) {
  for (const node of mountedContainers.keys()) {
    if (!node.isConnected) {
      render(null, node);
      mountedContainers.delete(node);
    }
  }
  const nodes = [...root.querySelectorAll<HTMLElement>("[data-chart-spec]")];
  if (nodes.length === 0) return;

  void loadChartCard()
    .then((ChartCard) => {
      for (const node of nodes) {
        if (!node.isConnected) continue;
        const raw = node.dataset.chartSpec;
        const spec = raw ? parseChartSpec(raw) : null;
        if (!spec) continue;
        const vnode = h(ChartCard, { spec });
        mountedContainers.set(node, vnode);
        render(vnode, node);
      }
    })
    .catch((error) => {
      console.error("chart card load failed:", error);
    });
}
