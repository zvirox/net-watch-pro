import React, { useEffect, useRef } from 'react';
import * as echarts from 'echarts';
import { getLinearEChartsOption } from './linear/echarts-linear-theme';

export interface TrafficChartProps {
    seriesData?: { time: string; up: number; down: number }[];
    isDark?: boolean;
    className?: string;
}

export const TrafficChart: React.FC<TrafficChartProps> = ({
    seriesData,
    isDark = true,
    className = 'h-64 w-full',
}) => {
    const chartRef = useRef<HTMLDivElement>(null);
    const chartInstance = useRef<echarts.ECharts | null>(null);

    useEffect(() => {
        if (!chartRef.current) return;

        if (!chartInstance.current) {
            chartInstance.current = echarts.init(chartRef.current);
        }

        const times = seriesData?.map((d) => d.time) || [
            '10:00',
            '10:05',
            '10:10',
            '10:15',
            '10:20',
            '10:25',
            '10:30',
        ];
        const downData = seriesData?.map((d) => d.down) || [12, 28, 45, 30, 85, 42, 60];
        const upData = seriesData?.map((d) => d.up) || [5, 12, 18, 14, 32, 20, 25];

        const baseOption = getLinearEChartsOption(isDark);
        const option = {
            ...baseOption,
            tooltip: {
                ...baseOption.tooltip,
                trigger: 'axis',
            },
            xAxis: {
                ...baseOption.xAxis,
                type: 'category',
                data: times,
            },
            yAxis: {
                ...baseOption.yAxis,
                type: 'value',
                axisLabel: {
                    formatter: '{value} MB',
                },
            },
            series: [
                {
                    name: 'Download',
                    type: 'line',
                    smooth: true,
                    showSymbol: false,
                    areaStyle: {
                        opacity: 0.15,
                    },
                    data: downData,
                    color: isDark ? '#22d3ee' : '#0891b2',
                },
                {
                    name: 'Upload',
                    type: 'line',
                    smooth: true,
                    showSymbol: false,
                    areaStyle: {
                        opacity: 0.15,
                    },
                    data: upData,
                    color: isDark ? '#a855f7' : '#7c3aed',
                },
            ],
        };

        chartInstance.current.setOption(option);

        const handleResize = () => chartInstance.current?.resize();
        window.addEventListener('resize', handleResize);

        return () => {
            window.removeEventListener('resize', handleResize);
        };
    }, [seriesData, isDark]);

    return <div ref={chartRef} className={className} />;
};
