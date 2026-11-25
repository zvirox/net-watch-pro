import * as echarts from 'echarts';

export const LINEAR_COLORS = {
  blue: '#3355FF',
  sky: '#38B6F0',
  green: '#22C55E',
  red: '#F43F5E',
  amber: '#EAB308',
  teal: '#1FB5A5',
  pink: '#EC4899',
  purple: '#7C5CE6',
  orange: '#F97316',
  idleDark: '#202128',
  idleLight: '#EAF0FF',
  surfaceDark: '#111213',
  surfaceLight: '#FFFFFF',
  borderDark: '#1F2125',
  borderLight: '#E5E7EB',
  textDark: '#F5F5F6',
  textLight: '#0F1115',
  mutedDark: '#8A8B91',
  mutedLight: '#6B7280',
} as const;

export const LINEAR_SERIES_ORDER = [
  LINEAR_COLORS.blue,
  LINEAR_COLORS.sky,
  LINEAR_COLORS.green,
  LINEAR_COLORS.amber,
  LINEAR_COLORS.teal,
  LINEAR_COLORS.pink,
  LINEAR_COLORS.purple,
  LINEAR_COLORS.orange,
] as const;

export function lineAreaGradient(hex: string) {
  return new echarts.graphic.LinearGradient(0, 0, 0, 1, [
    { offset: 0, color: `${hex}38` },
    { offset: 1, color: `${hex}00` },
  ]);
}

export function getLinearTooltipConfig(isDark: boolean) {
  const bg = isDark ? LINEAR_COLORS.surfaceDark : LINEAR_COLORS.surfaceLight;
  const border = isDark ? LINEAR_COLORS.borderDark : LINEAR_COLORS.borderLight;
  const text = isDark ? LINEAR_COLORS.textDark : LINEAR_COLORS.textLight;

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
    extraCssText: isDark
      ? 'border-radius: 4px; box-shadow: none;'
      : 'border-radius: 4px; box-shadow: 0 1px 2px rgb(16 24 40 / 5%);',
  };
}

export function getLinearEChartsOption(isDark: boolean = true) {
  const border = isDark ? LINEAR_COLORS.borderDark : LINEAR_COLORS.borderLight;
  const muted = isDark ? LINEAR_COLORS.mutedDark : LINEAR_COLORS.mutedLight;

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
    tooltip: getLinearTooltipConfig(isDark),
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
          color: border,
        },
      },
    },
    color: [...LINEAR_SERIES_ORDER],
  };
}
