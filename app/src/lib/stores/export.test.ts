import { describe, expect, it, beforeEach } from 'vitest';
import { exportStore } from './export';
import { licenseStore } from './license';

describe('exportStore', () => {
  beforeEach(() => {
    exportStore.clear();
  });

  it('initializes with idle state', () => {
    expect(exportStore.busy).toBe(false);
    expect(exportStore.status).toBeNull();
    expect(exportStore.error).toBeNull();
  });

  it('clear() resets status and error', () => {
    exportStore.clear();
    expect(exportStore.status).toBeNull();
    expect(exportStore.error).toBeNull();
  });

  it('opens upgrade modal when non-pro user attempts export', async () => {
    // Default tier is free
    expect(licenseStore.isPro).toBe(false);
    await exportStore.download('csv');
    expect(licenseStore.upgradeOpen).toBe(true);
    licenseStore.closeUpgrade();
  });
});
