import React, { useState } from 'react';
import { Check } from 'lucide-react';
import { LinearButton, LinearDialog } from '$lib/components/linear';
import { useDaemonStore } from '$lib/stores/useDaemonStore';

export const Upgrade: React.FC = () => {
    const { upgradeOpen, isPro, closeUpgrade, activateLicenseKey, deactivateLicenseKey } =
        useDaemonStore();

    const [keyInput, setKeyInput] = useState('');
    const [error, setError] = useState<string | null>(null);
    const [busy, setBusy] = useState(false);

    const handleActivate = async (e: React.FormEvent) => {
        e.preventDefault();
        setBusy(true);
        setError(null);
        const err = await activateLicenseKey(keyInput);
        setBusy(false);
        if (err) {
            setError(err);
        } else {
            setKeyInput('');
        }
    };

    const handleDeactivate = async () => {
        setBusy(true);
        setError(null);
        const err = await deactivateLicenseKey();
        setBusy(false);
        if (err) setError(err);
    };

    return (
        <LinearDialog
            open={upgradeOpen}
            onClose={closeUpgrade}
            title="Pro License"
            subtitle="Net Watch Pro"
            maxWidthClass="max-w-md"
        >
            {isPro ? (
                <div className="space-y-4">
                    <div className="p-4 rounded-lg border border-[var(--border)] bg-[var(--bg-surface-2)] space-y-1">
                        <p className="text-sm font-semibold text-[var(--text)]">
                            Pro license active
                        </p>
                        <p className="text-xs text-[var(--text-muted)]">
                            Advanced history filters, CSV/JSON export, and 365-day retention are
                            unlocked on this device.
                        </p>
                    </div>
                    {error && (
                        <p className="text-xs text-[var(--tint-red-fg)]" role="alert">
                            {error}
                        </p>
                    )}
                    <div className="flex justify-end">
                        <LinearButton
                            variant="secondary"
                            size="sm"
                            disabled={busy}
                            onClick={() => void handleDeactivate()}
                        >
                            Deactivate license
                        </LinearButton>
                    </div>
                </div>
            ) : (
                <form onSubmit={(e) => void handleActivate(e)} className="space-y-4">
                    <ul className="space-y-2 text-xs text-[var(--text)]">
                        {[
                            'Multi-filter connection history search',
                            'CSV and JSON export via native save dialog',
                            'Extended local SQLite retention up to 365 days',
                        ].map((item) => (
                            <li key={item} className="flex items-center gap-2">
                                <span className="size-4 rounded bg-[var(--tint-green)] text-[var(--tint-green-fg)] flex items-center justify-center shrink-0">
                                    <Check className="size-3" aria-hidden="true" />
                                </span>
                                <span>{item}</span>
                            </li>
                        ))}
                    </ul>

                    <label className="block space-y-1.5 text-xs">
                        <span className="font-medium text-[var(--text)]">Offline license key</span>
                        <input
                            type="text"
                            value={keyInput}
                            onChange={(e) => {
                                setKeyInput(e.target.value);
                                setError(null);
                            }}
                            placeholder="NWP1-..."
                            autoComplete="off"
                            spellCheck={false}
                            className="w-full h-9 px-3 rounded-md bg-[var(--bg-surface-2)] border border-[var(--border-strong)] text-xs text-[var(--text)] placeholder:text-[var(--text-faint)] focus:outline-none focus:border-[var(--accent-blue)]"
                        />
                        <span className="block text-[var(--text-muted)]">
                            Verified locally against the embedded Ed25519 public key.
                        </span>
                    </label>

                    {error && (
                        <p className="text-xs text-[var(--tint-red-fg)]" role="alert">
                            {error}
                        </p>
                    )}

                    <div className="pt-3 border-t border-[var(--border)] flex items-center justify-end gap-2">
                        <LinearButton variant="ghost" size="sm" onClick={closeUpgrade}>
                            Cancel
                        </LinearButton>
                        <LinearButton
                            type="submit"
                            variant="primary"
                            size="sm"
                            disabled={busy || keyInput.trim().length === 0}
                        >
                            {busy ? 'Activating...' : 'Activate'}
                        </LinearButton>
                    </div>
                </form>
            )}
        </LinearDialog>
    );
};
