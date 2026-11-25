# DESIGN.md — Net Watch Pro

**Version:** 3.0 (Final & Locked — Linear Design System)
**Last Updated:** 2026-11-25
**Status:** **FINAL AND LOCKED.** No further visual style changes, theme overhauls, or design system experiments will be made. All future frontend work must preserve this design system and pass `npm --prefix app run check:linear` and `python3 tests/a11y/contrast.py`.

---

## 1. Visual Standard & Frozen Invariants

Net Watch Pro uses a strict **Linear Design System** engineered for dense network telemetry, rapid visual scanning, and high contrast across both Light and Dark themes on Linux desktops.

### Non-Negotiable Rules (Enforced by `app/scripts/check-linear.mjs`)

1. **Flat, Opaque Surfaces Only:** Zero glassmorphism, zero `backdrop-filter: blur(...)`, zero translucent `rgba()` / Tailwind `/10` surface fills, zero radial glows, and zero decorative borders.
2. **Shadows:** **Zero** `box-shadow` in Dark mode (`dark:shadow-none` required wherever light elevation appears). Light cards use at most `0 1px 2px rgb(16 24 40 / 5%)`.
3. **1px Hairline Borders:** Every card, input, button, table, dialog, and control panel uses crisp `1px solid var(--border)` (and `var(--border-hover)` on interaction).
4. **Gradients:** Prohibited everywhere **except** soft vertical area fills directly beneath ECharts line strokes and inline sparkline SVG paths (`linear-gradient` fade into transparent).
5. **Strict Border Radius Hierarchy:**
   - **Cards & Dialogs:** `8px` (`rounded-lg`)
   - **Buttons, Inputs, Navigation Rows & Tabs:** `6px` (`rounded-md`)
   - **Chips, Tags, Badges & Tooltips:** `4px` (`rounded`)
   - **Circular Exceptions Only:** Status indicator dots, compact avatars, and donut stroke endpoints (`rounded-full`)
6. **Typography:**
   - **UI & Numbers:** Self-hosted `InterVariable` / `InterVariable-Italic` (`app/src/lib/theme/fonts/`), `tabular-nums` (`font-variant-numeric: tabular-nums`) on all rates, byte counts, percentages, timestamps, and table numbers.
   - **Monospace (`JetBrainsMono`):** Reserved strictly for executable paths, IP:port endpoints, PIDs, license keys, event codes (`EBPF_ATTACH_FAILED`), and diagnostic copy blocks.
7. **Motion:** `150ms cubic-bezier(0.2, 0.8, 0.2, 1)` on hover, active press, and sidebar collapse. Respects `@media (prefers-reduced-motion: reduce)`.
8. **Zero External Network Calls:** Fonts (`InterVariable.woff2`, `InterVariable-Italic.woff2`, `JetBrainsMono[wght].ttf`) and icons (`lucide-react`, `/logo.png`) are 100% bundled locally.

---

## 2. Color Palette & CSS Tokens

All tokens live in `app/src/tailwind.css` (and compatibility aliases in `app/src/lib/theme/tokens.css`). Components must never hardcode hex values.

### 2.1 Dark Theme (`[data-theme="dark"]`)

| Token | Hex | Role |
|---|---|---|
| `--bg-app` | `#050505` | Deep OLED window canvas, sidebar, titlebar |
| `--bg-surface` | `#111213` | Cards, panels, tables, dialogs (Level 1) |
| `--bg-surface-2` | `#1A1B1E` | Inputs, subtle fills, active nav rows (Level 2) |
| `--bg-elevated` | `#202128` | Tooltips, active dropdowns, hover surfaces (Level 3) |
| `--border` | `#27272D` | 1px hairline card & panel borders |
| `--border-hover` | `#353740` | Hover & focused input borders |
| `--text` | `#EDEDEF` | Primary headings & values (>= 17.2:1 on `--bg-surface`) |
| `--text-muted` | `#93949D` | Labels, metadata, timestamps (>= 5.5:1 on `--bg-surface`) |

### 2.2 Light Theme (`[data-theme="light"]`)

