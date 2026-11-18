// Budget alerts (P6.9 / D6a): reads the additive `budgets` data on U1 and
// raises a local desktop notification when a configured budget crosses the
// warning or breach threshold. Level dedup prevents a notification storm; the
// app itself makes no network calls (PRD non-goal).

import { isTauri } from '@tauri-apps/api/core';
import { SvelteMap } from 'svelte/reactivity';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';
import { daemonStore } from './daemon.svelte';

export type BudgetLevel = 'ok' | 'warning' | 'breach';

export type BudgetState = {
  used_bytes: number;
  limit_bytes: number;
  level: BudgetLevel;
};

export type AppBudgetState = BudgetState & { app_id: number; name: string };

export type BudgetsData = {
  total_daily: BudgetState;
  total_monthly: BudgetState;
  per_app_daily: AppBudgetState[];
  per_app_monthly: AppBudgetState[];
};

const REFRESH_MS = 60_000;
let data = $state<BudgetsData | null>(null);
const lastLevels = new SvelteMap<string, BudgetLevel>();
let timer: ReturnType<typeof setInterval> | null = null;

function percent(state: BudgetState): number {
  if (state.limit_bytes <= 0) return 0;
  return Math.min(999, Math.round((state.used_bytes / state.limit_bytes) * 100));
}

async function notify(): Promise<void> {
  if (!isTauri()) return;
  let granted = await isPermissionGranted();
  if (!granted) granted = (await requestPermission()) === 'granted';
  if (!granted || data === null) return;

  const entries: Array<[string, string, BudgetState]> = [
    ['total-daily', 'Daily total', data.total_daily],
    ['total-monthly', 'Monthly total', data.total_monthly],
    ...data.per_app_daily.map((item): [string, string, BudgetState] => [
      `app-daily-${String(item.app_id)}`,
      `${item.name} (daily)`,
      item,
    ]),
    ...data.per_app_monthly.map((item): [string, string, BudgetState] => [
      `app-monthly-${String(item.app_id)}`,
      `${item.name} (monthly)`,
      item,
    ]),
  ];

  for (const [key, label, state] of entries) {
    const previous = lastLevels.get(key) ?? 'ok';
    lastLevels.set(key, state.level);
    if (state.limit_bytes <= 0 || state.level === 'ok' || state.level === previous) continue;
    const title = state.level === 'breach' ? 'Budget exceeded' : 'Budget warning';
    sendNotification({
      title,
      body: `${label} is at ${String(percent(state))}% of its budget.`,
    });
  }
}

async function refresh(): Promise<void> {
  if (daemonStore.connection !== 'connected') return;
  try {
    const summary = (await daemonStore.request('GET', '/api/v1/usage/summary')) as {
      budgets: BudgetsData;
    };
    data = summary.budgets;
    await notify();
  } catch {
    // Budget alerts are advisory; a failed poll must not break the UI.
  }
}

function start(): void {
  void refresh();
  if (timer === null) timer = setInterval(() => void refresh(), REFRESH_MS);
}

function stop(): void {
  if (timer !== null) clearInterval(timer);
  timer = null;
}

daemonStore.onConnection((state) => {
  if (state === 'connected') void refresh();
  else {
    data = null;
    lastLevels.clear();
  }
});

export const budgetsStore = {
  get data(): BudgetsData | null {
    return data;
  },
  percent,
  start,
  stop,
  refresh,
};
