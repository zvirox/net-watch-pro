// DESIGN.md §3.3 / §10 — the ECharts theme is derived from CSS tokens at
// runtime. No color, font, radius or shadow is duplicated in TypeScript.
import { registerTheme } from 'echarts/core';

/** Fixed chart series order (DESIGN.md §3.3). Download is always series 1. */
export const SERIES_ORDER = [
  'download',
  'upload',
  'tertiary',
  'quaternary',
  'fifth',
  'muted',
] as const;

const THEME_NAME = 'netwatch-pro';

function token(name: string): string {
  return window.getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

/** Build an ECharts theme from the currently active token set. */
export function buildChartTheme(): Record<string, unknown> {
  const fontUi = token('--nwp-font-ui');
  const fontMono = token('--nwp-font-mono');
  const text = token('--nwp-text');
  const textSecondary = token('--nwp-text-secondary');
  const axisLine = token('--nwp-chart-axis-line');
  const axisLabel = token('--nwp-chart-axis-label');
  const grid = token('--nwp-chart-grid');
  const radius = token('--nwp-radius-md');
  const radiusPx = Number.parseInt(radius, 10);

  return {
    color: [
      token('--nwp-chart-1'),
      token('--nwp-chart-2'),
      token('--nwp-chart-3'),
      token('--nwp-chart-4'),
      token('--nwp-chart-5'),
      token('--nwp-chart-muted'),
    ],
    backgroundColor: 'transparent',
    // DESIGN.md §10: implicit animations must not shift layout.
    animation: false,
    textStyle: { fontFamily: fontUi, color: text },
    grid: {
      borderColor: axisLine,
      containLabel: true,
    },
    categoryAxis: {
      axisLine: { lineStyle: { color: axisLine } },
      axisTick: { show: false },
      axisLabel: { color: axisLabel, fontFamily: fontMono, fontSize: 11 },
      splitLine: { show: false },
    },
    valueAxis: {
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { color: axisLabel, fontFamily: fontMono, fontSize: 11 },
      splitLine: { show: true, lineStyle: { color: grid } },
    },
    line: {
      symbol: 'none',
      smooth: false,
      lineStyle: { width: 2 },
      areaStyle: { opacity: 0.12 },
    },
    bar: {
      itemStyle: { borderRadius: [radiusPx, radiusPx, 0, 0] },
    },
    tooltip: {
      backgroundColor: token('--nwp-glass-surface'),
      borderColor: token('--nwp-glass-border'),
      borderWidth: 1,
      padding: [8, 12],
      textStyle: { color: text, fontFamily: fontUi, fontSize: 12 },
      extraCssText: `border-radius: ${radius}; box-shadow: ${token('--nwp-shadow-md')};`,
    },
    legend: {
      icon: 'circle',
      itemWidth: 8,
      itemHeight: 8,
      textStyle: { color: textSecondary, fontFamily: fontUi, fontSize: 12 },
    },
  };
}

/** Register (or refresh) the theme and return its stable ECharts name. */
export function registerChartTheme(): string {
  registerTheme(THEME_NAME, buildChartTheme());
  return THEME_NAME;
}
