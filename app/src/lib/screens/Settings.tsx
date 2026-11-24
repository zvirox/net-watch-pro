import React, { useState } from 'react';
import { LinearCard } from '$lib/components/linear/LinearCard';
import { LinearButton } from '$lib/components/linear/LinearButton';
import { Server, Shield, HardDrive, Palette } from 'lucide-react';

export const Settings: React.FC = () => {
    const [retention, setRetention] = useState('90');
    const [theme, setTheme] = useState<'dark' | 'light'>('dark');

    const toggleTheme = (newTheme: 'dark' | 'light') => {
        setTheme(newTheme);
        if (newTheme === 'dark') {
            document.documentElement.classList.add('dark');
            document.documentElement.classList.remove('light');
        } else {
            document.documentElement.classList.add('light');
            document.documentElement.classList.remove('dark');
        }
    };

    return (
        <div className="space-y-6 max-w-3xl">
            {/* Visual System */}
            <LinearCard level={1} className="p-5">
                <div className="flex items-start gap-4">
                    <Palette className="size-5 text-cyan-400 mt-0.5" />
                    <div className="flex-1">
                        <h3 className="text-sm font-medium text-zinc-200">Interface Theme</h3>
                        <p className="text-xs text-zinc-500 mt-1">
                            Linear-Style Design System (Dark Elevated Surface Minimalism / Alabaster
                            Paper)
                        </p>
                        <div className="mt-4 flex gap-3">
                            <button
                                onClick={() => toggleTheme('dark')}
                                className={`px-3 py-1.5 rounded-lg text-xs font-mono border transition-all ${
                                    theme === 'dark'
                                        ? 'bg-[#17171c] border-cyan-500 text-zinc-100'
                                        : 'bg-[#121215] border-[#23232a] text-zinc-400'
                                }`}
                            >
                                Obsidian (Dark)
                            </button>
                            <button
                                onClick={() => toggleTheme('light')}
                                className={`px-3 py-1.5 rounded-lg text-xs font-mono border transition-all ${
                                    theme === 'light'
                                        ? 'bg-[#ffffff] border-cyan-500 text-zinc-900 shadow-sm'
                                        : 'bg-[#121215] border-[#23232a] text-zinc-400'
                                }`}
                            >
                                Alabaster (Light)
                            </button>
                        </div>
                    </div>
                </div>
            </LinearCard>

            {/* Retention Settings */}
            <LinearCard level={1} className="p-5">
                <div className="flex items-start gap-4">
                    <HardDrive className="size-5 text-purple-400 mt-0.5" />
                    <div className="flex-1">
                        <h3 className="text-sm font-medium text-zinc-200">
                            Data Retention & Local SQLite
                        </h3>
                        <p className="text-xs text-zinc-500 mt-1">
                            Historical traffic storage period in /var/lib/netwatch-pro/netwatch.db
                        </p>
                        <div className="mt-4 flex items-center gap-4">
                            <select
                                value={retention}
                                onChange={(e) => setRetention(e.target.value)}
                                className="h-8 px-3 rounded-lg bg-[#17171c] border border-[#23232a] text-xs font-mono text-zinc-200 focus:outline-none focus:border-cyan-500"
                            >
                                <option value="30">30 Days</option>
                                <option value="90">90 Days (Free Standard)</option>
                                <option value="180">180 Days (Pro)</option>
                                <option value="365">1 Year (Pro Max)</option>
                            </select>
                            <span className="text-xs text-zinc-500 font-mono">
                                Pruned daily at 03:00 UTC
                            </span>
                        </div>
                    </div>
                </div>
            </LinearCard>

            {/* Daemon Status & Unix Socket */}
            <LinearCard level={1} className="p-5">
                <div className="flex items-start gap-4">
                    <Server className="size-5 text-emerald-400 mt-0.5" />
                    <div className="flex-1">
                        <h3 className="text-sm font-medium text-zinc-200">Daemon Architecture</h3>
                        <p className="text-xs text-zinc-500 mt-1">
                            Unprivileged Tauri 2 UI talking to root netwatchd over local Unix Domain
                            Socket
                        </p>
                        <div className="mt-4 p-3 rounded-lg bg-[#17171c] border border-[#23232a] font-mono text-xs text-zinc-400 space-y-1">
                            <div>
                                Socket Path:{' '}
                                <span className="text-zinc-200">/run/netwatch-pro/api.sock</span>
                            </div>
                            <div>
                                Auth Mechanism:{' '}
                                <span className="text-emerald-400">
                                    SO_PEERCRED (UID 1000 allowed)
                                </span>
                            </div>
                            <div>
                                Observer Core:{' '}
                                <span className="text-cyan-400">Aya eBPF (CO-RE / BTF)</span>
                            </div>
                        </div>
                    </div>
                </div>
            </LinearCard>
        </div>
    );
};