| Token | Hex | Role |
|---|---|---|
| `--bg-app` | `#F3F4F6` | Cool off-white canvas & sidebar |
| `--bg-surface` | `#FFFFFF` | Pure crisp white cards & dialogs |
| `--bg-surface-2` | `#F4F5F7` | Inputs, nested rows, subtle fills |
| `--bg-elevated` | `#EAF0FF` | Highlighted states |
| `--border` | `#D9DCE3` | Soft hairline borders |
| `--border-hover` | `#C2C7D2` | Hover borders |
| `--text` | `#111318` | Crisp ink text (>= 17.1:1 on `--bg-app`) |
| `--text-muted` | `#5A5E6B` | Secondary labels (>= 4.8:1 on `--bg-surface`) |

### 2.3 Shared Accent & Solid Tint Tokens

Accents are shared across themes; solid `--tint-*` backgrounds with paired `--tint-*-fg` foregrounds ensure `>= 4.5:1` WCAG AA contrast without any alpha translucency:

- **Primary Action / Download:** `--accent-blue: #3355FF` (`#2B4AF0` hover, `--on-accent: #FFFFFF`)
- **Upload / Secondary Series:** `--accent-sky: #38B6F0`
- **Live / Pass / Healthy:** `--accent-green: #22C55E`
- **Warning / Degraded / Metered:** `--accent-amber: #EAB308`
- **Error / Breach / Stop:** `--accent-red: #F43F5E` (`#E5484D` window close hover)
- **Additional Categorical Donut/Series Tones:** Teal `#1FB5A5`, Pink `#EC4899`, Violet `#7C5CE6`, Orange `#F97316`

Automated verification: `python3 tests/a11y/contrast.py` verifies all 28 dark/light token pairs on every change.

---

## 3. Desktop Shell & Window Chrome (`Tauri 2`)

Configured in `app/src-tauri/tauri.conf.json` and `app/src-tauri/capabilities/default.json`:

- **Window Geometry:** `1280 x 800` default (16:10), `1024 x 640` minimum, `decorations: false`, `transparent: false`, `shadow: true`, `resizable: true`.
- **Custom Title Bar (`app/src/lib/components/linear/TitleBar.tsx`):** `40px` (`h-10`), `data-tauri-drag-region`, displays `/logo.png` icon, `"Net Watch Pro"`, dynamic `__NWP_VERSION__` chip (`v0.9.2-beta.5` from `tauri.conf.json`), `ThemeToggle`, and Minimize / Maximize-Restore / Close window buttons.
- **Theme Switching (`ThemeToggle.tsx` & `app/index.html`):**
  - Persists under `localStorage.getItem('ui-theme')`.
  - Pre-paint `<script>` in `app/index.html` applies `data-theme="dark|light"` before first paint to avoid theme flash.
  - **Only in the Title Bar:** No duplicate theme switcher exists inside Settings.
- **Collapsible Sidebar (`Sidebar.tsx`):** `240px` (`w-60`) expanded / `56px` (`w-14`) collapsed; workspace scope indicator (`Local Machine` / `Demo Sandbox` + `Linux` badge), 6 navigation items (`Dashboard`, `Live`, `History`, `Apps`, `Diagnostics`, `Settings`), bottom `License` tier button (`Free` / `Pro`), `StatusChip` (links to Diagnostics), and collapse toggle.
- **Breadcrumb Bar (`BreadcrumbBar.tsx`):** `44px` (`h-11`), clickable breadcrumb trail (`Net Watch Pro > Apps > Firefox`), plus Demo Mode badge and `Exit demo` action when active.

---

## 4. Shared Linear Components (`app/src/lib/components/linear/`)

