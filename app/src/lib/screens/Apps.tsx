import React from 'react';
import { LinearCard } from '$lib/components/linear/LinearCard';
import { Package, ArrowUpRight } from 'lucide-react';

export const Apps: React.FC = () => {
    const apps = [
        {
            id: 1,
            name: 'Firefox',
            path: '/usr/lib/firefox/firefox',
            sockets: 14,
            totalDown: '12.4 GB',
            totalUp: '840 MB',
        },
        {
            id: 2,
            name: 'Discord',
            path: '/opt/discord/Discord',
            sockets: 6,
            totalDown: '3.1 GB',
            totalUp: '420 MB',
        },
        {
            id: 3,
            name: 'Steam',
            path: '/usr/bin/steam',
            sockets: 8,
            totalDown: '18.9 GB',
            totalUp: '120 MB',
        },
        {
            id: 4,
            name: 'Spotify',
            path: '/usr/share/spotify/spotify',
            sockets: 2,
            totalDown: '820 MB',
            totalUp: '14 MB',
        },
        {
            id: 5,
            name: 'systemd-resolved',
            path: '/usr/lib/systemd/systemd-resolved',
            sockets: 4,
            totalDown: '45 MB',
            totalUp: '45 MB',
        },
    ];

    return (
        <div className="space-y-4">
            <LinearCard level={1} className="overflow-hidden">
                <table className="w-full text-left text-xs">
                    <thead className="bg-[#17171c] border-b border-[#23232a] text-zinc-400 font-mono">
                        <tr>
                            <th className="py-2.5 px-4 font-medium">Application</th>
                            <th className="py-2.5 px-4 font-medium">Binary / Command</th>
                            <th className="py-2.5 px-4 font-medium text-center">Open Sockets</th>
                            <th className="py-2.5 px-4 font-medium text-right">Total Download</th>
                            <th className="py-2.5 px-4 font-medium text-right">Total Upload</th>
                            <th className="py-2.5 px-4 font-medium text-right">Action</th>
                        </tr>
                    </thead>
                    <tbody className="divide-y divide-[#23232a]">
                        {apps.map((app) => (
                            <tr key={app.id} className="hover:bg-[#17171c] transition-colors">
                                <td className="py-3 px-4 font-medium text-zinc-200">
                                    <div className="flex items-center gap-2">
                                        <Package className="size-4 text-zinc-400" />
                                        <span>{app.name}</span>
                                    </div>
                                </td>
                                <td className="py-3 px-4 font-mono text-zinc-500 truncate max-w-xs">
                                    {app.path}
                                </td>
                                <td className="py-3 px-4 font-mono text-zinc-400 text-center">
                                    {app.sockets}
                                </td>
                                <td className="py-3 px-4 font-mono text-cyan-400 text-right">
                                    {app.totalDown}
                                </td>
                                <td className="py-3 px-4 font-mono text-purple-400 text-right">
                                    {app.totalUp}
                                </td>
                                <td className="py-3 px-4 text-right">
                                    <button className="text-zinc-400 hover:text-zinc-100 font-medium inline-flex items-center gap-1">
                                        Details <ArrowUpRight className="size-3" />
                                    </button>
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            </LinearCard>
        </div>
    );
};
