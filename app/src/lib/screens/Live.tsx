import React, { useState } from 'react';
import { LinearCard } from '$lib/components/linear/LinearCard';
import { Search, Filter, ArrowUpDown } from 'lucide-react';

export const Live: React.FC = () => {
  const [filter, setFilter] = useState('');

  const liveConnections = [
    {
      pid: 2481,
      app: 'Firefox',
      proto: 'TCP',
      remote: '142.250.190.46:443',
      rateDown: '1.2 MB/s',
      rateUp: '45 KB/s',
      status: 'ESTABLISHED',
    },
    {
      pid: 3120,
      app: 'Discord',
      proto: 'UDP',
      remote: '162.159.130.233:50001',
      rateDown: '24 KB/s',
      rateUp: '18 KB/s',
      status: 'ACTIVE',
    },
    {
      pid: 4892,
      app: 'Steam',
      proto: 'TCP',
      remote: '155.133.248.50:27017',
      rateDown: '8.4 MB/s',
      rateUp: '120 KB/s',
      status: 'ESTABLISHED',
    },
    {
      pid: 1420,
      app: 'starship-control',
      proto: 'UDS',
      remote: '/run/starship/nwp.sock',
      rateDown: '4 KB/s',
      rateUp: '12 KB/s',
      status: 'CONNECTED',
    },
  ];

  return (
    <div className="space-y-4">
      {/* Controls Bar */}
      <div className="flex items-center justify-between gap-4">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-2.5 size-4 text-zinc-500" />
          <input
            type="text"
            placeholder="Filter by app, IP, port, or protocol..."
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="w-full h-9 pl-9 pr-4 rounded-lg bg-[#121215] border border-[#23232a] text-xs text-zinc-200 placeholder:text-zinc-500 focus:outline-none focus:border-cyan-500"
          />
        </div>
        <div className="flex items-center gap-2 text-xs font-mono text-zinc-400">
          <span className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border border-[#23232a] bg-[#121215]">
            <Filter className="size-3.5 text-zinc-500" />
            <span>Active Only</span>
          </span>
          <span className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border border-[#23232a] bg-[#121215]">
            <ArrowUpDown className="size-3.5 text-zinc-500" />
            <span>Sort: Throughput</span>
          </span>
        </div>
      </div>

      {/* Live Table */}
      <LinearCard level={1} className="overflow-hidden">
        <table className="w-full text-left text-xs">
          <thead className="bg-[#17171c] border-b border-[#23232a] text-zinc-400 font-mono">
            <tr>
              <th className="py-2.5 px-4 font-medium">PID</th>
              <th className="py-2.5 px-4 font-medium">Application</th>
              <th className="py-2.5 px-4 font-medium">Proto</th>
              <th className="py-2.5 px-4 font-medium">Remote Endpoint</th>
              <th className="py-2.5 px-4 font-medium text-right">Down Rate</th>
              <th className="py-2.5 px-4 font-medium text-right">Up Rate</th>
              <th className="py-2.5 px-4 font-medium text-right">State</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[#23232a]">
            {liveConnections.map((conn, i) => (
              <tr key={i} className="hover:bg-[#17171c] transition-colors">
                <td className="py-3 px-4 font-mono text-zinc-500">{conn.pid}</td>
                <td className="py-3 px-4 font-medium text-zinc-200">{conn.app}</td>
                <td className="py-3 px-4 font-mono text-zinc-400">{conn.proto}</td>
                <td className="py-3 px-4 font-mono text-zinc-300">{conn.remote}</td>
                <td className="py-3 px-4 font-mono text-cyan-400 text-right">
                  {conn.rateDown}
                </td>
                <td className="py-3 px-4 font-mono text-purple-400 text-right">
                  {conn.rateUp}
                </td>
                <td className="py-3 px-4 text-right">
                  <span className="inline-flex items-center gap-1.5 font-mono text-[11px] text-emerald-400">
                    <span className="size-1.5 rounded-full bg-emerald-500" />
                    {conn.status}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </LinearCard>
    </div>
  );
};
