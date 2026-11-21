# Net Watch Pro — DESIGN.md (UI Design System)

**Version:** 1.6 · **Date:** 2026-11-21
**Source of truth:** `PRD.md` (scope, screens, flows) · `REAPM.md` (reference teardown) · `docs/decisions.md` (owner decisions D1–D7)

**Version history:** 1.1 (2026-11-15): Dashboard §8.3 gains an "Unattributed / system" row + explanatory tooltip (F-03/D3a); settings §8.7 notes timezone + billing-cycle settings (F-04/D4a); App detail confidence badge stays per F4 (F-05/D5a). 1.2 (2026-11-19): add the "Network now" live-rate card with 60-second sparkline, Apps live-speed columns, `speed_unit` selector, and stale/no-collector states (live throughput plan, Phase A). 1.3 (2026-11-19): interfaces card shows Wi-Fi signal/link from local sysfs with "—" for missing values; last-hour rate history is labelled in-memory. 1.4 (2026-11-20): Phase D — range selector (§7.13), range-driven Dashboard totals and bar timeline plus billing-cycle card (§8.3), and the new Apps list screen (§8.10). 1.5 (2026-11-20): Phase 9 contrast fix — `border-strong`, light `text-disabled`, light chart series 3/4/muted and the light axis-label were recalculated to satisfy §9.1 (owner-approved; measured ratios recorded there). No new tokens. 1.6 (2026-11-21): Live/History connection rows are fed by the per-socket collector; `pid = 0` renders as "—" (unknown) and the empty state is reserved for genuinely idle periods.
**Stack context:** Tauri 2 shell · Svelte 5 · shadcn-svelte components · ECharts · localhost WS data

> **STRICT RULES — every UI built later must follow this file.**
> 1. No colors outside §3. No fonts outside §4. No new primitives outside §7 without updating this file first.
> 2. All spacing, radius, shadows, motion and breakpoints come from the scales in §5–§6 — never hardcode other values.
> 3. Every interactive component implements the states defined in §7 (or is explicitly marked N/A there).
> 4. Screens are composed only from the layouts in §8 and components in §7.
> 5. Accessibility rules in §9 are blockers, not suggestions: a build that fails them does not ship.
> 6. Deviations require a design review and an update to this file (version bump), never a one-off exception in code.

---

## 1. Brand Feel

**Net Watch Pro feels like a precision instrument: calm, exact, trustworthy.**

The product watches the user's network — a space where people feel watched *back*. The UI must feel like a clean lab readout, not a dashboard-metrics toy and not a security-scare booth: quiet dark and light surfaces, monospaced numerals that read like a terminal of truth, one confident accent color that means "the system is watching, so you don't have to." Data is the hero; chrome disappears. The single emotional promise of the brand — *everything local, nothing hidden* — is expressed through visible system status (a persistent, honest daemon state chip) and through restraint everywhere else.

### 3 Design Principles

1. **Glass over calm (Calm Surface, Glass Overlay)** — The base UI is a modern-minimal, flat, matte system: solid surfaces, hairline borders, no decorative gradients. Only *transient, floating* layers (modals, toasts, dialogs over content) use a glass treatment: translucent surface with backdrop blur, so the live data stays perceptible underneath without ever competing with it. The app never shimmer-distorts; the glass is subtle (low blur, low alpha) and reserved.
2. **Numbers are the interface** — Every quantitative element (bytes, rates, IPs, ports, timestamps) is rendered in the mono numerals with tabular alignment so columns never jitter and values are comparable at a glance. Charts are the *summary* of the numbers, never a replacement; every chart has a data-table equivalent.
3. **Honest status, always visible** — The system state (daemon running / degraded / stopped, eBPF attached, recording) is permanently surfaced as a status chip in the top bar and mirrored in the tray. No state is ever communicated by color alone, and no degraded mode is ever silently hidden. If data would be wrong or stale, the UI says so before showing it.

---

## 2. Theme Model

Two full themes: **Light** (default on first run, follows system option) and **Dark** (default for dark-mode systems). Every token exists in both themes under the same name. Components reference tokens only — never raw hex.

Semantic layers: `background` (window) → `surface` (cards/panels) → `surface-2` (nested fills, hover, table hover) → `border`/`border-strong`.

---

## 3. Color Palette

### 3.1 Light theme

| Token | Hex | Usage |
|---|---|---|
| `background` | `#F6F8FB` | Window / app background |
| `surface` | `#FFFFFF` | Cards, panels, tables, inputs |
| `surface-2` | `#EEF2F7` | Muted fills, row hover, nested blocks |
| `border` | `#DCE3ED` | Hairline borders, dividers |
| `border-strong` | `#7B8794` | Input borders, emphasized dividers (3.66:1 vs surface — §9.1) |
| `text` | `#101828` | Primary text |
| `text-secondary` | `#475467` | Secondary text, labels |
| `text-disabled` | `#7B8794` | Disabled text, placeholders (3.66:1 vs surface — §9.1) |
| `primary` | `#0E7490` | Actions, active nav, links, primary chart series |
| `primary-hover` | `#155E75` | Hover on primary fills |
| `primary-active` | `#164E63` | Pressed on primary fills |
| `on-primary` | `#FFFFFF` | Text/icons on primary fills |
| `primary-soft` | `#E3F4F9` | Selected rows, soft tints, focus tint |
| `secondary` | `#4338CA` | Pro/brand accent, secondary chart series |
| `secondary-hover` | `#3730A3` | Hover on secondary fills |
| `secondary-soft` | `#E0E7FF` | Pro tints, soft badges |
| `on-secondary` | `#FFFFFF` | Text/icons on secondary fills |
| `success` | `#067647` | Success text/icons, healthy status |
| `success-soft` | `#ECFDF3` | Success tint fills |
| `warning` | `#B54708` | Warning text/icons, degraded status |
| `warning-soft` | `#FFFAEB` | Warning tint fills |
| `error` | `#B42318` | Error text/icons, destructive actions |
| `error-hover` | `#912018` | Hover on destructive fills |
| `error-soft` | `#FEF3F2` | Error tint fills |
| `on-status` | `#FFFFFF` | Text on solid success/warning/error fills |
| `overlay` | `rgba(16,24,40,0.50)` | Modal overlay |
| `glass-surface` | `rgba(255,255,255,0.86)` | Glass layers (modals/toasts) |
| `glass-border` | `rgba(220,227,237,0.90)` | Glass layer border |

