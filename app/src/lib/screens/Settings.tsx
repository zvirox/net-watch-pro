import React, { useState } from 'react';
import { LinearButton, LinearCard, ProgressRow } from '$lib/components/linear';
import { budgetsStore } from '$lib/stores/budgets';
import { useDaemonStore, type RangePreset } from '$lib/stores/useDaemonStore';
import { formatBytes } from '$lib/utils/format';

const TABS = ['General', 'Data', 'Budgets', 'Interfaces', 'Daemon', 'License', 'About'] as const;
type SettingsTab = (typeof TABS)[number];

const APP_VERSION = typeof __NWP_VERSION__ === 'string' ? `v${__NWP_VERSION__}` : 'v0.9.2-beta.5';

function budgetColorVar(level: string): string {
    if (level === 'breach' || level === 'critical') return 'var(--red)';
    if (level === 'warning') return 'var(--amber)';
    return 'var(--sky)';
}

export const Settings: React.FC = () => {
    const {
        settingsMap,
        settingsError,
        speedUnit,
        rangePreset,
        interfaces,
        summary,
        isPro,
        updateSetting,
        toggleInterfaceFlag,
        setActiveScreen,
        openUpgrade,
    } = useDaemonStore();

    const [tab, setTab] = useState<SettingsTab>('General');
    const [savedNote, setSavedNote] = useState<string | null>(null);

    const handleSave = async (key: string, value: unknown) => {
        await updateSetting(key, value);
        setSavedNote(`Updated ${key}`);
        window.setTimeout(() => setSavedNote(null), 2000);
    };

    const dailyBudget = summary?.budgets.total_daily ?? {
        used_bytes: 0,
        limit_bytes: 0,
        level: 'ok' as const,
    };
    const monthlyBudget = summary?.budgets.total_monthly ?? {
        used_bytes: 0,
        limit_bytes: 0,
        level: 'ok' as const,
    };

    return (
        <div className="space-y-4 max-w-4xl">
            <div className="flex flex-wrap items-center justify-between gap-3">
                <div>
                    <h1 className="text-base font-semibold text-[var(--text)]">Settings</h1>
                    <p className="text-xs text-[var(--text-muted)]">
                        Local daemon configuration, budgets, and interface policies
                    </p>
                </div>
                {savedNote && (
                    <span className="text-xs text-[var(--sky-text)] font-medium" role="status">
                        {savedNote}
                    </span>
                )}
            </div>

            {/* Section Tabs */}
            <div
                className="flex flex-wrap items-center gap-4 border-b border-[var(--border)]"
                role="tablist"
            >
                {TABS.map((t) => (
                    <button
                        key={t}
                        type="button"
                        role="tab"
                        aria-selected={tab === t}
                        onClick={() => setTab(t)}
                        className={`pb-2.5 text-[13px] font-medium border-b-2 transition-colors duration-150 cursor-pointer ${
                            tab === t
                                ? 'border-[var(--primary-text)] text-[var(--text)]'
                                : 'border-transparent text-[var(--text-muted)] hover:text-[var(--text-secondary)]'
                        }`}
                    >
                        {t}
                    </button>
                ))}
            </div>

            {settingsError && (
                <LinearCard
                    level={1}
                    className="p-3 border-[var(--red)] bg-[var(--chip-red)] text-xs text-[var(--red-text)]"
                    role="alert"
                >
                    {settingsError}
                </LinearCard>
            )}

            {tab === 'General' && (
                <LinearCard level={1} className="p-5 space-y-5">
                    <div>
                        <h2 className="text-sm font-semibold text-[var(--text)]">
                            Units &amp; Defaults
                        </h2>
                        <p className="text-xs text-[var(--text-muted)] mt-0.5">
                            Dark and light appearance can be switched anytime from the title bar
                            toggle.
                        </p>
                    </div>

                    <div className="space-y-2">
                        <span className="block text-xs font-medium text-[var(--text)]">
                            Live speed unit
                        </span>
                        <div className="inline-flex p-0.5 rounded-md border border-[var(--border)] bg-[var(--bg-surface-2)]">
                            {(
                                [
                                    { id: 'mbps', label: 'Mbps' },
                                    { id: 'mbytes', label: 'MB/s' },
                                ] as const
                            ).map((opt) => (
                                <button
                                    key={opt.id}
                                    type="button"
                                    onClick={() => void handleSave('speed_unit', opt.id)}
                                    className={`px-3 py-1 rounded text-xs font-medium cursor-pointer ${
                                        speedUnit === opt.id
                                            ? 'bg-[var(--bg-surface)] text-[var(--text)] border border-[var(--border)]'
                                            : 'text-[var(--text-muted)] hover:text-[var(--text)] border border-transparent'
                                    }`}
                                >
                                    {opt.label}
                                </button>
                            ))}
                        </div>
                    </div>

                    <div className="space-y-2">
                        <span className="block text-xs font-medium text-[var(--text)]">
                            Default usage range
                        </span>
                        <div className="inline-flex flex-wrap p-0.5 rounded-md border border-[var(--border)] bg-[var(--bg-surface-2)]">
                            {(['1h', '24h', '7d', '30d', 'cycle'] as RangePreset[]).map((opt) => (
                                <button
                                    key={opt}
                                    type="button"
                                    onClick={() => void handleSave('range_default', opt)}
                                    className={`px-3 py-1 rounded text-xs font-medium cursor-pointer ${
                                        rangePreset === opt
                                            ? 'bg-[var(--bg-surface)] text-[var(--text)] border border-[var(--border)]'
                                            : 'text-[var(--text-muted)] hover:text-[var(--text)] border border-transparent'
                                    }`}
                                >
                                    {opt}
                                </button>
                            ))}
                        </div>
                    </div>
                </LinearCard>
            )}

            {tab === 'Data' && (
                <LinearCard level={1} className="p-5 space-y-4">
                    <h2 className="text-sm font-semibold text-[var(--text)]">
                        Retention &amp; Local SQLite
                    </h2>
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                        <label className="space-y-1.5 text-xs">
                            <span className="block font-medium text-[var(--text)]">
                                Retention days
                            </span>
                            <input
                                type="number"
                                min={7}
                                max={isPro ? 365 : 90}
                                defaultValue={Number(settingsMap.retention_days ?? 90)}
                                onBlur={(e) =>
                                    void handleSave('retention_days', Number(e.target.value))
                                }
                                className="w-full h-9 px-3 rounded-md bg-[var(--bg-surface-2)] border border-[var(--border-strong)] text-xs text-[var(--text)] tabular-nums focus:outline-none focus:border-[var(--primary-text)]"
                            />
                            <span className="block text-[var(--text-muted)]">
                                Free tier max 90 days; Pro allows up to 365 days.
                            </span>
                        </label>

                        <label className="space-y-1.5 text-xs">
                            <span className="block font-medium text-[var(--text)]">
                                Billing cycle day (1–31)
                            </span>
                            <input
                                type="number"
                                min={1}
                                max={31}
                                defaultValue={Number(settingsMap.billing_cycle_day ?? 1)}
                                onBlur={(e) =>
                                    void handleSave('billing_cycle_day', Number(e.target.value))
                                }
                                className="w-full h-9 px-3 rounded-md bg-[var(--bg-surface-2)] border border-[var(--border-strong)] text-xs text-[var(--text)] tabular-nums focus:outline-none focus:border-[var(--primary-text)]"
                            />
                        </label>

                        <label className="space-y-1.5 text-xs sm:col-span-2">
                            <span className="block font-medium text-[var(--text)]">
                                Timezone (IANA)
                            </span>
                            <input
                                type="text"
                                defaultValue={String(settingsMap.timezone ?? 'UTC')}
                                onBlur={(e) => void handleSave('timezone', e.target.value)}
                                className="w-full h-9 px-3 rounded-md bg-[var(--bg-surface-2)] border border-[var(--border-strong)] text-xs text-[var(--text)] focus:outline-none focus:border-[var(--primary-text)]"
                            />
                        </label>
                    </div>
                </LinearCard>
            )}

            {tab === 'Budgets' && (
                <div className="space-y-4">
                    <LinearCard level={1} className="p-5 space-y-4">
                        <h2 className="text-sm font-semibold text-[var(--text)]">
                            Usage Quotas (MB)
                        </h2>
                        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
                            <label className="space-y-1.5 text-xs">
                                <span className="block font-medium text-[var(--text)]">
                                    Daily limit (MB)
                                </span>
                                <input
                                    type="number"
                                    min={0}
                                    defaultValue={Number(settingsMap.budget_total_daily_mb ?? 0)}
                                    onBlur={(e) =>
                                        void handleSave(
                                            'budget_total_daily_mb',
                                            Number(e.target.value),
                                        )
                                    }
                                    className="w-full h-9 px-3 rounded-md bg-[var(--bg-surface-2)] border border-[var(--border-strong)] text-xs text-[var(--text)] tabular-nums focus:outline-none focus:border-[var(--primary-text)]"
                                />
                            </label>
                            <label className="space-y-1.5 text-xs">
                                <span className="block font-medium text-[var(--text)]">
                                    Monthly limit (MB)
                                </span>
                                <input
                                    type="number"
                                    min={0}
                                    defaultValue={Number(settingsMap.budget_total_monthly_mb ?? 0)}
                                    onBlur={(e) =>
                                        void handleSave(
                                            'budget_total_monthly_mb',
                                            Number(e.target.value),
                                        )
                                    }
                                    className="w-full h-9 px-3 rounded-md bg-[var(--bg-surface-2)] border border-[var(--border-strong)] text-xs text-[var(--text)] tabular-nums focus:outline-none focus:border-[var(--primary-text)]"
                                />
                            </label>
                            <label className="space-y-1.5 text-xs">
                                <span className="block font-medium text-[var(--text)]">
                                    Warn threshold (%)
                                </span>
                                <input
                                    type="number"
                                    min={1}
                                    max={99}
                                    defaultValue={Number(settingsMap.budget_warn_pct ?? 80)}
                                    onBlur={(e) =>
                                        void handleSave('budget_warn_pct', Number(e.target.value))
                                    }
                                    className="w-full h-9 px-3 rounded-md bg-[var(--bg-surface-2)] border border-[var(--border-strong)] text-xs text-[var(--text)] tabular-nums focus:outline-none focus:border-[var(--primary-text)]"
                                />
                            </label>
                        </div>
                    </LinearCard>

                    <LinearCard level={1} className="p-5 space-y-3">
                        <h3 className="text-sm font-semibold text-[var(--text)]">
                            Current Utilization
                        </h3>
                        <ProgressRow
                            label="Daily budget"
                            value={
                                dailyBudget.limit_bytes > 0 ? budgetsStore.percent(dailyBudget) : 0
                            }
                            displayValue={
                                dailyBudget.limit_bytes > 0
                                    ? `${formatBytes(dailyBudget.used_bytes)} / ${formatBytes(dailyBudget.limit_bytes)}`
                                    : 'Disabled (0 MB)'
                            }
                            colorVar={budgetColorVar(dailyBudget.level)}
                        />
                        <ProgressRow
                            label="Monthly budget"
                            value={
                                monthlyBudget.limit_bytes > 0
                                    ? budgetsStore.percent(monthlyBudget)
                                    : 0
                            }
                            displayValue={
                                monthlyBudget.limit_bytes > 0
                                    ? `${formatBytes(monthlyBudget.used_bytes)} / ${formatBytes(monthlyBudget.limit_bytes)}`
                                    : 'Disabled (0 MB)'
                            }
                            colorVar={budgetColorVar(monthlyBudget.level)}
                        />
                    </LinearCard>
                </div>
            )}

            {tab === 'Interfaces' && (
                <LinearCard level={1} className="p-5 space-y-3">
                    <h2 className="text-sm font-semibold text-[var(--text)]">Interface Labeling</h2>
                    <div className="divide-y divide-[var(--border)]">
                        {interfaces.map((iface) => (
                            <div
                                key={iface.name}
                                className="py-3 flex flex-wrap items-center justify-between gap-3 text-xs"
                            >
                                <div>
                                    <div className="font-medium text-[var(--text)]">
                                        {iface.name}
                                    </div>
                                    <div className="text-[var(--text-muted)]">
                                        {iface.kind ?? 'ethernet'} · ↓{' '}
                                        {formatBytes(iface.bytes?.down ?? iface.bytes_rx ?? 0)} / ↑{' '}
                                        {formatBytes(iface.bytes?.up ?? iface.bytes_tx ?? 0)}
                                    </div>
                                </div>
                                <div className="flex items-center gap-3">
                                    <label className="inline-flex items-center gap-1.5 cursor-pointer">
                                        <input
                                            type="checkbox"
                                            checked={Boolean(iface.is_metered)}
                                            onChange={(e) => {
                                                if (iface.id)
                                                    void toggleInterfaceFlag(
                                                        iface.id,
                                                        'is_metered',
                                                        e.target.checked,
                                                    );
                                            }}
                                        />
                                        <span>Metered</span>
                                    </label>
                                </div>
                            </div>
                        ))}
                    </div>
                </LinearCard>
            )}

            {tab === 'Daemon' && (
                <LinearCard level={1} className="p-5 space-y-4">
                    <h2 className="text-sm font-semibold text-[var(--text)]">Systemd Service</h2>
                    <label className="flex items-center justify-between gap-4 text-xs cursor-pointer">
                        <div>
                            <div className="font-medium text-[var(--text)]">
                                Start netwatchd on boot
                            </div>
                            <div className="text-[var(--text-muted)]">
                                Synced to systemd unit enable/disable state
                            </div>
                        </div>
                        <input
                            type="checkbox"
                            checked={settingsMap.start_on_boot !== false}
                            onChange={(e) => void handleSave('start_on_boot', e.target.checked)}
                        />
                    </label>
                    <div className="pt-2 border-t border-[var(--border)]">
                        <LinearButton
                            variant="secondary"
                            size="sm"
                            onClick={() => setActiveScreen('diagnostics')}
                        >
                            Open Diagnostics
                        </LinearButton>
                    </div>
                </LinearCard>
            )}

            {tab === 'License' && (
                <LinearCard
                    level={1}
                    className="p-5 flex flex-wrap items-center justify-between gap-4"
                >
                    <div>
                        <h2 className="text-sm font-semibold text-[var(--text)]">License Status</h2>
                        <p className="text-xs text-[var(--text-muted)] mt-0.5">
                            Current tier:{' '}
                            <strong>{isPro ? 'Pro (offline verified)' : 'Free'}</strong>
                        </p>
                    </div>
                    <LinearButton variant="primary" size="sm" onClick={openUpgrade}>
                        {isPro ? 'Manage license' : 'Enter license key'}
                    </LinearButton>
                </LinearCard>
            )}

            {tab === 'About' && (
                <LinearCard level={1} className="p-5">
                    <h2 className="text-sm font-semibold text-[var(--text)] mb-3">
                        About Net Watch Pro
                    </h2>
                    <dl className="grid grid-cols-[120px_1fr] gap-y-2 text-xs">
                        <dt className="text-[var(--text-muted)]">Version</dt>
                        <dd className="font-medium text-[var(--text)] tabular-nums">
                            {APP_VERSION}
                        </dd>
                        <dt className="text-[var(--text-muted)]">License</dt>
                        <dd className="text-[var(--text)]">AGPL-3.0-or-later</dd>
                        <dt className="text-[var(--text-muted)]">Privacy</dt>
                        <dd className="text-[var(--text)]">
                            Zero cloud, zero telemetry, zero outbound requests.
                        </dd>
                    </dl>
                </LinearCard>
            )}
        </div>
    );
};
