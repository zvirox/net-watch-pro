import React from 'react';
import { LinearCard } from '$lib/components/linear/LinearCard';
import { TrafficChart } from '$lib/components/TrafficChart';
import { ArrowDown, ArrowUp, Activity, ShieldCheck, Cpu } from 'lucide-react';
import { formatBytes } from '$lib/utils/format';

export const Dashboard: React.FC = () => {
    return (
        <div className="space-y-6">
            {/* Metric Cards Grid */}
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
                <LinearCard level={1} className="p-4">
                    <div className="flex items-center justify-between text-zinc-400">
                        <span className="text-xs font-mono uppercase tracking-wider">
                            Download Today
                        </span>
                        <ArrowDown className="size-4 text-cyan-400" />
                    </div>
                    <div className="mt-2 text-2xl font-sans font-semibold tracking-tight text-zinc-100">
                        {formatBytes(1080000000)}
                    </div>
                    <div className="mt-1 text-xs text-zinc-500 font-mono">
                        Rate: <span className="text-zinc-300">2.4 MB/s</span>
                    </div>
                </LinearCard>

                <LinearCard level={1} className="p-4">
                    <div className="flex items-center justify-between text-zinc-400">
                        <span className="text-xs font-mono uppercase tracking-wider">
                            Upload Today
                        </span>
                        <ArrowUp className="size-4 text-purple-400" />
                    </div>
                    <div className="mt-2 text-2xl font-sans font-semibold tracking-tight text-zinc-100">
                        {formatBytes(160000000)}
                    </div>
                    <div className="mt-1 text-xs text-zinc-500 font-mono">
                        Rate: <span className="text-zinc-300">420 KB/s</span>
                    </div>
                </LinearCard>

                <LinearCard level={1} className="p-4">
                    <div className="flex items-center justify-between text-zinc-400">
                        <span className="text-xs font-mono uppercase tracking-wider">
                            Active Sockets
                        </span>
                        <Activity className="size-4 text-emerald-400" />
                    </div>
                    <div className="mt-2 text-2xl font-sans font-semibold tracking-tight text-zinc-100">
                        38
                    </div>
                    <div className="mt-1 text-xs text-zinc-500 font-mono">
                        Across <span className="text-zinc-300">12 apps</span>
                    </div>
                </LinearCard>

                <LinearCard level={1} className="p-4">
                    <div className="flex items-center justify-between text-zinc-400">
                        <span className="text-xs font-mono uppercase tracking-wider">
                            Attribution
                        </span>
                        <ShieldCheck className="size-4 text-cyan-400" />
                    </div>
                    <div className="mt-2 text-2xl font-sans font-semibold tracking-tight text-zinc-100">
                        96.8%
                    </div>
                    <div className="mt-1 text-xs text-zinc-500 font-mono">
                        Kernel vs cgroup match
                    </div>
                </LinearCard>
            </div>

            {/* Main Traffic Telemetry Chart */}
            <LinearCard level={1} className="p-5">
                <div className="flex items-center justify-between mb-4">
                    <div>
                        <h3 className="text-sm font-sans font-medium text-zinc-200">
                            Throughput History
                        </h3>
                        <p className="text-xs text-zinc-500">
                            Real-time per-second socket traffic observed via Aya eBPF
                        </p>
                    </div>
                    <div className="flex items-center gap-4 text-xs font-mono text-zinc-400">
                        <span className="flex items-center gap-1.5">
                            <span className="size-2 rounded-full bg-cyan-400" /> Download
                        </span>
                        <span className="flex items-center gap-1.5">
                            <span className="size-2 rounded-full bg-purple-400" /> Upload
                        </span>
                    </div>
                </div>
                <TrafficChart />
            </LinearCard>

            {/* Top Talkers Table */}
            <LinearCard level={1} className="overflow-hidden">
                <div className="p-4 border-b border-[#23232a] flex items-center justify-between">
                    <h3 className="text-sm font-sans font-medium text-zinc-200">
                        Top Applications
                    </h3>
                    <span className="text-xs font-mono text-zinc-500">24-hour window</span>
                </div>
                <div className="divide-y divide-[#23232a] text-xs">
                    {[
                        { name: 'Firefox', down: '4.2 GB', up: '240 MB', pct: '62%' },
                        { name: 'Discord', down: '1.1 GB', up: '180 MB', pct: '18%' },
                        { name: 'Steam', down: '850 MB', up: '45 MB', pct: '12%' },
                        { name: 'Spotify', down: '420 MB', up: '12 MB', pct: '6%' },
                        { name: 'System / Unattributed', down: '120 MB', up: '8 MB', pct: '2%' },
                    ].map((app, i) => (
                        <div
                            key={i}
                            className="p-3.5 flex items-center justify-between hover:bg-[#17171c] transition-colors"
                        >
                            <div className="flex items-center gap-3">
                                <span className="font-mono text-zinc-500 w-4">{i + 1}</span>
                                <span className="font-medium text-zinc-200">{app.name}</span>
                            </div>
                            <div className="flex items-center gap-6 font-mono">
                                <span className="text-cyan-400">↓ {app.down}</span>
                                <span className="text-purple-400">↑ {app.up}</span>
                                <span className="text-zinc-500 w-12 text-right">{app.pct}</span>
                            </div>
                        </div>
                    ))}
                </div>
            </LinearCard>
        </div>
    );
};
