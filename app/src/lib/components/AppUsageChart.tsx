import React, { useEffect, useRef, useState } from 'react';
import * as echarts from 'echarts';
import {
    LINEAR_COLORS,
    LINEAR_SERIES_ORDER,
    getLinearTooltipConfig,
} from './linear/echarts-linear-theme';
import { DonutLegend, type DonutLegendItem } from './linear/DonutLegend';
import { formatBytes } from '$lib/utils/format';

export interface AppUsageSlice {
    name: string;
    bytes: number;
}

export interface AppUsageChartProps {
    slices: AppUsageSlice[];
    totalBytes?: number;
    className?: string;
}

function detectDarkTheme(): boolean {
    if (typeof document === 'undefined') return true;
    return document.documentElement.getAttribute('data-theme') !== 'light';
}

export const AppUsageChart: React.FC<AppUsageChartProps> = ({
    slices,
    totalBytes,
    className = '',
}) => {
    const containerRef = useRef<HTMLDivElement>(null);
    const instanceRef = useRef<echarts.ECharts | null>(null);
    const [isDark, setIsDark] = useState<boolean>(detectDarkTheme);

    useEffect(() => {
        const onTheme = () => setIsDark(detectDarkTheme());
        window.addEventListener('theme-changed', onTheme);
        return () => window.removeEventListener('theme-changed', onTheme);
    }, []);

    const computedTotal =
        totalBytes ?? slices.reduce((sum, slice) => sum + Math.max(0, slice.bytes), 0);

    const legendItems: DonutLegendItem[] = slices.map((slice, idx) => {
        const color = LINEAR_SERIES_ORDER[idx % LINEAR_SERIES_ORDER.length] ?? LINEAR_COLORS.blue;
        const pct = computedTotal > 0 ? Math.round((slice.bytes / computedTotal) * 100) : 0;
        return {
            label: slice.name,
            value: formatBytes(slice.bytes),
            subValue: `${pct}%`,
            color,
        };
    });

    useEffect(() => {
        const el = containerRef.current;
        if (!el) return;

        if (!instanceRef.current) {
            instanceRef.current = echarts.init(el);
        }
        const chart = instanceRef.current;
        const tickColor = isDark ? LINEAR_COLORS.borderDark : LINEAR_COLORS.borderLight;
        const textColor = isDark ? LINEAR_COLORS.textDark : LINEAR_COLORS.textLight;
        const mutedColor = isDark ? LINEAR_COLORS.mutedDark : LINEAR_COLORS.mutedLight;

        const option: echarts.EChartsCoreOption = {
            animation: false,
            backgroundColor: 'transparent',
            tooltip: {
                ...getLinearTooltipConfig(isDark),
                trigger: 'item',
                formatter: (params: unknown) => {
                    const item = params as { name?: string; value?: number; percent?: number };
                    return `<div style="font-weight:600;margin-bottom:2px;">${item.name ?? ''}</div><div>${formatBytes(Number(item.value ?? 0))} (${String(item.percent ?? 0)}%)</div>`;
                },
            },
            series: [
                // Thin outer tick ring
                {
                    type: 'gauge',
                    radius: '92%',
                    startAngle: 90,
                    endAngle: -270,
                    min: 0,
                    max: 100,
                    splitNumber: 12,
                    axisLine: { show: false },
                    splitLine: { show: false },
                    axisLabel: { show: false },
                    pointer: { show: false },
                    detail: { show: false },
                    axisTick: {
                        distance: -2,
                        length: 4,
                        splitNumber: 4,
                        lineStyle: { color: tickColor, width: 1 },
                    },
                    silent: true,
                },
                // Primary donut with rounded arc ends
                {
                    name: 'App usage',
                    type: 'pie',
                    radius: ['62%', '78%'],
                    center: ['50%', '50%'],
                    padAngle: 2,
                    itemStyle: {
                        borderRadius: 6,
                    },
                    label: {
                        show: true,
                        position: 'center',
                        formatter: () => `{total|${formatBytes(computedTotal)}}\n{caption|Total}`,
                        rich: {
                            total: {
                                fontSize: 16,
                                fontWeight: 600,
                                color: textColor,
                                lineHeight: 22,
                                fontFamily: 'Inter, system-ui, -apple-system, sans-serif',
                            },
                            caption: {
                                fontSize: 11,
                                color: mutedColor,
                                fontFamily: 'Inter, system-ui, -apple-system, sans-serif',
                            },
                        },
                    },
                    labelLine: { show: false },
                    color: [...LINEAR_SERIES_ORDER],
                    data: slices.map((slice) => ({
                        name: slice.name,
                        value: slice.bytes,
                    })),
                },
            ],
        };

        chart.setOption(option, true);
    }, [slices, computedTotal, isDark]);

    useEffect(() => {
        const el = containerRef.current;
        if (!el || typeof ResizeObserver === 'undefined') return;
        const observer = new ResizeObserver(() => {
            instanceRef.current?.resize();
        });
        observer.observe(el);
        return () => {
            observer.disconnect();
            instanceRef.current?.dispose();
            instanceRef.current = null;
        };
    }, []);

    return (
        <div className={`flex flex-col items-center gap-4 min-w-0 ${className}`}>
            <div
                ref={containerRef}
                className="h-40 w-40 mx-auto shrink-0"
                role="img"
                aria-label="Application usage distribution donut chart"
            />
            <DonutLegend items={legendItems} />
            <table className="sr-only">
                <caption>Application usage split</caption>
                <thead>
                    <tr>
                        <th>Application</th>
                        <th>Total</th>
                    </tr>
                </thead>
                <tbody>
                    {slices.map((s, idx) => (
                        <tr key={`${s.name}-${String(idx)}`}>
                            <td>{s.name}</td>
                            <td>{formatBytes(s.bytes)}</td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
};
