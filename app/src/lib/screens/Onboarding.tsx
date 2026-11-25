import React from 'react';
import { Database, Lock, Shield, Trash2 } from 'lucide-react';
import { LinearButton, LinearCard } from '$lib/components/linear';
import { useDaemonStore } from '$lib/stores/useDaemonStore';

const INFO_CARDS = [
    {
        title: 'What we collect',
        body: 'Connection timestamps, owning application, remote endpoints, and byte counters. Never packet payloads.',
        icon: Shield,
        toneClass: 'bg-[var(--tint-blue)] text-[var(--tint-blue-fg)]',
    },
    {
        title: 'Where it stays',
        body: 'A root-owned SQLite database on this machine. Zero cloud servers, zero telemetry, zero outbound calls.',
        icon: Database,
        toneClass: 'bg-[var(--tint-teal)] text-[var(--tint-teal-fg)]',
    },
    {
        title: 'How to remove it',
        body: 'Uninstall the native package anytime. System units and socket configs clean up cleanly.',
        icon: Trash2,
        toneClass: 'bg-[var(--tint-amber)] text-[var(--tint-amber-fg)]',
    },
];

export const Onboarding: React.FC = () => {
    const { enablingMonitoring, enableError, triggerEnableMonitoring, startDemo } =
        useDaemonStore();

    return (
        <div className="flex-1 flex items-center justify-center p-6 bg-[var(--bg-app)] overflow-y-auto">
            <div className="w-full max-w-3xl space-y-6 text-center">
                <div className="space-y-2">
                    <div className="inline-flex size-11 rounded-lg border border-[var(--border)] bg-[var(--bg-surface)] items-center justify-center mx-auto">
                        <img
                            src="/logo.png"
                            alt="Net Watch Pro"
                            className="size-7 object-contain"
                        />
                    </div>
                    <h1 className="text-xl font-semibold tracking-tight text-[var(--text)]">
                        Net Watch Pro
                    </h1>
                    <p className="text-sm text-[var(--text-muted)] max-w-lg mx-auto">
                        Local Linux network traffic monitor and per-application bandwidth ledger.
                    </p>
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 text-left">
                    {INFO_CARDS.map((card) => {
                        const Icon = card.icon;
                        return (
                            <LinearCard key={card.title} level={1} className="p-5 space-y-2.5">
                                <span
                                    className={`size-8 rounded flex items-center justify-center ${card.toneClass}`}
                                    aria-hidden="true"
                                >
                                    <Icon className="size-4" />
                                </span>
                                <h2 className="text-sm font-semibold text-[var(--text)]">
                                    {card.title}
                                </h2>
                                <p className="text-xs text-[var(--text-muted)] leading-relaxed">
                                    {card.body}
                                </p>
                            </LinearCard>
                        );
                    })}
                </div>

                {enableError && (
                    <LinearCard
                        level={1}
                        className="p-4 border-[var(--accent-red)] bg-[var(--tint-red)] text-left flex flex-wrap items-center justify-between gap-4"
                        role="alert"
                    >
                        <div className="space-y-1">
                            <strong className="text-xs font-semibold text-[var(--tint-red-fg)]">
                                Monitoring could not be enabled
                            </strong>
                            <p className="text-xs text-[var(--text)] font-mono">{enableError}</p>
                        </div>
                        <LinearButton
                            variant="secondary"
                            size="sm"
                            onClick={() => void triggerEnableMonitoring()}
                        >
                            Retry
                        </LinearButton>
                    </LinearCard>
                )}

                <div className="flex flex-wrap items-center justify-center gap-3">
                    <LinearButton
                        variant="primary"
                        size="lg"
                        disabled={enablingMonitoring}
                        onClick={() => void triggerEnableMonitoring()}
                    >
                        <Lock className="size-4" aria-hidden="true" />
                        <span>
                            {enablingMonitoring ? 'Enabling monitoring...' : 'Enable monitoring'}
                        </span>
                    </LinearButton>
                    <LinearButton variant="ghost" size="lg" onClick={startDemo}>
                        See a demo instead
                    </LinearButton>
                </div>

                <p className="text-xs text-[var(--text-muted)]">
                    One admin authorization via Polkit to register your UID and start netwatchd.
                    Subsequent launches connect automatically.
                </p>
            </div>
        </div>
    );
};
