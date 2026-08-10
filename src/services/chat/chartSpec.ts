export interface ChartSeries {
  name?: string;
  /**
   * Flat values for bar/line/radar/parallel rows; point tuples for
   * scatter/heatmap/candlestick and the 3D types (see per-type validators).
   */
  data: number[] | number[][];
}

export interface ChartSlice {
  name: string;
  value: number;
}

export interface ChartIndicator {
  name: string;
  max: number;
}

export interface ChartNode {
  name: string;
  value?: number;
}

export interface ChartLink {
  source: string;
  target: string;
  value?: number;
}

export interface ChartTreeNode {
  name: string;
  value?: number;
  children?: ChartTreeNode[];
}

export type ChartType =
  | "bar"
  | "line"
  | "scatter"
  | "pie"
  | "funnel"
  | "gauge"
  | "radar"
  | "heatmap"
  | "candlestick"
  | "treemap"
  | "sankey"
  | "graph"
  | "parallel"
  | "bar3d"
  | "scatter3d"
  | "surface"
  | "line3d"
  | "custom";

export interface ChartSpec {
  type: ChartType;
  title?: string;
  unit?: string;
  /** Category labels (bar/line/heatmap/candlestick/bar3d). */
  x?: string[];
  /** Second category axis (heatmap/bar3d). */
  y?: string[];
  series?: ChartSeries[];
  /** Slices for pie/funnel/gauge. */
  items?: ChartSlice[];
  /** Radar axes. */
  indicators?: ChartIndicator[];
  /** Entities for sankey/graph. */
  nodes?: ChartNode[];
  /** Relations for sankey/graph. */
  links?: ChartLink[];
  /** Column names for parallel. */
  dimensions?: string[];
  /** Nested tree for treemap. */
  data?: ChartTreeNode[];
  /** Gauge bounds (default 0–100). */
  min?: number;
  max?: number;
  /** Full ECharts option passthrough for `custom`. */
  option?: Record<string, unknown>;
}

const CHART_TYPES = new Set<ChartType>([
  "bar",
  "line",
  "scatter",
  "pie",
  "funnel",
  "gauge",
  "radar",
  "heatmap",
  "candlestick",
  "treemap",
  "sankey",
  "graph",
  "parallel",
  "bar3d",
  "scatter3d",
  "surface",
  "line3d",
  "custom",
]);

/**
 * Series `type` values that a `custom` passthrough option may declare. These
 * are exactly the classes registered by ChartCard (core + lazy extra /
 * echarts-gl loaders) plus the ECharts built-ins used for passthrough
 * else is rejected at parse time: handing an unregistered series type to
 * `setOption` leaves the series model undefined in the scheduler and can
 * crash with `Cannot read properties of undefined (reading 'getProgressive')`.
 *
 * Names must match ECharts' own series type identifiers (note the capital D
 * in the 3D types). Lowercase 3D variants are normalized to the canonical
 * names.
 */
export const CUSTOM_SERIES_TYPES = new Set<string>([
  "bar",
  "line",
  "scatter",
  "pie",
  "funnel",
  "gauge",
  "radar",
  "heatmap",
  "candlestick",
  "treemap",
  "sankey",
  "graph",
  "parallel",
  "bar3D",
  "scatter3D",
  "surface",
  "line3D",
  "custom",
  "sunburst",
  "map",
  "boxplot",
]);

