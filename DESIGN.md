# DESIGN.md — Net Watch Pro

**Version:** 3.1 (Final & Locked — Linear Design System + Unified Family Color Palette)
**Last Updated:** 2026-11-26
**Status:** **FINAL AND LOCKED.** Shared charcoal/light Linear surfaces with **Royal blue** (`--primary`) accent, zero green/teal hues, and `--shadow-card` card depth. All frontend work must pass `npm --prefix app run check:linear` and `python3 tests/a11y/contrast.py`.

---

## 1. Visual Standard & Frozen Invariants

Net Watch Pro uses the locked **Linear Design System** shared across the Zvirox desktop suite: charcoal dark surfaces (`#1C1C1C` / `#292929`), crisp light surfaces (`#F8F9FA` / `#FFFFFF`), 1px hairline borders, and **Royal blue** as NWP's primary key color.

### Non-Negotiable Rules (Enforced by `app/scripts/check-linear.mjs`)

1. **Flat Linear Surfaces & Allowed Translucency:** Zero glassmorphism, zero `backdrop-filter: blur(...)`, zero translucent surface fills, zero glows, zero inset highlights. Translucency is permitted **only** inside `--scrim`, `--chart-fade-up`, `--chart-fade-down`, and `--shadow-card` in `app/src/styles/linear-theme.css`.
2. **Shadows:** The **only** permitted `box-shadow` is `var(--shadow-card)` (`0 12px 28px -6px rgba(0,0,0,0.5)` dark, `0 1px 2px rgb(16 24 40 / 5%)` light).
3. **1px Hairline Borders:** Every card, input, button, table, dialog, and control panel uses crisp `1px solid var(--border)` (and `var(--border-hover)` / `var(--border-strong)` where emphasized).
4. **No Green or Teal:** Zero green hues (`75°–170°`) and zero teal/emerald/lime classes anywhere in source. Positive, healthy, online, and upward deltas use `--sky` (`#2FADFF` / `#0284C7`), `--sky-text`, and `--chip-sky`.
5. **Colors Only in Token Files:** All hex, `rgb()`, `hsl()`, and Tailwind palette classes live strictly in `app/src/styles/linear-theme.css` (byte-identical shared palette) and `app/src/styles/app-accent.css` (Royal blue primary). Foreign sibling app primary hexes and retired hexes are banned.
6. **Strict Border Radius Hierarchy:**
   - **Cards & Dialogs:** `8px` (`rounded-lg`)
   - **Buttons, Inputs, Navigation Rows & Tabs:** `6px` (`rounded-md`)
   - **Chips, Tags, Badges & Tooltips:** `4px` (`rounded`)
   - **Circular Exceptions Only:** Status indicator dots, compact avatars, and donut stroke endpoints (`rounded-full`)
7. **Typography:**
   - **UI & Numbers:** Self-hosted `InterVariable` / `InterVariable-Italic` (`app/src/lib/theme/fonts/`), `tabular-nums` on all rates, byte counts, percentages, timestamps, and table numbers.
   - **Monospace (`JetBrainsMono`):** Reserved strictly for executable paths, IP:port endpoints, PIDs, license keys, event codes (`EBPF_ATTACH_FAILED`), and diagnostic copy blocks.
8. **Motion:** `150ms cubic-bezier(0.2, 0, 0, 1)` on hover, active press, and sidebar collapse. Respects `@media (prefers-reduced-motion: reduce)`.
9. **Zero External Network Calls:** Fonts and icons (`lucide-react`, `/logo.png`) are 100% bundled locally.

---

## 2. Color Palette & CSS Tokens

All tokens live in `app/src/styles/linear-theme.css` and `app/src/styles/app-accent.css`, mapped into Tailwind via `@theme inline` in `app/src/tailwind.css`.

### 2.1 Shared Tokens (`app/src/styles/linear-theme.css`)

