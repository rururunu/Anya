function cssVar(name: string, fallback: string): string {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

/** Mermaid `theme: base` variables aligned with peek theme tokens. */
export function buildMermaidThemeVariables(isDark: boolean) {
  const text = cssVar("--peek-text", isDark ? "#e8e8e8" : "#242424");
  const muted = cssVar("--peek-muted", isDark ? "#9a9a9a" : "#5a5a5a");
  const faint = cssVar("--peek-faint", isDark ? "#6e6e6e" : "#767676");
  const border = cssVar(
    "--peek-code-border",
    isDark ? "rgba(255,255,255,0.08)" : "rgba(27,31,36,0.12)",
  );
  const surface = cssVar("--peek-code-body-bg", isDark ? "#121212" : "#f6f8fa");
  const card = cssVar("--peek-code-bg", isDark ? "#1a1a1a" : "#ffffff");
  const accent = cssVar("--peek-chart-1", isDark ? "#6ea8ff" : "#4f8ef7");
  const accentSoft = cssVar("--peek-chart-2", isDark ? "#4fd6a0" : "#34c98f");

  return {
    darkMode: isDark,
    background: "transparent",
    fontFamily: 'var(--font-sans, "Noto Sans SC", system-ui, sans-serif)',
    fontSize: "12px",
    primaryColor: card,
    primaryTextColor: text,
    primaryBorderColor: border,
    secondaryColor: surface,
    secondaryTextColor: text,
    secondaryBorderColor: border,
    tertiaryColor: surface,
    tertiaryTextColor: muted,
    tertiaryBorderColor: border,
    lineColor: faint,
    textColor: text,
    mainBkg: card,
    nodeBorder: border,
    nodeTextColor: text,
    clusterBkg: surface,
    clusterBorder: border,
    defaultLinkColor: faint,
    titleColor: text,
    edgeLabelBackground: card,
    actorBorder: border,
    actorBkg: card,
    actorTextColor: text,
    signalColor: text,
    signalTextColor: text,
    labelBoxBkgColor: surface,
    labelBoxBorderColor: border,
    labelTextColor: text,
    loopTextColor: text,
    noteBorderColor: border,
    noteBkgColor: surface,
    noteTextColor: text,
    activationBorderColor: accent,
    activationBkgColor: isDark ? "#2a2a2a" : "#e8e8e8",
    sequenceNumberColor: muted,
    sectionBkgColor: surface,
    altSectionBkgColor: card,
    gridColor: border,
    taskBkgColor: accentSoft,
    taskTextColor: text,
    taskTextLightColor: muted,
    taskTextOutsideColor: text,
    taskTextClickableColor: accent,
    activeTaskBkgColor: accent,
    activeTaskBorderColor: accent,
    doneTaskBkgColor: surface,
    doneTaskBorderColor: border,
    critBkgColor: cssVar("--peek-danger", "#c42b1c"),
    critBorderColor: cssVar("--peek-danger", "#c42b1c"),
    todayLineColor: accent,
    pie1: cssVar("--peek-chart-1", "#4f8ef7"),
    pie2: cssVar("--peek-chart-2", "#34c98f"),
    pie3: cssVar("--peek-chart-3", "#f7a44f"),
    pie4: cssVar("--peek-chart-4", "#e05c7a"),
    pie5: cssVar("--peek-chart-5", "#9b6df2"),
    pie6: cssVar("--peek-chart-6", "#3ec6d9"),
    pie7: cssVar("--peek-chart-7", "#d9c14f"),
    pieTitleTextColor: text,
    pieSectionTextColor: text,
    pieLegendTextColor: muted,
    pieStrokeColor: card,
    pieOuterStrokeColor: border,
  };
}
