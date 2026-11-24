export const licenseStore = {
  isPro: false,
  upgradeOpen: false,
  openUpgrade() {
    this.upgradeOpen = true;
  },
  closeUpgrade() {
    this.upgradeOpen = false;
  },
};