### 3.2 Dark theme

| Token | Hex | Usage |
|---|---|---|
| `background` | `#0C111D` | Window / app background |
| `surface` | `#161B26` | Cards, panels, tables, inputs |
| `surface-2` | `#212A3B` | Muted fills, row hover, nested blocks |
| `border` | `#2C3442` | Hairline borders, dividers |
| `border-strong` | `#667085` | Input borders, emphasized dividers (3.46:1 vs surface — §9.1) |
| `text` | `#F0F2F5` | Primary text |
| `text-secondary` | `#9AA4B2` | Secondary text, labels |
| `text-disabled` | `#5D6B7E` | Disabled text, placeholders |
| `primary` | `#22D3EE` | Actions, active nav, links, primary chart series |
| `primary-hover` | `#67E8F9` | Hover on primary fills |
| `primary-active` | `#06B6D4` | Pressed on primary fills |
| `on-primary` | `#0C111D` | Text/icons on primary fills (dark fills use light text: `#FFFFFF` on `#0E7490`) |
| `primary-soft` | `#123A46` | Selected rows, soft tints, focus tint |
| `secondary` | `#A5B4FC` | Pro/brand accent, secondary chart series |
| `secondary-hover` | `#C7D2FE` | Hover on secondary text/icons |
| `secondary-soft` | `#23264A` | Pro tints, soft badges |
| `on-secondary` | `#0C111D` | Text on secondary fills |
| `success` | `#47CD89` | Success text/icons, healthy status |
| `success-soft` | `#052E22` | Success tint fills |
| `warning` | `#FDB022` | Warning text/icons, degraded status |
| `warning-soft` | `#33230A` | Warning tint fills |
| `error` | `#F97066` | Error text/icons, destructive actions |
| `error-hover` | `#FDA29B` | Hover on destructive fills |
| `error-soft` | `#3B1210` | Error tint fills |
| `on-status` | `#0C111D` | Text on solid status fills |
| `overlay` | `rgba(0,0,0,0.60)` | Modal overlay |
| `glass-surface` | `rgba(22,27,38,0.82)` | Glass layers (modals/toasts) |
| `glass-border` | `rgba(44,52,66,0.90)` | Glass layer border |

### 3.3 Chart palette (ECharts theme)

Fixed series order — charts never introduce colors outside this list. **Download = series 1, Upload = series 2** everywhere.

| Slot | Light | Dark | Meaning |
|---|---|---|---|
| 1 | `#0E7490` | `#22D3EE` | Primary series / Download |
| 2 | `#4338CA` / `#6172F3` | `#A5B4FC` | Upload / secondary series |
| 3 | `#0B8F5F` | `#32D583` | Tertiary (success-tinted series) |
| 4 | `#C5680A` | `#FDB022` | Quaternary (quota/warning series) |
| 5 | `#7A5AF8` | `#B69DF8` | Fifth series |
| 6 | `#7B8794` | `#667085` | Muted/other |
| Axis line/label | `#DCE3ED` / `#667085` | `#2C3442` / `#9AA4B2` | Axis **label** is text: 4.97:1 light / 6.83:1 dark |
| Grid split line | `#EEF2F7` | `#212A3B` | |
| Tooltip | surface + shadow-md | surface + shadow-dk | Glass style, radius 8 |

### 3.4 Hard color rules

- Primary actions are **cyan/teal**; **indigo (`secondary`) is reserved for Pro branding** and secondary series — never for confirm/cancel actions.
- `success`/`warning`/`error` are status-only; buttons that destroy data use `error` (destructive), never `warning`.
- Tint fills (`*-soft`) never carry text lighter than their base status color; body text on tints is `text`/`text-secondary`.
- Charts use the §3.3 order; never repurpose status colors as decorative series colors.

---

## 4. Typography

### 4.1 Font families

| Token | Family | Usage |
|---|---|---|
| `font-ui` | **Inter Variable** | All UI text: headings, body, labels, buttons, navigation |
| `font-mono` | **JetBrains Mono** | ALL numerals, IP addresses, ports, byte counts, rates, timestamps, table numeric cells, code, keys |

Fallback stacks (system-only, never introduce other fonts): Inter Variable → `system-ui, sans-serif`; JetBrains Mono → `ui-monospace, monospace`. Fonts are bundled with the app (no Google Fonts fetch — the app makes zero outbound connections).

### 4.2 Type scale

Sizes are px at default zoom; line-height is unitless. All sizes use the 4px-based rhythm.

