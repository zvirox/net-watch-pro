export interface LiveAppRow {
  cgroup_id: number;
  name: string;
  down_bps: number;
  up_bps: number;
  confidence: 'high' | 'medium' | 'low';
}

let lastRefresh = 0;
let heldOrderList: LiveAppRow[] = [];

export const liveAppsStore = {
  active: false,
  rows: [] as LiveAppRow[],
  stop() {
    this.active = false;
    this.rows = [];
    lastRefresh = 0;
    heldOrderList = [];
  },
};

export function holdOrder(newRows: LiveAppRow[], holdMs: number = 2000): LiveAppRow[] {
  const now = Date.now();
  const sorted = [...newRows].sort(
    (a, b) => (b.down_bps + b.up_bps) - (a.down_bps + a.up_bps)
  );

  if (heldOrderList.length === 0 || now - lastRefresh >= holdMs) {
    lastRefresh = now;
    heldOrderList = sorted;
    return sorted;
  }

  // Preserve ordering established in heldOrderList
  const posMap = new Map(heldOrderList.map((r, i) => [r.cgroup_id, i]));
  return [...sorted].sort((a, b) => {
    const aPos = posMap.get(a.cgroup_id);
    const bPos = posMap.get(b.cgroup_id);
    if (aPos !== undefined && bPos !== undefined) return aPos - bPos;
    if (aPos !== undefined) return -1;
    if (bPos !== undefined) return 1;
    return 0;
  });
}
