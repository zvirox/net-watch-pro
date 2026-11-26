import React, { useEffect, useRef, useState } from 'react';
import * as echarts from 'echarts';
import {
    getCssToken,
    getLinearEChartsOption,
    lineAreaGradient,
} from './linear/echarts-linear-theme';
import { formatBytes } from '$lib/utils/format';

export interface TrafficChartPoint {
    label: string;
    down: number;
    up: number;
}

export interface TrafficChartProps {
    data: TrafficChartPoint[];
    mode?: 'line' | 'bar';
    unit?: 'bytes' | 'rate';
    activeBarIndex?: number;
    className?: string;
    ariaLabel?: string;
}

function detectDarkTheme(): boolean {
    if (typeof document === 'undefined') return true;
    return document.documentElement.getAttribute('data-theme') !== 'light';
}

function formatYAxisValue(val: number, unit: 'bytes' | 'rate'): string {
    if (unit === 'rate') {
        if (val <= 0) return '0';
        const mbps = val / 1_000_000;
        return mbps >= 10 ? `${Math.round(mbps)}M` : `${mbps.toFixed(1)}M`;
    }
    return formatBytes(val);
}

export const TrafficChart: React.FC<TrafficChartProps> = ({
    data,
    mode = 'bar',
    unit = 'bytes',
    activeBarIndex,
    className = 'h-64 w-full',
    ariaLabel = 'Traffic chart',
}) => {
    const containerRef = useRef<HTMLDivElement>(null);
    const instanceRef = useRef<echarts.ECharts | null>(null);
    const [isDark, setIsDark] = useState<boolean>(detectDarkTheme);

    useEffect(() => {
        const onTheme = () => setIsDark(detectDarkTheme());
        window.addEventListener('theme-changed', onTheme);
        const observer =
            typeof MutationObserver !== 'undefined'
                ? new MutationObserver(() => setIsDark(detectDarkTheme()))
                : null;
        observer?.observe(document.documentElement, {
            attributes: true,
            attributeFilter: ['data-theme'],
        });
        return () => {
            window.removeEventListener('theme-changed', onTheme);
            observer?.disconnect();
        };
    }, []);

    useEffect(() => {
        const el = containerRef.current;
        if (!el) return;

        if (!instanceRef.current) {
            instanceRef.current = echarts.init(el);
        }
        const chart = instanceRef.current;

        const base = getLinearEChartsOption();
        const labels = data.map((d) => d.label);
        const primaryColor = getCssToken('--primary');
        const uploadColor = getCssToken('--orange');
        const dotColor = getCssToken('--sky');
        const idleColor = getCssToken('--bg-bar-idle');
        const highlightIdx =
            activeBarIndex !== undefined ? activeBarIndex : Math.max(0, data.length - 1);

        const series =
            mode === 'line'
                ? [
                      {
                          name: 'Download',
                          type: 'line',
                          smooth: false,
                          showSymbol: false,
                          lineStyle: { width: 2, color: primaryColor },
                          itemStyle: { color: primaryColor },
                          areaStyle: { color: lineAreaGradient(primaryColor) },
                          data: data.map((d) => d.down),
                      },
                      {
                          name: 'Upload',
                          type: 'line',
                          smooth: false,
                          showSymbol: false,
                          lineStyle: { width: 2, color: uploadColor },
                          itemStyle: { color: uploadColor },
                          areaStyle: { color: lineAreaGradient(uploadColor) },
                          data: data.map((d) => d.up),
                      },
                  ]
                : [
                      {
                          name: 'Download',
                          type: 'bar',
                          barMaxWidth: 20,
                          data: data.map((d, idx) => ({
                              value: d.down,
                              itemStyle: {
                                  color: idx === highlightIdx ? primaryColor : idleColor,
                                  borderRadius: [4, 4, 0, 0],
                              },
                          })),
                      },
                      {
                          name: 'Upload',
                          type: 'bar',
                          barMaxWidth: 20,
                          data: data.map((d, idx) => ({
                              value: d.up,
                              itemStyle: {
                                  color: idx === highlightIdx ? uploadColor : idleColor,
                                  borderRadius: [4, 4, 0, 0],
                              },
                          })),
                      },
                  ];

        const option: echarts.EChartsCoreOption = {
            ...base,
            tooltip: {
                ...base.tooltip,
                trigger: 'axis',
                axisPointer: {
                    type: mode === 'bar' ? 'shadow' : 'line',
                },
                formatter: (params: unknown) => {
                    const list = Array.isArray(params) ? params : [];
                    if (list.length === 0) return '';
                    const first = list[0] as { axisValueLabel?: string };
                    const header = `<div style="font-weight:600;margin-bottom:4px;display:flex;align-items:center;gap:6px;"><span style="width:6px;height:6px;border-radius:9999px;background:${dotColor};display:inline-block;"></span><span>${first.axisValueLabel ?? ''}</span></div>`;
                    const rows = list
                        .map((entry) => {
                            const item = entry as {
                                seriesName?: string;
                                value?: number;
                                color?: string;
                            };
                            const val = Number(item.value ?? 0);
                            const formatted =
                                unit === 'rate'
                                    ? `${(val / 1_000_000).toFixed(2)} Mbps`
                                    : formatBytes(val);
                            return `<div style="display:flex;justify-content:space-between;gap:16px;font-variant-numeric:tabular-nums;"><span>${item.seriesName ?? ''}</span><strong>${formatted}</strong></div>`;
                        })
                        .join('');
                    return `${header}${rows}`;
                },
            },
            xAxis: {
                ...base.xAxis,
                type: 'category',
                data: labels,
            },
            yAxis: {
                ...base.yAxis,
                type: 'value',
                axisLabel: {
                    ...base.yAxis.axisLabel,
                    formatter: (val: number) => formatYAxisValue(val, unit),
                },
            },
            series,
        };

        chart.setOption(option, true);
    }, [data, mode, unit, activeBarIndex, isDark]);

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
        <div className="relative w-full">
            <div ref={containerRef} className={className} role="img" aria-label={ariaLabel} />
            <table className="sr-only">
                <caption>{ariaLabel}</caption>
                <thead>
                    <tr>
                        <th>Time</th>
                        <th>Download</th>
                        <th>Upload</th>
                    </tr>
                </thead>
                <tbody>
                    {data.slice(0, 24).map((point, idx) => (
                        <tr key={`${point.label}-${String(idx)}`}>
                            <td>{point.label}</td>
                            <td>
                                {unit === 'rate'
                                    ? `${(point.down / 1_000_000).toFixed(2)} Mbps`
                                    : formatBytes(point.down)}
                            </td>
                            <td>
                                {unit === 'rate'
                                    ? `${(point.up / 1_000_000).toFixed(2)} Mbps`
                                    : formatBytes(point.up)}
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
};
