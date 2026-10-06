import type { TokenUsageReport } from "@/types/tokenUsage";

export function formatProfileTokens(value: number, language = "zh-CN"): string {
  const tokens = Number.isFinite(value) ? Math.max(0, Math.floor(value)) : 0;
  if (language !== "zh-CN") {
    return new Intl.NumberFormat(language, {
      notation: "compact",
      maximumFractionDigits: 2,
    }).format(tokens);
  }
  const units = [
    { scale: 1000, label: "千" },
    { scale: 10000, label: "万" },
    { scale: 100000000, label: "亿" },
  ];
  let index = -1;
  for (let candidate = 0; candidate < units.length; candidate += 1) {
    if (tokens >= units[candidate].scale) index = candidate;
  }
  if (index < 0) return String(tokens);
  if (
    index < units.length - 1 &&
    Math.round((tokens / units[index].scale) * 100) / 100 >=
      units[index + 1].scale / units[index].scale
  )
    index += 1;
  return `${new Intl.NumberFormat(language, { maximumFractionDigits: 2 }).format(tokens / units[index].scale)}${units[index].label}`;
}

export function localDayKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function profileStats(report: TokenUsageReport, today = new Date()) {
  const daily = new Map<string, number>();
  for (const bucket of report.timeline) {
    const date = new Date(bucket.bucket);
    if (!Number.isFinite(date.getTime())) continue;
    const key = localDayKey(date);
    daily.set(key, (daily.get(key) ?? 0) + Math.max(0, bucket.totalTokens));
  }
  const days = [...daily.keys()].sort();
  const dayNumber = (key: string) => {
    const [year, month, day] = key.split("-").map(Number);
    return Date.UTC(year, month - 1, day) / 86400000;
  };
  let longest = 0;
  let run = 0;
  let previous = -Infinity;
  for (const key of days) {
    const day = dayNumber(key);
    run = day === previous + 1 ? run + 1 : 1;
    longest = Math.max(longest, run);
    previous = day;
  }
  let cursor = dayNumber(localDayKey(today));
  const active = new Set(days.map(dayNumber));
  if (!active.has(cursor)) cursor -= 1;
  let current = 0;
  while (active.has(cursor)) {
    current += 1;
    cursor -= 1;
  }
  return { daily, longest, current, peak: Math.max(0, ...daily.values()) };
}

export function activityDays(today = new Date()) {
  const start = new Date(today.getFullYear(), today.getMonth(), today.getDate() - 364);
  start.setDate(start.getDate() - start.getDay());
  const end = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  end.setDate(end.getDate() + 6 - end.getDay());
  const days: { key: string; date: Date; future: boolean }[] = [];
  for (const date = new Date(start); date <= end; date.setDate(date.getDate() + 1)) {
    days.push({ key: localDayKey(date), date: new Date(date), future: date > today });
  }
  return days;
}
