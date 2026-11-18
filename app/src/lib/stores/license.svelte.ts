// License store (ARCHITECTURE.md §6.1 tier 2): L1–L3 plus the shared upgrade
// modal state. The API broadcasts a `status` frame on activate/deactivate
// (API.md L2/L3); we treat that frame as an invalidation signal and re-read L1
// so Pro unlocks everywhere without a restart (PRD Flow 4).

import { daemonStore } from './daemon.svelte';

export type LicenseTier = 'free' | 'pro';

export type Entitlements = {
  advanced_filters: boolean;
  export: boolean;
  retention_max_days: number;
};

export type LicenseState = {
  tier: LicenseTier;
  entitlements: Entitlements;
};

// DESIGN.md §8.8: the Buy action is a user-triggered external link. The app
// itself never fetches it (PRD non-goal: zero outbound traffic). Until a
// storefront exists, this points at the project's licence information.
export const LICENSE_PURCHASE_URL = 'https://github.com/zvirox/net-watch-pro#license';

const FREE: LicenseState = {
  tier: 'free',
  entitlements: { advanced_filters: false, export: false, retention_max_days: 90 },
};

let license = $state<LicenseState>(FREE);
let loading = $state(false);
let activating = $state(false);
let error = $state<string | null>(null);
let upgradeOpen = $state(false);

type TierListener = (tier: LicenseTier) => void;
// Callback registry, not rendered state (same pattern as daemonStore).
// eslint-disable-next-line svelte/prefer-svelte-reactivity
const tierListeners = new Set<TierListener>();

function setLicense(next: LicenseState): void {
  const changed = license.tier !== next.tier;
  license = next;
  if (changed)
    tierListeners.forEach((listener) => {
      listener(next.tier);
    });
}

function apply(value: unknown): void {
  if (isLicenseState(value)) setLicense(value);
}

function fail(err: unknown, fallback: string): string {
  return err instanceof Error && err.message ? err.message : fallback;
}

async function load(): Promise<void> {
  if (daemonStore.connection !== 'connected') return;
  loading = true;
  try {
    apply(await daemonStore.request('GET', '/api/v1/license'));
    error = null;
  } catch (err) {
    // Honest fallback: never claim Pro when state could not be read.
    setLicense(FREE);
    error = fail(err, 'Could not read the license state');
  } finally {
    loading = false;
  }
}

async function activate(key: string): Promise<boolean> {
  const trimmed = key.trim();
  if (!trimmed) {
    error = 'Enter your license key.';
    return false;
  }
  activating = true;
  error = null;
  try {
    apply(await daemonStore.request('POST', '/api/v1/license/activate', { key: trimmed }));
    upgradeOpen = false;
    return true;
  } catch (err) {
    // API.md §1.3 messages are safe to display and distinct per failure mode.
    error = fail(err, 'Could not activate this key');
    return false;
  } finally {
    activating = false;
  }
}

async function deactivate(): Promise<void> {
  activating = true;
  error = null;
  try {
    apply(await daemonStore.request('DELETE', '/api/v1/license'));
  } catch (err) {
    error = fail(err, 'Could not deactivate the license');
  } finally {
    activating = false;
  }
}

function openUpgrade(): void {
  upgradeOpen = true;
  error = null;
  void load();
}

function closeUpgrade(): void {
  upgradeOpen = false;
  error = null;
}

function clearError(): void {
  error = null;
}

daemonStore.onConnection((state) => {
  if (state === 'connected') {
    void load();
  } else {
    setLicense(FREE);
  }
});

daemonStore.onFrame((frame) => {
  if (isStatusFrame(frame)) void load();
});

export const licenseStore = {
  get tier(): LicenseTier {
    return license.tier;
  },
  get entitlements(): Entitlements {
    return license.entitlements;
  },
  get isPro(): boolean {
    return license.tier === 'pro';
  },
  get loading(): boolean {
    return loading;
  },
  get activating(): boolean {
    return activating;
  },
  get error(): string | null {
    return error;
  },
  get upgradeOpen(): boolean {
    return upgradeOpen;
  },
  load,
  activate,
  deactivate,
  openUpgrade,
  closeUpgrade,
  clearError,
  /** Subscribe to tier flips (e.g. History clears Pro filters on deactivate). */
  onTierChange(listener: TierListener): () => void {
    tierListeners.add(listener);
    listener(license.tier);
    return () => {
      tierListeners.delete(listener);
    };
  },
};

function isLicenseState(value: unknown): value is LicenseState {
  if (typeof value !== 'object' || value === null) return false;
  const candidate = value as Partial<LicenseState>;
  if (candidate.tier !== 'free' && candidate.tier !== 'pro') return false;
  // Widen to unknown: the wire payload is untrusted, so the null check is real
  // even though TypeScript's declared type excludes it.
  const entitlements: unknown = candidate.entitlements;
  if (typeof entitlements !== 'object' || entitlements === null) return false;
  const fields = entitlements as Record<string, unknown>;
  return (
    typeof fields.advanced_filters === 'boolean' &&
    typeof fields.export === 'boolean' &&
    typeof fields.retention_max_days === 'number'
  );
}

function isStatusFrame(value: unknown): boolean {
  return (
    typeof value === 'object' && value !== null && 'channel' in value && value.channel === 'status'
  );
}
