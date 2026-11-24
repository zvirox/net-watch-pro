export interface BudgetState {
  used_bytes: number;
  limit_bytes: number;
  level: 'ok' | 'warning' | 'critical' | 'breach';
}

export const budgetsStore = {
  percent(budget: BudgetState): number {
    if (budget.limit_bytes <= 0) return 0;
    const used = Math.max(0, budget.used_bytes);
    return Math.min(999, Math.round((used / budget.limit_bytes) * 100));
  },
};
