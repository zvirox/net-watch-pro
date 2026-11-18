// UI-only settings (P7.6): owns the effective theme and applies it to the
// document root. The persisted preference lives in T1/T2 `theme_default`;
// charts re-theme themselves by observing `data-theme` (Chart.svelte §10).

import { settingsStore } from './settings.svelte';

export type ThemePreference = 'system' | 'light' | 'dark';
export type EffectiveTheme = 'light' | 'dark';

let preference = $state<ThemePreference>('system');
let systemTheme: 'light' | 'dark' = $state('light');
const effective = $derived<EffectiveTheme>(preference === 'system' ? systemTheme : preference);

function apply(): void {
  document.documentElement.dataset.theme = effective;
}

function syncPreference(): void {
  const stored = settingsStore.value('theme_default');
  if (stored === 'system' || stored === 'light' || stored === 'dark') {
    preference = stored;
  }
  apply();
}

settingsStore.onChange(syncPreference);

if (typeof window !== 'undefined') {
  const query = window.matchMedia('(prefers-color-scheme: dark)');
  systemTheme = query.matches ? 'dark' : 'light';
  query.addEventListener('change', (event) => {
    systemTheme = event.matches ? 'dark' : 'light';
    apply();
  });
  apply();
}

async function setTheme(next: ThemePreference): Promise<void> {
  preference = next;
  apply(); // optimistic — the server is the source of truth after persistence
  try {
    await settingsStore.setValue('theme_default', next);
  } catch {
    // The store keeps the server message; revert to the last stored value.
    syncPreference();
  }
}

export const settingsUIStore = {
  get theme(): ThemePreference {
    return preference;
  },
  get effective(): EffectiveTheme {
    return effective;
  },
  setTheme,
};