| Component | Role |
|---|---|
| `TitleBar.tsx` | 40px custom undecorated window header & Tauri window controls |
| `ThemeToggle.tsx` | Sun/Moon toggle syncing `data-theme`, `localStorage('ui-theme')`, and `'theme-changed'` event |
| `Sidebar.tsx` | 240px/56px collapsible navigation rail |
| `BreadcrumbBar.tsx` | 44px route breadcrumb header & Demo banner controls |
| `LinearCard.tsx` | Solid 1px-bordered surface container (`level={1 \| 2}`) |
| `LinearButton.tsx` | 6px radius button (`primary`, `secondary`, `ghost`, `danger`; `sm`/`md`/`lg`) |
| `StatCard.tsx` | KPI card with top icon tile, tabular value + unit, inline SVG gradient sparkline, and solid status delta pill |
| `StatusPill.tsx` | Semantic badge (`pass`/`ok`/`live`, `warn`, `fail`, `neutral`, `info`) with 6px status dot |
| `StatusChip.tsx` | Live daemon + eBPF collector state pill in sidebar footer |
| `ProgressRow.tsx` | Label + right-aligned tabular metric over 6px horizontal fill bar |
| `RangeTabs.tsx` | Segmented period selector (`1h`, `24h`, `7d`, `30d`, `Cycle`) |
| `DonutLegend.tsx` | Dashed-divider legend paired with `AppUsageChart` |
| `DataTable.tsx` | Clean 1px-bordered table with interactive row click & hover states |
| `LinearDialog.tsx` | Solid modal container (`bg-[var(--bg-surface)]`, dark backdrop without blur) |
| `echarts-linear-theme.ts` | Shared ECharts axis, dashed gridlines, bordered dark/light tooltip with green indicator dot, and 8-color series order |
| `TrafficChart.tsx` | ECharts download/upload line (with soft area fade) and grouped bar chart |
| `AppUsageChart.tsx` | ECharts donut chart with outer subtle tick ring + stacked `DonutLegend` |

---

## 5. Screen Inventory (9 React Screens)

All screens live in `app/src/lib/screens/` and connect to `useDaemonStore`:

1. **`Onboarding.tsx`:** Shown on `auth-required` / `auth-invalid` before monitoring is enabled. Explains local-only data collection, SQLite privacy, and package cleanup; offers **Enable monitoring** (Polkit helper) and **See a demo instead**.
2. **`Dashboard.tsx`:** 4 KPI `StatCard`s (Download rate + sparkline, Upload rate + sparkline, Active connections, Total used in range), **Throughput & Usage** ECharts line chart, **App Usage Split** donut chart + legend, **Top Applications** progress list, and **Interfaces & Cycle Budgets** card.
3. **`Live.tsx`:** Real-time ECharts bandwidth chart (`net.rates`), search + protocol filter (`All`, `tcp`, `udp`, `icmp`, `other`), Pause/Resume snapshot freeze, and live socket `DataTable` (`PID`, `Application`, `Protocol`, `Remote Endpoint`, `Down Rate`, `Up Rate`, `Transferred`, `Status`).
4. **`History.tsx`:** Usage History ECharts bar chart (`TrafficChart mode="bar"`), period selector, app/host/port/protocol filters (Pro-gated advanced filters), native CSV/JSON export (`Pro`), and paginated `DataTable`.
5. **`Apps.tsx`:** Per-application accounting table with search, `Total | Speed | Name` sort control, `Received`, `Sent`, `Traffic Share` (`ProgressRow` capped `<= 100%`), `Live Speed`, inline SVG row trend, and pinned `Unattributed / system` summary footer row.
6. **`AppDetail.tsx`:** Per-app drilldown (`Overview`, `Connections`, `Endpoints` tabs), `attribution_confidence` badge (`exact`/`heuristic`), executable path in monospace, summary KPIs, timeline chart, live/historical sockets, and aggregated remote endpoints.
7. **`Diagnostics.tsx`:** Health checks (`service_unit`, `kernel`, `ebpf_attach`, `database`), DB size & retention metrics, `rejected_connections` (`SO_PEERCRED`) count & last UID, monospace event log, **Copy report**, **Refresh**, and **Restart daemon**.
8. **`Settings.tsx`:** Horizontal underline tabs (`General`, `Data`, `Budgets`, `Interfaces`, `Daemon`, `License`, `About`) for units, default range, retention window (`90`/`365` days), daily/monthly cycle budgets, interface metered/excluded toggles, systemd startup, offline Ed25519 key management, and About build info. Theme toggle intentionally omitted (TitleBar only).
9. **`Upgrade.tsx`:** Solid Linear modal (`LinearDialog`) listing Pro capabilities, `NWP1-...` license activation input, and inline error feedback.