const CUSTOM_TYPE_CANON: Record<string, string> = {
  bar3d: "bar3D",
  scatter3d: "scatter3D",
  line3d: "line3D",
};

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function optionalText(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

/** Non-empty array of non-blank strings. */
function parseStringArray(value: unknown): string[] | null {
  if (!Array.isArray(value) || value.length === 0) return null;
  const entries: string[] = [];
  for (const entry of value) {
    if (typeof entry !== "string" || entry.trim() === "") return null;
    entries.push(entry);
  }
  return entries;
}

/** Non-empty array of finite numbers. */
function parseNumbers(value: unknown): number[] | null {
  if (!Array.isArray(value) || value.length === 0) return null;
  const entries: number[] = [];
  for (const entry of value) {
    if (!isFiniteNumber(entry)) return null;
    entries.push(entry);
  }
  return entries;
}

/** A point tuple of exactly `dims` finite numbers. */
function parsePoint(value: unknown, dims: number): number[] | null {
  if (!Array.isArray(value) || value.length !== dims) return null;
  for (const entry of value) {
    if (!isFiniteNumber(entry)) return null;
  }
  return value as number[];
}

/** Non-empty array of point tuples. */
function parsePoints(value: unknown, dims: number): number[][] | null {
  if (!Array.isArray(value) || value.length === 0) return null;
  const points: number[][] = [];
  for (const entry of value) {
    const point = parsePoint(entry, dims);
    if (!point) return null;
    points.push(point);
  }
  return points;
}

/** Grid cells `[xi, yi, value]` with category indices within bounds. */
function parseCells(value: unknown, xCount: number, yCount: number): number[][] | null {
  const cells = parsePoints(value, 3);
  if (!cells) return null;
  for (const [xi, yi] of cells) {
    if (
      !Number.isInteger(xi) ||
      !Number.isInteger(yi) ||
      xi < 0 ||
      xi >= xCount ||
      yi < 0 ||
      yi >= yCount
    ) {
      return null;
    }
  }
  return cells;
}

function parseSlices(value: unknown): ChartSlice[] | null {
  if (!Array.isArray(value) || value.length === 0) return null;
  const items: ChartSlice[] = [];
  for (const entry of value) {
    if (typeof entry !== "object" || entry === null || Array.isArray(entry)) return null;
    const { name, value: sliceValue } = entry as Record<string, unknown>;
    if (typeof name !== "string" || !isFiniteNumber(sliceValue)) return null;
    items.push({ name, value: sliceValue });
  }
  return items;
}

function parseIndicators(value: unknown): ChartIndicator[] | null {
  if (!Array.isArray(value) || value.length === 0) return null;
  const indicators: ChartIndicator[] = [];
  for (const entry of value) {
    if (typeof entry !== "object" || entry === null || Array.isArray(entry)) return null;
    const { name, max } = entry as Record<string, unknown>;
    if (typeof name !== "string" || !isFiniteNumber(max)) return null;
    indicators.push({ name, max });
  }
  return indicators;
}

function parseNodes(value: unknown): ChartNode[] | null {
  if (!Array.isArray(value) || value.length === 0) return null;
  const nodes: ChartNode[] = [];
  for (const entry of value) {
    if (typeof entry !== "object" || entry === null || Array.isArray(entry)) return null;
    const { name, value: nodeValue } = entry as Record<string, unknown>;
    if (typeof name !== "string") return null;
    const node: ChartNode = { name };
    if (nodeValue !== undefined) {
      if (!isFiniteNumber(nodeValue)) return null;
      node.value = nodeValue;
    }
    nodes.push(node);
  }
  return nodes;
}

function parseLinks(value: unknown): ChartLink[] | null {
  if (!Array.isArray(value) || value.length === 0) return null;
  const links: ChartLink[] = [];
  for (const entry of value) {
    if (typeof entry !== "object" || entry === null || Array.isArray(entry)) return null;
    const { source, target, value: linkValue } = entry as Record<string, unknown>;
    if (typeof source !== "string" || typeof target !== "string") return null;
    const link: ChartLink = { source, target };
    if (linkValue !== undefined) {
      if (!isFiniteNumber(linkValue)) return null;
      link.value = linkValue;
    }
    links.push(link);
  }
  return links;
}

function parseTree(value: unknown): ChartTreeNode[] | null {
  if (!Array.isArray(value) || value.length === 0) return null;
  const nodes: ChartTreeNode[] = [];
  for (const entry of value) {
    if (typeof entry !== "object" || entry === null || Array.isArray(entry)) return null;
    const { name, value: nodeValue, children } = entry as Record<string, unknown>;
    if (typeof name !== "string") return null;
    const node: ChartTreeNode = { name };
    if (nodeValue !== undefined) {
      if (!isFiniteNumber(nodeValue)) return null;
      node.value = nodeValue;
    }
    if (children !== undefined) {
      const kids = parseTree(children);
      if (!kids) return null;
      node.children = kids;
    }
    nodes.push(node);
  }
  return nodes;
}

/**
 * Series array with a per-type `data` validator. Names are optional and
 * trimmed; blank names are dropped.
 */
function parseSeries(
  value: unknown,
  dataParser: (data: unknown) => number[] | number[][] | null,
): ChartSeries[] | null {
  if (!Array.isArray(value) || value.length === 0) return null;
  const series: ChartSeries[] = [];
  for (const entry of value) {
    if (typeof entry !== "object" || entry === null || Array.isArray(entry)) return null;
    const { name, data } = entry as Record<string, unknown>;
    const parsed = dataParser(data);
    if (!parsed) return null;
    const seriesName = optionalText(name);
    series.push(seriesName ? { name: seriesName, data: parsed } : { data: parsed });
  }
  return series;
}

/**
 * Parses and validates a `chart` fence spec. Returns null when the source is
 * not valid JSON or does not match the supported shapes, so callers can fall
 * back to rendering the raw code block.
 */
export function parseChartSpec(source: string): ChartSpec | null {
  let raw: unknown;
  try {
    raw = JSON.parse(source);
  } catch {
    return null;
  }
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) return null;
  const candidate = raw as Record<string, unknown>;

  if (typeof candidate.type !== "string" || !CHART_TYPES.has(candidate.type as ChartType)) {
    return null;
  }
  const type = candidate.type as ChartType;
  const spec: ChartSpec = { type };
  const title = optionalText(candidate.title);
  if (title) spec.title = title;
  const unit = optionalText(candidate.unit);
  if (unit) spec.unit = unit;

  switch (type) {
    case "bar":
    case "line": {
      const x = parseStringArray(candidate.x);
      if (!x) return null;
      spec.x = x;
      const series = parseSeries(candidate.series, parseNumbers);
      if (!series) return null;
      spec.series = series;
      return spec;
    }

    case "scatter": {
      const series = parseSeries(candidate.series, (data) => parsePoints(data, 2));
      if (!series) return null;
      spec.series = series;
      return spec;
    }

    case "pie":
    case "funnel": {
      const items = parseSlices(candidate.items);
      if (!items) return null;
      spec.items = items;
      return spec;
    }

    case "gauge": {
      const items = parseSlices(candidate.items);
      if (!items) return null;
      spec.items = items;
      if (candidate.min !== undefined) {
        if (!isFiniteNumber(candidate.min)) return null;
        spec.min = candidate.min;
      }
      if (candidate.max !== undefined) {
        if (!isFiniteNumber(candidate.max)) return null;
        spec.max = candidate.max;
      }
      if (spec.min !== undefined && spec.max !== undefined && spec.min >= spec.max) return null;
      return spec;
    }

    case "radar": {
      const indicators = parseIndicators(candidate.indicators);
      if (!indicators) return null;
      spec.indicators = indicators;
      const series = parseSeries(candidate.series, (data) => {
        const values = parseNumbers(data);
        return values && values.length === indicators.length ? values : null;
      });
      if (!series) return null;
      spec.series = series;
      return spec;
    }

    case "heatmap": {
      const x = parseStringArray(candidate.x);
      const y = parseStringArray(candidate.y);
      if (!x || !y) return null;
      spec.x = x;
      spec.y = y;
      const series = parseSeries(candidate.series, (data) => parseCells(data, x.length, y.length));
      if (!series) return null;
      spec.series = series;
      return spec;
    }

    case "candlestick": {
      const x = parseStringArray(candidate.x);
      if (!x) return null;
      spec.x = x;
      const series = parseSeries(candidate.series, (data) => parsePoints(data, 4));
      if (!series) return null;
      spec.series = series;
      return spec;
    }

    case "treemap": {
      const data = parseTree(candidate.data);
      if (!data) return null;
      spec.data = data;
      return spec;
    }

    case "sankey":
    case "graph": {
      const nodes = parseNodes(candidate.nodes);
      const links = parseLinks(candidate.links);
      if (!nodes || !links) return null;
      spec.nodes = nodes;
      spec.links = links;
      return spec;
    }

    case "parallel": {
      const dimensions = parseStringArray(candidate.dimensions);
      if (!dimensions) return null;
      spec.dimensions = dimensions;
      const series = parseSeries(candidate.series, (data) => {
        if (!Array.isArray(data) || data.length === 0) return null;
        const rows: number[][] = [];
        for (const row of data) {
          const parsed = parsePoint(row, dimensions.length);
          if (!parsed) return null;
          rows.push(parsed);
        }
        return rows;
      });
      if (!series) return null;
      spec.series = series;
      return spec;
    }

    case "bar3d": {
      const x = parseStringArray(candidate.x);
      const y = parseStringArray(candidate.y);
      if (!x || !y) return null;
      spec.x = x;
      spec.y = y;
      const series = parseSeries(candidate.series, (data) => parseCells(data, x.length, y.length));
      if (!series) return null;
      spec.series = series;
      return spec;
    }

    case "scatter3d":
    case "surface":
    case "line3d": {
      const series = parseSeries(candidate.series, (data) => parsePoints(data, 3));
      if (!series) return null;
      spec.series = series;
      return spec;
    }

    case "custom": {
      const option = candidate.option;
      if (typeof option !== "object" || option === null || Array.isArray(option)) return null;
      // The passthrough option is handed to echarts.setOption as-is, so every
      // series entry must declare a type we have actually registered. An
      // unregistered or missing type would leave an undefined series model in
      // echarts 6's scheduler and crash at `getProgressive()`.
      const series = (option as Record<string, unknown>).series;
      if (series !== undefined) {
        if (!Array.isArray(series)) return null;
        for (const entry of series) {
          if (typeof entry !== "object" || entry === null || Array.isArray(entry)) return null;
          const type = (entry as Record<string, unknown>).type;
          const canonical = typeof type === "string" ? CUSTOM_TYPE_CANON[type] : undefined;
          if (
            typeof type !== "string" ||
            (!CUSTOM_SERIES_TYPES.has(type) && canonical === undefined)
          )
            return null;
          if (canonical) (entry as Record<string, unknown>).type = canonical;
        }
      }
      spec.option = option as Record<string, unknown>;
      return spec;
    }
  }
}
