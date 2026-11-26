import * as echarts from 'echarts';

export function getCssToken(name: string): string {
  if (typeof document === 'undefined') return '';
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

export function resolveCssColor(tokenOrValue: string): string {
  const trimmed = tokenOrValue.trim();
  const match = /^var\((--[^)]+)\)$/.exec(trimmed);
  const token = match ? (match[1] ?? '') : trimmed.startsWith('--') ? trimmed : '';
  return token ? getCssToken(token) : trimmed;
}

/** NWP series order per color migration plan §5 (skipping purple due to <25 distance from primary blue). */
export const LINEAR_SERIES_TOKENS = ['--primary', '--sky', '--amber', '--pink'] as const;

export function getLinearSeriesOrder(): string[] {
  return LINEAR_SERIES_TOKENS.map((token) => getCssToken(token));
}

export function lineAreaGradient(cssColor: string) {
  if (!cssColor.startsWith('#') || cssColor.length !== 7) {
    return 'transparent';
  }
  return new echarts.graphic.LinearGradient(0, 0, 0, 1, [
    { offset: 0, color: `${cssColor}26` },
    { offset: 1, color: `${cssColor}00` },
  ]);
}

export function getLinearTooltipConfig() {
  const bg = getCssToken('--bg-surface-2');
  const border = getCssToken('--border-strong');
  const text = getCssToken('--text');

  return {
    backgroundColor: bg,
    borderColor: border,
    borderWidth: 1,
    padding: [8, 12],
    textStyle: {
      color: text,
      fontSize: 12,
      fontFamily: 'Inter, system-ui, -apple-system, sans-serif',
    },
    extraCssText: 'border-radius: 4px; box-shadow: var(--shadow-card);',
  };
}

export function getLinearEChartsOption() {
  const border = getCssToken('--border');
  const chartGrid = getCssToken('--chart-grid');
  const muted = getCssToken('--text-muted');

  return {
    animation: false,
    backgroundColor: 'transparent',
    textStyle: {
      fontFamily: 'Inter, system-ui, -apple-system, sans-serif',
      fontSize: 12,
      color: muted,
    },
    grid: {
      top: 16,
      right: 12,
      bottom: 24,
      left: 8,
      containLabel: true,
      borderColor: border,
    },
    tooltip: getLinearTooltipConfig(),
    xAxis: {
      axisLine: {
        lineStyle: { color: border },
      },
      splitLine: { show: false },
      axisTick: { show: false },
      axisLabel: {
        color: muted,
        fontSize: 12,
        fontFamily: 'Inter, system-ui, -apple-system, sans-serif',
      },
    },
    yAxis: {
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: {
        color: muted,
        fontSize: 12,
        fontFamily: 'Inter, system-ui, -apple-system, sans-serif',
      },
      splitLine: {
        show: true,
        lineStyle: {
          type: 'dotted' as const,
          color: chartGrid,
        },
      },
    },
    color: getLinearSeriesOrder(),
  };
}