| Token | Size | Weight | Line height | Usage |
|---|---|---|---|---|
| `display` | 32px | 600 | 1.2 | Dashboard headline totals (pairs with mono value) |
| `title-1` | 24px | 600 | 1.25 | Page titles |
| `title-2` | 18px | 600 | 1.3 | Section headings, modal titles |
| `title-3` | 15px | 600 | 1.4 | Card headings, list group headers |
| `body` | 14px | 400 | 1.5 | Default body text, table cells (text) |
| `body-strong` | 14px | 600 | 1.5 | Emphasized body, button labels |
| `label` | 12px | 500 | 1.4 | Input labels, nav items, badges |
| `label-mono` | 12px | 500 | 1.4 | Mono micro-labels (column headers over numeric data) |
| `caption` | 11px | 400 | 1.4 | Helper text, timestamps in history, footnotes |
| `data-lg` | 24px | 500 | 1.2 | Mono large data values (headline bytes) — mono |
| `data` | 14px | 400 | 1.5 | Mono table numerals, IPs, ports — mono |
| `data-sm` | 12px | 400 | 1.4 | Mono sparkline labels, dense tables — mono |

### 4.3 Typographic rules

1. **Tabular numerals always:** every mono usage sets `font-variant-numeric: tabular-nums`. Numeric table columns are right-aligned in `font-mono`.
2. **Bytes and rates** use the app's formatter (B → KiB → MiB → GiB → TiB, binary units; rates as `KB/s`-style decimal for live speed). Formatting is a code-level convention mirrored from this file; never hand-format inside markup.
3. **No text below 11px.** No all-caps text except `label-mono` micro-labels with `letter-spacing: 0.04em`.
4. Headings never use `font-mono`; numerals inside headings (e.g. "90 days") render in `font-ui`.
5. Body text max width: 68ch for paragraphs in onboarding/settings help.

---

## 5. Spacing, Radius, Shadows, Breakpoints

### 5.1 Spacing scale (4px base)

Only these values: `4 · 8 · 12 · 16 · 20 · 24 · 32 · 40 · 48 · 64`.

| Usage | Value |
|---|---|
| Icon ↔ label gap, inline chip padding | 4px (chip padding-x: 8) |
| Related elements (label ↔ input) | 8px |
| Within-component padding (buttons, inputs, cells) | 8px / 12px |
| Card padding | 16px (dense: 12px) |
| Section separation within a page | 24px |
| Page gutters (content padding) | 24px (≥1280px width: 32px) |
| Major layout blocks (sidebar ↔ content) | 0 (border-separated) |

### 5.2 Border radius

| Token | Value | Usage |
|---|---|---|
| `radius-sm` | 6px | Badges, chips, small tags |
| `radius-md` | 8px | Inputs, selects, buttons, tooltips, table containers |
| `radius-lg` | 12px | Cards, panels, popovers |
| `radius-xl` | 16px | Modals, dialogs |
| `radius-full` | 9999px | Status dots, avatars, pill buttons, skeleton circles |

Never mix radii inside one component edge; nested elements use parent radius minus padding (rounded corner continuity).

### 5.3 Shadows (elevation)

| Token | Light | Dark | Usage |
|---|---|---|---|
| `shadow-sm` | `0 1px 2px rgba(16,24,40,0.06)` | `0 1px 2px rgba(0,0,0,0.40)` | Cards at rest (optional, prefer borders) |
| `shadow-md` | `0 4px 8px -2px rgba(16,24,40,0.10), 0 2px 4px -2px rgba(16,24,40,0.06)` | `0 4px 8px -2px rgba(0,0,0,0.50)` | Popovers, dropdowns, chart tooltips, hover-lift on cards |
| `shadow-lg` | `0 12px 24px -4px rgba(16,24,40,0.14), 0 4px 8px -2px rgba(16,24,40,0.08)` | `0 12px 24px -4px rgba(0,0,0,0.55)` | Toasts |
| `shadow-xl` | `0 24px 48px -8px rgba(16,24,40,0.20), 0 8px 16px -4px rgba(16,24,40,0.10)` | `0 24px 48px -8px rgba(0,0,0,0.60)` | Modals |

Rule: flat surfaces rely on `border`; elevation is reserved for things that float (popover → toast → modal order). Focus and hover never use shadows as the only indicator.

### 5.4 Breakpoints / window sizes

Tauri window — min size **960 × 600**. Content is fluid; no mobile layout exists.

| Token | Range | Layout behavior |
|---|---|---|
| `compact` | 960–1279px | Sidebar collapsed to 68px icon rail; tables hide low-priority columns (Local addr, PID) |
| `regular` | 1280–1599px | Default: sidebar 240px, page gutters 24px |
| `wide` | ≥1600px | Sidebar 240px; content max-width 1440px, centered; gutters 32px |

Charts and tables reflow by container, not window: ECharts `resize` on container observation; tables switch to the `compact` column set when their container < 700px.

---

## 6. Motion & Iconography

### 6.1 Motion tokens

| Token | Value | Usage |
|---|---|---|
| `dur-fast` | 100ms | Hovers, color changes, checkbox/switch |
| `dur-base` | 160ms | Popovers, dropdowns, panel transitions |
| `dur-slow` | 240ms | Modals, toasts enter/exit, sidebar collapse |
| `ease-standard` | `cubic-bezier(0.2, 0, 0, 1)` | Default easing |
| `ease-exit` | `cubic-bezier(0.4, 0, 1, 1)` | Exit transitions |

Rules: modals/toasts fade + 8px rise (`translateY`); popovers fade + 4px scale (0.98 → 1); live table rows enter with a 120ms fade, no vertical shift animation; live rate numbers update in place — never animate counting. Respect `prefers-reduced-motion` (§9.5): all transitions ≤ 1ms, no transforms.

