// First-run onboarding (P8.5): shown only in the Tauri app when the daemon
// isn't linked yet. One pkexec prompt registers the UID and enables the unit;
// success is remembered locally so later launches never prompt again (D1a).

import { isTauri } from '@tauri-apps/api/core';
import { enableMonitoring } from '$lib/api/transport';
import { daemonStore } from './daemon.svelte';

const DONE_KEY = 'nwp.onboarding.done';
const HEALTH_ATTEMPTS = 25;
const HEALTH_DELAY_MS = 200;

let visible = $state(false);
let checked = $state(false);
let enabling = $state(false);
let error = $state<string | null>(null);
let started = false;

function finish(): void {
  visible = false;
  error = null;
  try {
    localStorage.setItem(DONE_KEY, '1');
  } catch {
    // Storage may be unavailable; the session still continues.
  }
}

/** Called once from App onMount; decides whether onboarding is needed. */
function begin(): void {
  if (started) return;
  started = true;
  if (!isTauri()) {
    checked = true; // browser preview: no daemon/pkexec, keep the honest empty UI
    return;
  }
  let demo = false;
  try {
    demo = localStorage.getItem('nwp.demo.active') === '1';
  } catch {
    demo = false;
  }
  if (demo) {
    checked = true;
    return;
  }
  let done = false;
  try {
    done = localStorage.getItem(DONE_KEY) === '1';
  } catch {
    done = false;
  }
  if (done || daemonStore.connection === 'connected') {
    checked = true;
    return;
  }
  daemonStore.onConnection((state) => {
    if (state === 'connected') finish();
  });
  // Short grace period for the normal socket connection before prompting.
  setTimeout(() => {
    if (daemonStore.connection !== 'connected') visible = true;
    checked = true;
  }, 1500);
}

function reset(): void {
  started = false;
  checked = false;
  visible = false;
  error = null;
}

async function enable(): Promise<void> {
  enabling = true;
  error = null;
  try {
    await enableMonitoring();
    await daemonStore.authorize();
    for (let attempt = 0; attempt < HEALTH_ATTEMPTS; attempt += 1) {
      try {
        await daemonStore.request('GET', '/api/v1/health');
        finish();
        return;
      } catch {
        await new Promise<void>((resolve) => {
          setTimeout(resolve, HEALTH_DELAY_MS);
        });
      }
    }
    throw new Error('Daemon did not become healthy after enabling. Check system logs.');
  } catch (err) {
    error = err instanceof Error ? err.message : 'Monitoring could not be enabled';
  } finally {
    enabling = false;
  }
}

export const onboardingStore = {
  get visible(): boolean {
    return visible;
  },
  get checked(): boolean {
    return checked;
  },
  get enabling(): boolean {
    return enabling;
  },
  get error(): string | null {
    return error;
  },
  /** Actionable next step paired with the raw error (DESIGN.md §8.2). */
  get guidance(): string | null {
    if (error === null) return null;
    const text = error.toLowerCase();
    if (text.includes('pkexec')) {
      return 'Install polkit/pkexec or cancel the prompt and retry from your desktop session.';
    }
    if (
      text.includes('systemctl') ||
      text.includes('not authorized') ||
      text.includes('access denied')
    ) {
      return 'Admin authorization was refused or systemd rejected the service. Ask your administrator, then retry.';
    }
    if (text.includes('uid')) {
      return 'The desktop user could not be identified. Launch the app from your normal user session.';
    }
    return 'Check the system service status, then retry. The demo mode lets you explore the UI meanwhile.';
  },
  begin,
  enable,
  reset,
};