| Token | Dark | Light | Role |
|---|---|---|---|
| `--bg-app` | `#1C1C1C` | `#F8F9FA` | Window, title bar, breadcrumb bar, main canvas |
| `--bg-sidebar` | `#1C1C1C` | `#F3F4F6` | Sidebar and left navigation rail |
| `--bg-surface` | `#1C1C1C` | `#FFFFFF` | Cards, panels, dialogs |
| `--bg-surface-2` | `#292929` | `#F3F4F6` | Inputs, version tag, status pill fill, tooltip body |
| `--bg-active` | `#292929` | `#E5E7EB` | Active nav item, selected range tab |
| `--bg-hover` | `#232323` | `#EEF0F2` | Hovered nav item, ghost/secondary hover, title bar button hover |
| `--bg-track` | `#333333` | `#E5E7EB` | Progress track, donut tick ring |
| `--bg-bar-idle` | `#333333` | `#E5E7EB` | Unselected chart bars |
| `--border` | `#2E2E2E` | `#E5E7EB` | 1px hairlines, dividers |
| `--border-strong` | `#363636` | `#D1D5DB` | Card hover, dialog/tooltip/selected tab borders |
| `--chart-grid` | `#2E2E2E` | `#D1D5DB` | Dotted chart gridlines |
| `--text` | `#FFFFFF` | `#111827` | Headings, KPI numbers, active labels |
| `--text-secondary` | `#CBCBCB` | `#111827` | Body text, labels, table rows, nav items |
| `--text-muted` | `#B4B4B4` | `#5B6370` | Units, timestamps, table headers, unselected tabs |
| `--text-faint` | `#939393` | `#818998` | Hints, idle status dot |
| `--blue` | `#346AFE` | `#2563EB` | Royal blue series color |
| `--sky` | `#2FADFF` | `#0284C7` | Positive / healthy / online / done / up sparkline & dot |
| `--sky-text` | `#2FADFF` | `#0369A1` | Positive delta text, upload table text |
| `--amber` | `#F6940F` | `#D97706` | Warning dot/icon/border, series 3 |
| `--pink` | `#FC669B` | `#DB2777` | Series 4 color |
| `--purple` | `#6D59F9` | `#6D28D9` | Shared palette purple (skipped in NWP series list due to blue proximity) |
| `--orange` | `#FB6801` | `#EA580C` | Upload series line/bar stroke & fade |
| `--red` | `#FC4232` | `#DC2626` | Error, critical, down sparkline, negative border |
| `--red-text` | `#FF6B5E` | `#B91C1C` | Negative delta text, error message |
| `--gray` | `#4F4F4F` | `#868E9C` | Neutral / unattributed series |
| `--chip-sky` | `#1F303C` | `#E0F2FE` | Positive delta chip, Pro benefit check chip, INFO badge |
| `--chip-red` | `#2D1F1F` | `#FEE2E2` | Negative delta chip, error banner/chip |
| `--close-hover` | `#E5484D` | `#E5484D` | Window close button hover fill |
| `--scrim` | `rgba(0,0,0,0.7)` | `rgba(0,0,0,0.4)` | Solid dim dialog backdrop |
| `--shadow-card` | `0 12px 28px -6px rgba(0,0,0,0.5)` | `0 1px 2px rgb(16 24 40 / 5%)` | Allowed card/dialog shadow |
| `--chart-fade-up` | `rgba(47,173,255,0.15)` | `rgba(2,132,199,0.1)` | Gradient fade under up sparklines |
| `--chart-fade-down` | `rgba(252,66,50,0.15)` | `rgba(220,38,38,0.1)` | Gradient fade under down sparklines |

### 2.2 Net Watch Pro Primary Accent: Royal Blue (`app/src/styles/app-accent.css`)

| Role | Token | Dark | Light |
|---|---|---|---|
| Primary button / switch / progress fill | `--primary` | `#346AFE` | `#2563EB` |
| Primary hover fill | `--primary-hover` | `#2E5DE0` | `#2157CF` |
| Primary button text | `--primary-fg` | `#FFFFFF` | `#FFFFFF` |
| Links, 2px focus ring, active underline & nav icon | `--primary-text` | `#6B9BFF` | `#2563EB` |
| Assigned / info / license chip fill | `--primary-tint` | `#1F273C` | `#F0F4FE` |

### 2.3 Chart Palette (`echarts-linear-theme.ts`)

- Reads all theme values dynamically via `getComputedStyle(document.documentElement)` and re-renders cleanly on `theme-changed` and `data-theme` attribute mutations.
- **Series Order:** `['--primary', '--sky', '--amber', '--pink']` (plus `'--gray'` for unattributed/other).
- **Live & History Charts (`TrafficChart.tsx`):** Download = `--primary`, Upload = `--orange`, idle History bars = `--bg-bar-idle`, tooltip indicator dot = `--sky`.

Automated verification: `python3 tests/a11y/contrast.py` validates all 23 WCAG 2.2 contrast pairs across Dark and Light themes.

---

## 3. Desktop Shell & Window Chrome (`Tauri 2`)

Configured in `app/src-tauri/tauri.conf.json` and `app/src-tauri/capabilities/default.json`:

