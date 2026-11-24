import { describe, expect, it } from 'vitest';
import { budgetsStore, type BudgetState } from './budgets';

describe('budgetsStore.percent', () => {
  it('returns 0 when limit_bytes is zero or negative', () => {
    const zeroLimit: BudgetState = { used_bytes: 1000, limit_bytes: 0, level: 'ok' };
    const negativeLimit: BudgetState = { used_bytes: 1000, limit_bytes: -10, level: 'ok' };

    expect(budgetsStore.percent(zeroLimit)).toBe(0);
    expect(budgetsStore.percent(negativeLimit)).toBe(0);
  });

  it('clamps negative used_bytes to 0%', () => {
    const negativeUsed: BudgetState = { used_bytes: -500, limit_bytes: 1000, level: 'ok' };
    expect(budgetsStore.percent(negativeUsed)).toBe(0);
  });

  it('calculates standard integer percentage rounded to nearest whole number', () => {
    const half: BudgetState = { used_bytes: 500, limit_bytes: 1000, level: 'ok' };
    const eightyPercent: BudgetState = { used_bytes: 800, limit_bytes: 1000, level: 'warning' };
    const hundredPercent: BudgetState = { used_bytes: 1000, limit_bytes: 1000, level: 'breach' };

    expect(budgetsStore.percent(half)).toBe(50);
    expect(budgetsStore.percent(eightyPercent)).toBe(80);
    expect(budgetsStore.percent(hundredPercent)).toBe(100);
  });

  it('clamps extreme over-budget usage to 999%', () => {
    const massiveOveruse: BudgetState = {
      used_bytes: 100_000_000,
      limit_bytes: 10_000,
      level: 'breach',
    };
    expect(budgetsStore.percent(massiveOveruse)).toBe(999);
  });
});
