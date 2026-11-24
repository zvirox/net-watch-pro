import React, { useState } from 'react';
import { LinearCard } from '$lib/components/linear/LinearCard';
import { Calendar, Download } from 'lucide-react';
import { LinearButton } from '$lib/components/linear/LinearButton';

export const History: React.FC = () => {
  const [range, setRange] = useState('7d');

  const historyEntries = [
    { time: '2026-11-24 10:15', app: 'Firefox', remote: 'api.github.com:443', bytes: '14.2 MB', duration: '12s' },
    { time: '2026-11-24 10:12', app: 'Discord', remote: 'gateway.discord.gg:443', bytes: '8.4 MB', duration: '4m' },
    { time: '2026-11-24 09:45', app: 'Steam', remote: 'content1.steampowered.com:80', bytes: '1.4 GB', duration: '2m' },
    { time: '2026-11-24 09:30', app: 'Spotify', remote: 'audio-ak.spotify.com:443', bytes: '42 MB', duration: '15m' },
  ];

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          {['24h', '7d', '30d', '90d'].map((r) => (
            <button
              key={r}
              onClick={() => setRange(r)}
              className={`px-3 py-1 rounded-md text-xs font-mono transition-colors ${
                range === r
                  ? 'bg-zinc-100 text-zinc-950 font-medium'
                  : 'bg-[#121215] text-zinc-400 border border-[#23232a] hover:text-zinc-200'
              }`}
            >
              {r}
            </button>
          ))}
        </div>
        <LinearButton variant="secondary" size="sm">
          <Download className="size-3.5" />
          <span>Export CSV</span>
        </LinearButton>
      </div>

      <LinearCard level={1} className="overflow-hidden">
        <table className="w-full text-left text-xs">
          <thead className="bg-[#17171c] border-b border-[#23232a] text-zinc-400 font-mono">
            <tr>
              <th className="py-2.5 px-4 font-medium">Timestamp</th>
              <th className="py-2.5 px-4 font-medium">Application</th>
              <th className="py-2.5 px-4 font-medium">Remote Endpoint</th>
              <th className="py-2.5 px-4 font-medium text-right">Data Volume</th>
              <th className="py-2.5 px-4 font-medium text-right">Duration</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[#23232a]">
            {historyEntries.map((entry, i) => (
              <tr key={i} className="hover:bg-[#17171c] transition-colors">
                <td className="py-3 px-4 font-mono text-zinc-400">{entry.time}</td>
                <td className="py-3 px-4 font-medium text-zinc-200">{entry.app}</td>
                <td className="py-3 px-4 font-mono text-zinc-300">{entry.remote}</td>
                <td className="py-3 px-4 font-mono text-cyan-400 text-right">{entry.bytes}</td>
                <td className="py-3 px-4 font-mono text-zinc-500 text-right">{entry.duration}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </LinearCard>
    </div>
  );
};
