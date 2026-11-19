// OQ4 (P8.6): read-only demo mode with in-memory sample data. No daemon, no
// disk writes beyond the local preference flag, no network.

import { historyStore } from './history.svelte';
import { onboardingStore } from './onboarding.svelte';
import { usageStore } from './usage.svelte';

const DEMO_KEY = 'nwp.demo.active';

let active = $state(false);

try {
  active = localStorage.getItem(DEMO_KEY) === '1';
} catch {
  active = false;
}
if (active) {
  usageStore.enterDemo();
  historyStore.enterDemo();
}

function start(): void {
  active = true;
  try {
    localStorage.setItem(DEMO_KEY, '1');
  } catch {
    // Demo still works for this session without persistence.
  }
  usageStore.enterDemo();
  historyStore.enterDemo();
}

function stop(): void {
  active = false;
  try {
    localStorage.removeItem(DEMO_KEY);
  } catch {
    // Ignore unavailable storage.
  }
  usageStore.exitDemo();
  historyStore.exitDemo();
  onboardingStore.reset();
  onboardingStore.begin();
}

export const demoStore = {
  get active(): boolean {
    return active;
  },
  start,
  stop,
};