### 6.2 Iconography

- **Set: Lucide** (shadcn-svelte default). Stroke style, 2px stroke, round caps/joins — no filled or mixed icon sets.
- Sizes: 16px (inline in tables/buttons), 20px (nav, default), 24px (empty states, headers).
- Color: icons inherit `text`/`text-secondary` or the semantic status token; never multi-colored.
- App icons for processes: rendered in a 20×20 or 32×32 rounded (`radius-sm`) container with `surface-2` fallback when the app has no icon — never a broken image.
---

## 7. Component Specs

Conventions for all components: built on shadcn-svelte primitives; tokens only; focus ring is uniform (§9.2); text minimums per §4. Every interactive component lists its full state set. **Error states for display-only controls are N/A where noted** — validation errors live on inputs/forms.

### 7.1 Buttons

Variants: `primary` (main actions: Enable monitoring, Export), `secondary` (surface-filled alternative), `ghost` (toolbar, table row actions, banner actions), `destructive` (data deletion, daemon shutdown).

| Property | primary | secondary | ghost | destructive |
|---|---|---|---|---|
| Rest fill | `primary` | `surface-2` | transparent | `error` |
| Text/icon | `on-primary` | `text` | `primary` | `on-status` |
| Border | none | `border-strong` | none | none |
| Radius | `radius-md` | `radius-md` | `radius-md` | `radius-md` |
| Padding | 8px/16px (sm: 6px/12px) | same | same | same |
| Min height | 36px (sm: 28px) | same | same | same |

States:

| State | Behavior |
|---|---|
| Default | As table above |
| Hover | primary → `primary-hover`; secondary → `surface-2` + `border` → `border-strong`; ghost → `surface-2` fill; destructive → `error-hover`. 100ms |
| Active/pressed | primary → `primary-active`; fills darken one step; no shadow change |
| Focus-visible | Focus ring (§9.2); never removed |
| Disabled | `surface-2` fill, `text-disabled` text, `border`, cursor `not-allowed`; opacity stays 100% (never dim via opacity) |
| Loading | Label replaced by 16px spinner + "Working…" (or verb: "Enabling…"); width preserved (min-width locked at rest); button disabled during loading |
| Error | N/A on the button itself — errors surface as inline form error (§7.2) or toast (§7.8). A failed async action re-enables the button and shows the error nearby. |

Icon-only buttons (toolbar, table rows, tray-adjacent): 32×32, `radius-md`, ghost behavior, require `aria-label`.

### 7.2 Inputs (text, search, select, number)

Structure: `label` (12px, 500) above the field, 8px gap; optional helper text (caption, `text-secondary`) below; error replaces helper.

| State | Behavior |
|---|---|
| Default | `surface` fill, `border-strong` 1px border, `radius-md`, height 36px, padding 8px/12px, `body` text; placeholder `text-disabled` |
| Hover | Border → `border`; fill unchanged |
| Focus-visible | Border → `primary` + focus ring; no fill change |
| Disabled | `surface-2` fill, `border`, `text-disabled` text; label dims to `text-disabled` |
| Loading (async validation/search) | 16px spinner at field end; field stays editable |
| Error | Border → `error`; message below in `error` 12px 500 with Lucide `circle-alert` 14px; focus ring uses `error` until corrected |
| Read-only | `surface` fill, `border`, no focus ring, not tabbable (use text instead if it matters) |

Search inputs (history, live view): leading Lucide `search` 16px icon, `radius-full` pill variant allowed, clear (`x`) button appears when non-empty.

### 7.3 Cards

- Fill `surface`, border `border` 1px, `radius-lg`, padding 16px (dense 12px).
- Header row: `title-3` + optional right-aligned action (ghost button / link); 16px gap to body.
- Rest: no shadow (border-only). Hover (if clickable, e.g. top-app card): `shadow-md`, border → `border-strong`, cursor pointer; clickable cards are focusable with visible ring.
- Status cards (banner-style inside pages): tint fill `*-soft` with `border` in the status color at 40% alpha; text stays `text`/`text-secondary`; icon in status color.
- Loading: skeleton (§7.10). Empty: §7.11.

### 7.4 Modals / dialogs

Glass treatment per Principle 1: `glass-surface` fill, `backdrop-filter: blur(8px)`, `glass-border`, `radius-xl`, `shadow-xl`, over `overlay`.

- Sizes: sm 400px (confirm), md 560px (license entry, forms), lg 720px (diagnostics). Max height 80vh, body scrolls.
- Header: `title-2` + close (icon button, ghost, `aria-label="Close"`); body 24px padding; footer right-aligned actions 8px gap, sticky bottom with hairline top border `glass-border`.
- States: enter (fade + 8px rise, 240ms `ease-standard`), exit 160ms `ease-exit`; focus trapped (§9.3); Escape closes unless a destructive confirm is pending; `aria-modal="true"` with labelled title.
- Destructive confirm variant: icon `triangle-alert` in `error`, confirm button `destructive`, cancel `secondary`; the *consequence* is named in body text (e.g. "Deletes 90 days of local history").
- Loading variant: submit button loading state (§7.1); no other spinner.
- Error variant: submit failure → inline error banner at top of body (error-soft card, §7.3), form values preserved.
- Never stack more than 2 modals; a confirm over a modal uses the same glass recipe at +1 elevation.

### 7.5 Navbar / App shell

Layout details in §8.1; component specs here.

