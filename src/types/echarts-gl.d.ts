/**
 * echarts-gl v2.x ships without bundled type declarations. These ambient
 * declarations cover only the subpath exports used by ChartCard (see
 * node_modules/echarts-gl/charts.js and components.js).
 */
declare module "echarts-gl/charts" {
  import type { EChartsExtensionInstallRegisters } from "echarts/core";

  export function install(registers: EChartsExtensionInstallRegisters): void;
  export const Bar3DChart: typeof install;
  export const Line3DChart: typeof install;
  export const Scatter3DChart: typeof install;
  export const SurfaceChart: typeof install;
}

declare module "echarts-gl/components" {
  import type { EChartsExtensionInstallRegisters } from "echarts/core";

  export function install(registers: EChartsExtensionInstallRegisters): void;
  export const Grid3DComponent: typeof install;
}
