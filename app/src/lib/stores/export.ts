import { licenseStore } from './license';

export const exportStore = {
  busy: false,
  status: null as string | null,
  error: null as string | null,
  clear() {
    this.busy = false;
    this.status = null;
    this.error = null;
  },
  async download(format: 'csv' | 'json') {
    if (!licenseStore.isPro) {
      licenseStore.openUpgrade();
      return;
    }
    this.busy = true;
    this.status = `Exporting ${format}...`;
  },
};