**Sidebar (240px, collapsed 68px):** fill `background`, right border `border`. Items: 36px height, `radius-md`, 8px vertical gap group; icon 20px + `label` text.
- Default: `text-secondary` text/icon, transparent fill.
- Hover: `surface-2` fill, `text` text.
- Active (current route): `primary-soft` fill, `primary` text+icon, 600 weight.
- Focus-visible: ring. Disabled (Pro-gated items): label + Lucide `lock` 14px in `secondary-soft` pill; click opens upgrade modal (never silent).
- Section labels: `label-mono`, `text-disabled`, uppercase.
- Collapse toggle at bottom; collapsed shows icons only with tooltips (§7.9).

**Topbar (56px):** fill `surface`, bottom border `border`. Left: page title (`title-2`) or breadcrumb. Right: **daemon status chip** (see below), search shortcut (Live/History pages), theme toggle, connection-quality dot for WS link.

**Daemon status chip states** (component `status-chip`): pill `radius-full`, 24px height, dot 8px `radius-full` + 12px 500 label. States:

| State | Fill | Dot | Label |
|---|---|---|---|
| Monitoring | `success-soft` | `success` | "Monitoring" |
| Degraded (partial data, e.g. some interfaces excluded) | `warning-soft` | `warning` | "Degraded" |
| Stopped / disconnected | `error-soft` | `error` | "Stopped" |
| Connecting (WS reconnect in progress) | `surface-2` | `text-disabled` (pulsing dot, reduced-motion-safe) | "Connecting…" |

Chip is a button; click opens Diagnostics. Chip always pairs with a tooltip naming the exact state. Status is never conveyed by color alone (icon shape differs per state: check / alert triangle / x / loader). With no eBPF collector the daemon reports `ebpf:no_collector` and the chip uses **Degraded** (`warning-soft` + `!`), never the green Monitoring style; the tooltip says "No collector — PC live speed works, per-app rates unavailable".

### 7.6 Tables

Used in Live Connections and History. Structure: header row (`label-mono`, uppercase, `text-secondary`, `surface-2` fill), rows 40px (dense 32px), hairline row borders `border`.

- Numeric columns (bytes, rate, port): `font-mono`, tabular-nums, **right-aligned**; header right-aligned to match.
- Text columns (app, remote host): left-aligned, `body`/`data`.
- Row hover: `surface-2`. Selected row: `primary-soft` + 2px left inset bar `primary`. Focus: ring on the interactive row/container per §9.3.
- App cell: 20px icon + name; unknown app uses fallback container (§6.2).
- Verdict/status column (future firewall, and connection state now): icon + color token, text label always present.
- States: **loading** = skeleton rows (§7.10), header stays visible; **empty** = §7.11 empty state centered in table body; **error** = error-soft banner row across the table with retry ghost button; **live mode** = newest rows insert at top with 120ms fade, rates update in place (no row reorder animation while a sort is user-applied).
- Sort: clickable headers with Lucide `arrow-up`/`arrow-down`; active sort header text → `text`. Sticky header within scroll container.
- Selection/scroll: virtualized lists beyond 500 rows; scroll position preserved across data refresh.

### 7.7 Badges & chips

Pill `radius-sm`+1 (7px), padding 2px/8px, `label` size. Variants: neutral (`surface-2`/`text-secondary`), primary-soft, secondary-soft (Pro), status soft fills with status-colored text. Never use badge colors without a text label.

### 7.8 Toasts

Glass recipe like modals but `radius-lg`, `shadow-lg`, width 360px max, bottom-right stack (max 3 visible, older collapse to "+n more").

- Variants (icon 20px + title `body-strong` + optional body `body`): success (`success`), error (`error`, no auto-dismiss), warning (`warning`, 8s), info (`primary`, 6s), action (info + one ghost button, e.g. "Start daemon").
- Timing: success/info 5s; warning 8s; error persists until dismissed. Hover pauses the timer. Progress: none (timers silent).
- Dismiss: `x` icon button, always present. `aria-live="polite"` region; errors use `role="alert"`.
- Enter: slide up 8px + fade 240ms; exit 160ms.

### 7.9 Banners (in-page)

Full-width strip below topbar: status-soft fill, 1px border in status color at 40% alpha, 12px padding, icon + `body` text + right-aligned ghost actions (max 2). Used for daemon-stopped, degraded, retention warnings. Banner is dismissible only if the underlying condition allows it (daemon-stopped is NOT dismissible). `role="status"` (or `alert` for errors).

### 7.10 Skeletons

Fill `surface-2`, `radius-md`, subtle 1.2s opacity pulse (disabled under reduced motion). Match final layout exactly (table → row-height bars; cards → block bars). Never show spinners for lists. skeletons reserve space so no layout shift occurs when data lands.

### 7.11 Empty states

Centered in content area: Lucide icon 24px `text-disabled` in a 48px `surface-2` circle, `title-3` headline, one-line `body` explanation in `text-secondary`, and one optional primary/ghost action. Empty ≠ error: empty states never show retry buttons.

### 7.12 Tooltips

`surface` (dark theme: `surface-2`) fill, `border`, `radius-md`, `shadow-md`, `label` text, 8px padding, arrow-less. Delay 300ms show / 0ms hide; trigger on hover + focus (keyboard users get tooltips too). Max width 280px. Never contains interactive content.

### 7.13 Switches & tabs

**Switch:** 36×20 track (`radius-full`), 16px thumb. Off: `surface-2` track / `border-strong` border / `surface` thumb; On: `primary` track / `on-primary` thumb. Disabled: `surface-2` + `text-disabled` thumb. Focus ring on the control. Labels left, 8px gap; description in caption below.

