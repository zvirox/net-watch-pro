<!-- Reusable ECharts surface. DESIGN.md §3.3, §7.14, §9.4, §10. -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { BarChart, LineChart } from 'echarts/charts';
  import { GridComponent, LegendComponent, TooltipComponent } from 'echarts/components';
  import * as echarts from 'echarts/core';
  import type { EChartsCoreOption, EChartsType } from 'echarts/core';
  import { CanvasRenderer } from 'echarts/renderers';
  import { registerChartTheme } from './theme';

  // Only the chart types and components used by the dashboard are bundled.
  echarts.use([
    LineChart,
    BarChart,
    GridComponent,
    TooltipComponent,
    LegendComponent,
    CanvasRenderer,
  ]);

  interface Props {
    /** ECharts option with no color/font literals; theme supplies those. */
    option: EChartsCoreOption;
    /** Accessible description of the series and time range. */
    ariaLabel: string;
    /** Optional ECharts event handlers, keyed by event name. */
    onEvents?: Record<string, (params: unknown) => void>;
    /** Parent-supplied class, used for chart height/layout. */
    class?: string;
  }

  let { option, ariaLabel, onEvents = {}, class: className = '' }: Props = $props();

  let container: HTMLDivElement;
  let chart: EChartsType | undefined;
  let resizeObserver: ResizeObserver | undefined;
  let themeObserver: MutationObserver | undefined;

  /** §6.1: read motion durations from the token layer, never hard-code them here. */
  function tokenMs(name: string, fallback: number): number {
    const raw = window.getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    const value = Number.parseFloat(raw);
    return raw.endsWith('ms') && Number.isFinite(value) ? value : fallback;
  }

  /** §9.5: reduced motion disables all chart animation, sparkline included. */
  function applyOption(next: EChartsCoreOption): void {
    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    const optionWithMotion: EChartsCoreOption = {
      animationDuration: tokenMs('--nwp-dur-slow', 240),
      animationDurationUpdate: tokenMs('--nwp-dur-base', 160),
      animationEasing: 'cubicOut' as const,
      ...next,
    };
    chart?.setOption(reduced ? { ...optionWithMotion, animation: false } : optionWithMotion, true);
  }

  function create(): void {
    chart?.dispose();
    chart = echarts.init(container, registerChartTheme(), { renderer: 'canvas' });
    applyOption(option);
    Object.entries(onEvents).forEach(([event, handler]) => {
      chart?.on(event, handler);
    });
    container.querySelector('canvas')?.setAttribute('aria-hidden', 'true');
  }

  onMount(() => {
    create();
    // DESIGN.md §5.4: charts resize with their container, not the window.
    resizeObserver = new ResizeObserver(() => {
      chart?.resize();
    });
    resizeObserver.observe(container);
    // DESIGN.md §10: re-register the theme when the active token set changes.
    themeObserver = new MutationObserver(() => {
      create();
    });
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme'],
    });
    return () => {
      resizeObserver?.disconnect();
      themeObserver?.disconnect();
      chart?.dispose();
      chart = undefined;
    };
  });

  $effect(() => {
    applyOption(option);
  });
</script>

<div class="chart-wrapper {className}" role="img" aria-label={ariaLabel}>
  <div class="chart" bind:this={container}></div>
</div>

<style>
  .chart-wrapper {
    width: 100%;
    height: 100%;
    min-height: var(--nwp-space-16);
  }
  .chart {
    width: 100%;
    height: 100%;
    min-height: inherit;
  }
</style>