- **Window Geometry:** `1280 x 800` default (16:10), `1024 x 640` minimum, `decorations: false`, `transparent: false`, `shadow: true`, `resizable: true`.
- **Custom Title Bar (`app/src/lib/components/linear/TitleBar.tsx`):** `40px` (`h-10`), `data-tauri-drag-region`, displays `/logo.png` icon, `"Net Watch Pro"`, dynamic `__NWP_VERSION__` chip (`v0.9.2-beta.6`), `ThemeToggle`, and Minimize / Maximize-Restore / Close window buttons.
- **Theme Switching (`ThemeToggle.tsx` & `app/index.html`):** Persists under `localStorage.getItem('ui-theme')`, pre-painted before hydration, placed strictly in the Title Bar.
- **Collapsible Sidebar (`Sidebar.tsx`):** `240px` (`w-60`) expanded / `56px` (`w-14`) collapsed on `--bg-sidebar` with workspace indicator, 6 nav items, License button, `StatusChip`, and collapse toggle.
- **Breadcrumb Bar (`BreadcrumbBar.tsx`):** `40px` (`h-10`), clickable route trail + Demo Mode indicator and exit button.

---

## 4. Shared Linear Components (`app/src/lib/components/linear/`)

| Component | Role |
|---|---|
| `TitleBar.tsx` | 40px custom undecorated window header & Tauri window controls |
| `ThemeToggle.tsx` | Sun/Moon toggle syncing `data-theme`, `localStorage('ui-theme')`, and `'theme-changed'` |
| `Sidebar.tsx` | 240px/56px collapsible navigation rail on `--bg-sidebar` |
| `BreadcrumbBar.tsx` | Route breadcrumb header & Demo banner controls |
| `LinearCard.tsx` | Solid 1px-bordered surface container with `shadow-[var(--shadow-card)]` |
| `LinearButton.tsx` | 6px radius button (`primary`, `secondary`, `ghost`, `danger`; `sm`/`md`/`lg`) |
| `StatCard.tsx` | KPI card with tone icon badge, tabular value, SVG sparkline (`--sky`/`--red`), and delta chip |
| `StatusPill.tsx` | Semantic badge (`online`, `active`, `warning`, `alert`, `idle`) |
| `StatusChip.tsx` | Live daemon + eBPF collector state pill in sidebar footer |
| `ProgressRow.tsx` | Label + right-aligned tabular metric over 4px `--bg-track` bar |
| `RangeTabs.tsx` | Segmented period selector (`1h`, `24h`, `7d`, `30d`, `Cycle`) |
| `DonutLegend.tsx` | Dashed-divider legend paired with `AppUsageChart` |
| `DataTable.tsx` | 1px-bordered table with interactive row hover (`--bg-hover`) & error card |
| `LinearDialog.tsx` | Modal container over `--scrim` solid backdrop |
| `echarts-linear-theme.ts` | Runtime CSS-variable ECharts config, `--chart-grid` dotted grid, `--bg-surface-2` tooltip |
| `TrafficChart.tsx` | ECharts download (`--primary`) / upload (`--orange`) line and bar chart |
| `AppUsageChart.tsx` | ECharts donut chart with `--bg-track` tick ring + stacked `DonutLegend` |

---

## 5. Screen Inventory (9 React Screens)

1. **`Onboarding.tsx`:** Privacy/storage/removal cards (`--primary-tint`, `--chip-sky`, `--bg-surface-2`), Polkit **Enable monitoring**, and **See a demo instead**.
2. **`Dashboard.tsx`:** 4 KPI `StatCard`s, **Throughput & Usage** line chart (`--primary` / `--orange`), **App Usage Split** donut, **Top Applications** list, **Interfaces & Billing Cycle Budgets** (`--sky` / `--amber` / `--red`).
3. **`Live.tsx`:** Real-time throughput chart (`blue` download, `orange` upload), protocol/search filters, Pause toggle, live socket `DataTable`.
4. **`History.tsx`:** Usage History bar chart (`--primary` / `--orange` / `--bg-bar-idle`), period selector, Free/Pro filters, CSV/JSON export, paginated `DataTable`.
5. **`Apps.tsx`:** Per-app usage ranking, `ProgressRow` share (`<= 100%`), `--sky` row sparklines, `Unattributed / system` summary row.
6. **`AppDetail.tsx`:** Overview/Connections/Endpoints tabs with `--primary-text` active underline.
7. **`Diagnostics.tsx`:** Health checks (`--sky` / `--amber` / `--red`), storage/UID metrics, monospace event journal (`INFO` `--chip-sky`, `WARN` `--amber`, `ERROR` `--chip-red`, code `--primary-text`).
8. **`Settings.tsx`:** 7 tabs (`General`, `Data`, `Budgets`, `Interfaces`, `Daemon`, `License`, `About`).
9. **`Upgrade.tsx`:** Solid Linear modal (`LinearDialog`) with `--chip-sky` checkmarks and offline `NWP1-` key entry.