**Tabs (used in App detail, Settings sections):** underline style — 36px row, `label` text; inactive `text-secondary`, hover `text`, active `text` + 2px underline `primary`; focus ring on tab; `role="tablist"` full keyboard nav (§9.3).

**Range selector (Phase D):** the same segmented pattern as §7.13 tabs, placed once above the Dashboard totals, timeline and top-apps so a single choice drives all three. Five presets: `1 h`, `24 h`, `7 d`, `30 d`, `Cycle`. `24 h` means the current **local calendar day** (local midnight → next midnight) so the Asia/Kolkata reset test is meaningful; `Cycle` uses `billing_cycle_day`. The chosen preset is persisted in the `range_default` setting and therefore shared by Dashboard, Apps, and App detail. A short caption beside the selector names the automatic bucket (`15 min`, `Hour`, `Day`) and the timezone label; never show raw UTC. Active state uses `primary-soft` fill + `primary` text + 600 weight, with `aria-pressed` on each button and a 100ms colour transition (§6.1). Keyboard: left/right arrows move between presets, `Home`/`End` jump to first/last (§9.3).

### 7.14 Progress / data-specific widgets

- **Usage bar** (budgets, top-app lists): 8px track `radius-full` `surface-2`, fill `primary` (or `warning` > 80%, `error` > 95% — always paired with a text % in mono).
- **Sparkline** (ECharts): 2px line, no symbols, `primary`; area fill `primary` at 12% alpha; min height 32px.
- **Charts** (ECharts): axis lines `border`, split lines per §3.3, labels `label-mono` in `text-secondary`; grid alignment 8px; tooltips use glass recipe; legend uses series color dots + text labels. Download/upload always series 1/2 (§3.3).

---

## 8. Page Layouts & Wireframes

App shell first, then every screen from PRD §6.1/§8. ASCII wireframes show structure, not styling. Legend: `■` interactive, `◈` status chip.

### 8.1 App shell

```
┌──────────────┬──────────────────────────────────────────────┐
│ ▣ Logo       │ TOPBAR 56px: Page title   ◈Status  ◐ ○ Theme │
│   (68/240px) ├──────────────────────────────────────────────┤
│ SIDEBAR      │ (banner slot — full width, dismiss rules §7.9)│
│ ▫ Dashboard  ├──────────────────────────────────────────────┤
│ ▫ Live       │                                              │
│ ▫ Apps       │                CONTENT                       │
│ ▫ History    │           (page gutters 24/32px)             │
│ ▫ Settings   │                                              │
│ …            │                                              │
│ ▫ Upgrade    │                                              │
│ ▸ collapse   │                                              │
└──────────────┴──────────────────────────────────────────────┘
```

### 8.2 Onboarding (first run; daemon not yet enabled)

```
┌──────────────────────────────────────────────┐
│                 NWP logo                     │
│        Welcome to Net Watch Pro              │  title-1
│   One-line pitch (body, 68ch max)            │
│                                              │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐         │
│  │card: What│ │card: Where│ │card: How │       │  3 cards §7.3
│  │we collect│ │it's stored│ │to remove │       │
│  └─────────┘ └─────────┘ └─────────┘         │
│                                              │
│   [ ■ Enable monitoring ]  (primary, pkexec) │
│   [ ■ See a demo instead ] (ghost, OQ4-gated)│
└──────────────────────────────────────────────┘
```
Failure state: card below button swaps to error-soft banner with reason + "View docs" ghost + "Retry" primary. Success: auto-routes to Dashboard after health check (≤3s), chip shows Connecting → Monitoring.

### 8.3 Dashboard

```
┌ TOPBAR: Dashboard            ◈ Monitoring ● ┐
├─────────────────────────────────────────────┤
│ NETWORK NOW  ↓ 10.0 Mbps  ↑ 1.0 Mbps  ▁▃▂▅  │  new live card + 60 s sparkline
├─────────────────────────────────────────────┤
│ RANGE [1h][24h][7d][30d][Cycle]  15 min · UTC│  §7.13, one choice drives all below
├─────────────────────────────────────────────┤
│ RANGE TOTAL 12.4 GiB │ ATTRIBUTED 11.1 GiB  │  range-driven headline cards
│ UNATTRIBUTED 1.3 GiB │ CYCLE 8 d left · 61% │  D5 cycle card = used/days/budget
├─────────────────────────────────────────────┤
│ ┌ Usage timeline (ECharts bars) [bucket] ─┐ │
│ │   download ▔▔▔ upload ▂▂▂ (series 1/2)  │ │
│ └─────────────────────────────────────────┘ │
│ ┌ Top apps (ECharts bars) ┐ ┌ Interfaces ┐  │
│ │ app ▇▇▇▇▇ 4.2 GiB →click │ │ wlan0 9.1  │  │
│ │ app ▇▇▇ 2.0   →detail   │ │ -57 dBm · 866 Mbps │ (wifi sysfs)
│ │ Unattributed / system ▇▇│ │ eth0  3.3  │  │  (tooltip: gap vs interface counters)
│ └─────────────────────────┘ └────────────┘  │
└─────────────────────────────────────────────┘
```
Loading: skeleton headline + chart placeholder. Empty (no data yet): empty state "Waiting for first data" + live hint. Error: chart-area error banner + retry. A failed `net.rates` subscription shows "Unavailable" instead of an endless "Loading…"; a failed `live.apps` subscription says per-app rates are unavailable.

**Network now card (§6.1 #11):** two `font-mono` tabular numbers (down/up) with the unit label from `speed_unit` (`Mbps` default, `MB/s` selectable), plus a 60-second ECharts line sparkline in the theme's download/upload series (§3.3). States: loading (before first frame) → live → idle (`0`) → **stale** (no frame for 3 s; show the last value dimmed with a "Stale" text label, never a fresh-looking number) → daemon stopped (existing banner + stale card) → no collector (PC rate still valid; the chip and Apps view explain the missing per-app data). Sparkline uses existing `Chart.svelte` with `animation:false`; no new colours or primitives.
**Unattributed row:** status-soft tint card with status-colored dot + text label (never color-only per §9.5); tooltip explains the gap = interface counter bytes − attributed per-app bytes (headers, ACKs, retransmits, kernel-handled/VPN traffic). Shown even when zero, with "—" until first sample.

### 8.4 Live Connections

```
┌ TOPBAR: Live            [search]  ◈ chip  ⏸ │
├─────────────────────────────────────────────┤
│ filter row: [App ▾] [Protocol ▾] rate: 1.2  │  mono live total
├─────────────────────────────────────────────┤
│ APP        PID  PROTO REMOTE           RATE│  header label-mono
│ firefox    2213 TCP  151.101.1.1:443 ▇2.1MB/s│ mono right-aligned
│ system_upd 881  TCP  17.253.4.1:80    ▇14KB/s │
│ …            (virtualized, new rows fade)   │
└─────────────────────────────────────────────┘
```
Pause freezes data (banner "Frozen at 14:32" + resume ghost). Row click → App detail. Compact breakpoint hides PID/Local. `pid = 0` means the collector could not resolve a process id — render "—" in the PID cell, never "0" (measured eBPF hooks return the softirq thread, so an invented PID would be dishonest). Only IPv4 flows produce rows; the timeline chart and Unattributed row still show the full PC total.

### 8.5 App Detail (`/apps/:id`)

```
┌ TOPBAR: ‹ Apps / Firefox      ◈ chip ┐
├──────────────────────────────────────┤
│ [icon] Firefox     TODAY 2.1GiB      │
│ Tabs: Overview │ Connections │ Endpoints
│ Overview: ECharts usage timeline     │
│   + stat cards (first seen, last active)
│ Connections: live table (this app)   │
│ Endpoints: ranked remote hosts/IPs   │  mono counts
└──────────────────────────────────────┘
```

### 8.6 History

```
┌ TOPBAR: History                 [Export ▾]★Pro
├──────────────────────────────────────┤
│ FREE: [app search] [date range]      │
│ PRO (locked look): + host/port/proto │  lock badges
│ ┌ results table (virtualized) ─────┐ │
│ │ DATE  APP  REMOTE  PORT  ▲BYTES ▼│ │  mono right
│ └──────────────────────────────────┘ │
│ rows: 12,482 · retention: 90 days    │  caption
└──────────────────────────────────────┘
```
Free tier: Pro filter chips render disabled with `lock` + tooltip; Export opens upgrade modal. Pro: filters live, Export menu → CSV/JSON → native save dialog.

### 8.7 Settings

```
┌ TOPBAR: Settings ┐   Sections as tabs or grouped list:
│ General     │ theme (Light/Dark/System switch group), speed unit (Mbps/MB/s)
│ Data        │ retention (number input + helper), clear-data destructive, timezone (IANA picker), billing-cycle day (1–31)
│ Interfaces  │ metered labeling (switch list)
│ Daemon      │ start-on-boot switch, Open diagnostics ghost
│ License     │ key input + status badge / Upgrade card
│ About       │ version (mono), licenses, "no telemetry" statement
└──────────────────────────────────────┘
```
Every setting: label + helper per §7.2; changes save immediately with undo toast for destructive ones.

### 8.8 Upgrade / License modal (sm/md)

```
┌ Glass modal md ─────────────────────┐
│ ✦ Net Watch Pro                  [x]│  secondary accent
│ Unlock Pro                          │
│ • Multi-filter history search       │
│ • CSV/JSON export                   │
│ • Retention up to 1 year            │
│ [ license key input        ]        │  §7.2 states
│ [ Activate ]      [ Buy a license ] │  primary / secondary(→URL)
│ invalid key → error state §7.2      │
└─────────────────────────────────────┘
```

### 8.9 Daemon stopped / Diagnostics

```
BANNER (not dismissible):
⚠ Monitoring stopped — daemon not running.
   [■ Start daemon] [■ Open diagnostics]
Diagnostics modal (lg):
 health checks list ✓/✗/— (mono result text)
 ├ service unit    ✓ active
 ├ eBPF attach     ✗ kernel lacks BTF → guidance link
 ├ database        ✓ writable (SQLite, 2.3 GB)
 journal excerpt (mono, scrollable, last 40 lines)
 [■ Copy report] [■ Restart daemon] [Close]
```

### 8.10 Apps list (Phase D)

```
┌ TOPBAR: Apps                 ◈ Monitoring ● ┐
├─────────────────────────────────────────────┤
│ RANGE [1h][24h][7d][30d][Cycle]  same pref │  §7.13 — shared with Dashboard/detail
├─────────────────────────────────────────────┤
│ [Search apps…]  SORT [Speed|Total|Name]     │  §7.2 input + §7.13 segmented
├─────────────────────────────────────────────┤
│ APP              LIVE ↓ / ↑      RANGE TOTAL│  §7.6 table, mono right-aligned
│ ◉ Motrix         10.0/1.0 Mbps   4.2 GiB    │  low confidence → caption badge
│ ◉ Firefox          0.4/0.1 Mbps  2.0 GiB    │  row click → #apps/{id}
│ Unattributed / system          1.3 GiB      │  always last, status-soft tint
└─────────────────────────────────────────────┘
```
Loading: skeleton rows at final height (§7.10). Empty: §7.11 — "No attributed apps yet". No collector: the table keeps the Unattributed row and shows an in-page banner explaining that live per-app speeds need a collector (L-C9); total column still works from rollups. Default sort is live speed, then range total. Columns follow §7.6: app left, numbers `font-mono` tabular + right-aligned, sticky header, compact breakpoint hides nothing critical (there is no PID column here).
---

## 9. Accessibility Rules

Blockers per the strict rules — a build failing any of these does not ship.

### 9.1 Contrast (WCAG 2.2 AA minimum)

Text ≥ 4.5:1 (≥ 3:1 for text ≥ 24px/600). UI components & focus indicators ≥ 3:1 against adjacent colors. Verified pairs (both themes):

Ratios below are **computed** from the token hex values (`tests/a11y/contrast.py`, P9.4). A pair is only allowed in this table when its measured ratio clears the requirement in both themes.

| Pair | Requirement | Light measured | Dark measured |
|---|---|---|---|
| `text` on `background` / `surface` | ≥ 12 / ≥ 13 | 16.68 / 17.75 | 16.82 / 15.36 |
| `text-secondary` on `surface` | ≥ 4.5 | 7.69 | 6.83 |
| `on-primary` on `primary` | ≥ 4.5 | 5.36 | 10.44 |
| `primary` text/icons on `surface` | ≥ 4.5 | 5.36 | 9.53 |
| `success` / `warning` / `error` as text on `surface` | ≥ 4.5 | 5.69 / 5.43 / 6.57 | 8.50 / 9.36 / 6.18 |
| `text-disabled` on `surface` (disabled exempt) | ≥ 3 | 3.66 | 3.17 |
| `border-strong` vs `surface` (input boundaries) | ≥ 3 | 3.66 | 3.46 |
| chart series 1–6 on `surface` | ≥ 3 | 5.36 / 7.90 / 4.11 / 3.92 / 4.52 / 3.66 | ≥ 7.5 (muted 3.46) |
| chart axis **label** (text) on `surface` | ≥ 4.5 | 4.97 | 6.83 |

New token pairs must be checked with `tests/a11y/contrast.py` before entering this file; chart series sit on `surface` and each pair must clear 3:1.

### 9.2 Focus rings

One uniform ring: **2px `primary` ring at 2px offset** (`outline: 2px solid var(--primary); outline-offset: 2px; border-radius inherits`). On `primary`-filled elements the ring color is `text`. Never `outline: none` without replacement. Inputs use `primary` border + ring on focus.

### 9.3 Keyboard navigation

- Full keyboard operability: sidebar (arrow keys + Enter), tables (roving focus, arrow navigation within cells for interactive rows), tabs (arrow keys per WAI-ARIA APG), menus/comboboxes per APG patterns.
- Focus order = visual order; page content wrapped in `<main>` with skip link as first focusable ("Skip to content").
- Modals: focus trap (§7.4) + focus returns to trigger on close.
- Live tables: keyboard users can pause updates (⏸ control is focusable); new-row insertion must not move focus.
- Shortcuts: `Ctrl+K` command palette/search (V2), `Esc` closes layers top-down. No trap without escape.

### 9.4 Screen readers & semantics

- Status chip: `aria-live="polite"`; state changes announce ("Monitoring degraded").
- Toasts per §7.8 (`polite` / `alert`).
- Tables: proper `<table>` semantics or grid roles; sortable headers `aria-sort`.
- Charts: every ECharts container gets an accessible data-table fallback (visually hidden or toggleable "View as table" link) listing series values; decorative canvas is `aria-hidden`.
- Icon-only buttons require `aria-label`; app icons `alt` = app name.

### 9.5 Status never color-only & reduced motion

- Every status (chip, banner, verdict, budget bar) pairs color with an icon shape **and** a text label.
- `prefers-reduced-motion: reduce` → durations ≤ 1ms, no transforms/pulses (§6.1); skeletons static.
- Zoom/resize: usable at 200% zoom, no loss of content or function (reflow per §5.4 container behavior).

---

## 10. Implementation Mapping (shadcn-svelte + ECharts)

- **shadcn-svelte:** map CSS variables to tokens §3 (e.g. `--primary`, `--background`, `--muted` = `surface-2`, `--destructive` = `error`, `--radius` = `radius-md` base). Components are vendored and customized to §7 specs; variants beyond this file are not added.
- **ECharts:** theme generated from §3.3 (series order fixed), axis/split/label tokens, tooltip glass recipe; charts re-register the theme on theme switch; `resize` observers per §5.4; disable implicit animations that shift layouts; data-zoom only on History timeline (Pro-style long ranges).
- **Fonts:** self-hosted Inter Variable + JetBrains Mono, mono always with `tabular-nums`.
- **Glass layers:** implement as a shared `.glass` utility (surface + blur + border + shadow tokens) used only by modal, toast, and popover-like surfaces.

---

## 11. Governance & Versioning

- This file is the **single source of truth** for UI. Every screen/component PR must state which sections it implements.
- Changes: proposal → design review → update this file with a version bump (semantic: MAJOR = breaking token/structure change, MINOR = new component/variant, PATCH = spec fix). No parallel theme files, no inline hex, no ad-hoc fonts, no unregistered components.
- Screens in PRD but not in §8 (new routes from V2+) must be added here before implementation begins.

---

*End of DESIGN.md. Any UI decision not covered here defaults to: shadcn-svelte defaults styled with the tokens above.*
