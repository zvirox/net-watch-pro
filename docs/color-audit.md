# Color Audit — Net Watch Pro (`colors/linear-palette`)

**Date:** 2026-11-26
**Scope:** Complete pre-migration inventory of every hex literal, `rgb()`/`rgba()`, CSS color custom property, shadow class, and green/teal color use across `app/src/` and `app/index.html`.

## Summary of Findings

- **Token files to replace:** `app/src/tailwind.css`, `app/src/lib/theme/tokens.css` (removed), and `app/src/lib/components/linear/echarts-linear-theme.ts` (moved from static hex map to runtime `getComputedStyle(document.documentElement)` lookups).
- **Green / Teal occurrences marked for removal:**
  - `--accent-green`, `--tint-green`, `--tint-green-fg` in `app/src/tailwind.css`, `app/src/lib/theme/tokens.css`, `StatCard.tsx`, `StatusPill.tsx`, `Dashboard.tsx`, `Settings.tsx`, `Diagnostics.tsx`, `Upgrade.tsx`, `Apps.tsx`, `TrafficChart.tsx`, `echarts-linear-theme.ts`.
  - `--accent-teal`, `--tint-teal`, `--tint-teal-fg` in `app/src/tailwind.css`, `app/src/lib/theme/tokens.css`, `StatCard.tsx`, `Onboarding.tsx`, `echarts-linear-theme.ts`.
- **Dark-mode shadow rule change:** Removed `dark:shadow-none` overrides on cards/dialogs/secondary buttons/active tabs so `--shadow-card` (`0 12px 28px -6px rgba(0,0,0,0.5)` dark, `0 1px 2px rgb(16 24 40 / 5%)` light) applies consistently.
- **Modal scrim change:** `LinearDialog.tsx` backdrop moved from `bg-[var(--bg-app)]` to `bg-[var(--scrim)]` (solid dim without blur).

## File-by-File Inventory

### `app/index.html`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 28 | `var(--bg-app)`, `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/App.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 84 | `var(--bg-app)`, `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 101 | `var(--bg-app)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 107 | `var(--tint-blue)`, `var(--tint-blue-fg)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 122 | `var(--accent-amber)`, `var(--tint-amber)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |
| 125 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 127 | `var(--tint-amber-fg)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |

### `app/src/lib/components/StatusChip.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 59 | `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |

### `app/src/lib/components/linear/BreadcrumbBar.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 22 | `var(--border)`, `var(--bg-surface)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 31 | `var(--text-faint)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 39 | `var(--text-muted)`, `var(--text)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 47 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 48 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/components/linear/DataTable.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 44 | `var(--accent-red)`, `var(--tint-red)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 47 | `var(--tint-red-fg)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 50 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 66 | `var(--bg-surface-2)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 75 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 76 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 91 | `var(--bg-surface-2)`, `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 97 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 104 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 120 | `var(--bg-surface-2)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 122 | `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |

### `app/src/lib/components/linear/DonutLegend.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 23 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 33 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 38 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 42 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/components/linear/LinearButton.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 24 | `var(--accent-blue)`, `var(--accent-blue-hover)`, `var(--accent-blue-active)`, `var(--on-accent)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 26 | `var(--bg-surface-2)`, `var(--bg-bar-idle)`, `var(--text)`, `var(--border)`, `var(--border-strong)`, `var(--shadow-card)`, `shadow-none` | No | Use `--shadow-card` (`.shadow-card` / `shadow-[var(--shadow-card)]`) |
| 27 | `var(--bg-surface-2)`, `var(--text-muted)`, `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 28 | `var(--tint-red)`, `var(--accent-red)`, `var(--tint-red-fg)`, `var(--border-strong)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 34 | `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |

### `app/src/lib/components/linear/LinearCard.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 9 | `var(--bg-surface-2)`, `var(--bg-surface)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 14 | `var(--border)`, `var(--shadow-card)`, `shadow-none`, `var(--text)` | No | Use `--shadow-card` (`.shadow-card` / `shadow-[var(--shadow-card)]`) |

### `app/src/lib/components/linear/LinearDialog.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 48 | `var(--bg-app)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 60 | `var(--border-strong)`, `var(--bg-surface)`, `var(--shadow-card)`, `shadow-none` | No | Use `--shadow-card` (`.shadow-card` / `shadow-[var(--shadow-card)]`) |
| 62 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 65 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 69 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 77 | `var(--text-muted)`, `var(--text)`, `var(--bg-surface-2)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 86 | `var(--border)`, `var(--bg-surface-2)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/components/linear/ProgressRow.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 15 | `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 23 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 24 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 28 | `var(--bg-bar-idle)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/components/linear/RangeTabs.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 43 | `var(--border)`, `var(--bg-surface-2)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 54 | `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 56 | `var(--bg-surface)`, `var(--text)`, `var(--border)`, `var(--shadow-card)`, `shadow-none` | No | Use `--shadow-card` (`.shadow-card` / `shadow-[var(--shadow-card)]`) |
| 57 | `var(--text-muted)`, `var(--text)`, `var(--bg-bar-idle)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/components/linear/Sidebar.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 55 | `var(--border)`, `var(--bg-app)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 62 | `var(--border)`, `var(--bg-surface)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 68 | `var(--tint-blue)`, `var(--tint-blue-fg)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 75 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 78 | `var(--bg-surface-2)`, `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 98 | `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 102 | `var(--bg-surface-2)`, `var(--text)`, `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 103 | `var(--text-muted)`, `var(--text)`, `var(--bg-surface-2)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 109 | `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 110 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 121 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 127 | `var(--text-muted)`, `var(--text)`, `var(--bg-surface-2)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 139 | `var(--tint-blue)`, `var(--tint-blue-fg)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 140 | `var(--bg-surface-2)`, `var(--text-muted)`, `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 162 | `var(--text-muted)`, `var(--text)`, `var(--bg-surface-2)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |

### `app/src/lib/components/linear/StatCard.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 24 | `var(--tint-blue)`, `var(--tint-blue-fg)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 25 | `var(--tint-sky)`, `var(--tint-sky-fg)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 26 | `var(--tint-green)`, `var(--tint-green-fg)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 27 | `var(--tint-red)`, `var(--tint-red-fg)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 28 | `var(--tint-amber)`, `var(--tint-amber-fg)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |
| 29 | `var(--tint-teal)`, `var(--tint-teal-fg)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 30 | `var(--tint-purple)`, `var(--tint-purple-fg)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 31 | `var(--tint-orange)`, `var(--tint-orange-fg)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 65 | `var(--accent-red)`, `var(--accent-green)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 70 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 85 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 89 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 125 | `var(--tint-green)`, `var(--tint-green-fg)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 126 | `var(--tint-red)`, `var(--tint-red-fg)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 137 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/components/linear/StatusPill.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 11 | `var(--accent-green)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 12 | `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 13 | `var(--accent-amber)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |
| 14 | `var(--accent-red)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 15 | `var(--text-faint)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 26 | `var(--border)`, `var(--bg-surface-2)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 32 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 34 | `var(--text)`, `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/components/linear/ThemeToggle.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 61 | `var(--text-muted)`, `var(--text)`, `var(--bg-surface-2)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |

### `app/src/lib/components/linear/TitleBar.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 82 | `var(--bg-app)`, `var(--border)`, `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 89 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 92 | `var(--bg-surface-2)`, `var(--border)`, `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 108 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 115 | `var(--text-muted)`, `var(--text)`, `var(--bg-surface-2)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 125 | `var(--text-muted)`, `var(--text)`, `var(--bg-surface-2)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 139 | `var(--text-muted)`, `var(--close-hover)`, `var(--on-accent)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |

### `app/src/lib/components/linear/echarts-linear-theme.ts`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 4 | `#3355FF` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 5 | `#38B6F0` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 6 | `#22C55E` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 7 | `#F43F5E` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 8 | `#EAB308` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 9 | `#1FB5A5` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 10 | `#EC4899` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 11 | `#7C5CE6` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 12 | `#F97316` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 13 | `#202128` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 14 | `#EAF0FF` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 15 | `#111213` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 16 | `#FFFFFF` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 17 | `#1F2125` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 18 | `#E5E7EB` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 19 | `#F5F5F6` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 20 | `#0F1115` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 21 | `#8A8B91` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 22 | `#6B7280` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 60 | `rgb(16 24 40 / 5%)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/screens/AppDetail.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 104 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 122 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 131 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 140 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 150 | `var(--tint-blue-fg)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 160 | `var(--tint-sky-fg)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 173 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 177 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 187 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 200 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 210 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 231 | `var(--tint-blue)`, `var(--tint-blue-fg)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 237 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 240 | `var(--bg-surface-2)`, `var(--border)`, `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 244 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 251 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 254 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 261 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 277 | `var(--accent-blue)`, `var(--text)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 278 | `var(--text-muted)`, `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 307 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/screens/Apps.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 39 | `var(--accent-green)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 94 | `var(--tint-blue)`, `var(--tint-blue-fg)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 100 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 104 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 119 | `var(--tint-blue-fg)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 129 | `var(--tint-sky-fg)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 163 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 181 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 182 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 198 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 207 | `var(--bg-surface-2)`, `var(--border)`, `var(--text)`, `var(--text-faint)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 228 | `var(--bg-surface-2)`, `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/screens/Dashboard.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 32 | `var(--accent-red)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 33 | `var(--accent-amber)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |
| 34 | `var(--accent-green)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 104 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 105 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 122 | `var(--accent-amber)`, `var(--tint-amber)`, `var(--text)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |
| 126 | `var(--tint-amber-fg)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |
| 203 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 206 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 210 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 212 | `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 216 | `var(--accent-sky)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 233 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 236 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 244 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 256 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 259 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 263 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 270 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 285 | `var(--bg-surface-2)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 298 | `var(--border)`, `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 302 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 313 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 314 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 320 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 324 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 334 | `var(--bg-surface-2)`, `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 336 | `var(--accent-sky)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 338 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 343 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 347 | `var(--tint-amber)`, `var(--tint-amber-fg)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |
| 353 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 369 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 372 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 386 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 389 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/screens/Diagnostics.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 53 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 56 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 93 | `var(--accent-red)`, `var(--tint-red)`, `var(--tint-red-fg)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 103 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 104 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 111 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 113 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 128 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 131 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 137 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 140 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 146 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 149 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 154 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 163 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 164 | `var(--border)`, `var(--bg-app)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 166 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 171 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 173 | `var(--text-faint)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 179 | `var(--tint-red)`, `var(--tint-red-fg)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 181 | `var(--tint-amber)`, `var(--tint-amber-fg)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |
| 182 | `var(--tint-green)`, `var(--tint-green-fg)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 187 | `var(--tint-blue-fg)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 190 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/screens/History.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 114 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 126 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 128 | `var(--bg-surface-2)`, `var(--border)`, `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 141 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 145 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 157 | `var(--tint-blue-fg)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 167 | `var(--tint-sky-fg)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 180 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 181 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 201 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 219 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 228 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 237 | `var(--bg-surface)`, `var(--border-strong)`, `var(--text)`, `var(--text-faint)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 249 | `var(--bg-surface)`, `var(--border-strong)`, `var(--text)`, `var(--text-faint)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 257 | `var(--bg-surface)`, `var(--border-strong)`, `var(--text)`, `var(--text-faint)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 263 | `var(--bg-surface)`, `var(--border-strong)`, `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 278 | `var(--bg-surface-2)`, `var(--border)`, `var(--text-muted)`, `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/screens/Live.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 66 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 76 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 80 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 91 | `var(--bg-surface-2)`, `var(--border)`, `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 101 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 105 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 117 | `var(--tint-blue-fg)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 127 | `var(--tint-sky-fg)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 137 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 156 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 157 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 180 | `var(--accent-amber)`, `var(--tint-amber)`, `var(--text)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |
| 184 | `var(--tint-amber-fg)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |
| 199 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 208 | `var(--bg-surface)`, `var(--border-strong)`, `var(--text)`, `var(--text-faint)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 216 | `var(--bg-surface)`, `var(--border-strong)`, `var(--text)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |

### `app/src/lib/screens/Onboarding.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 11 | `var(--tint-blue)`, `var(--tint-blue-fg)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 17 | `var(--tint-teal)`, `var(--tint-teal-fg)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 23 | `var(--tint-amber)`, `var(--tint-amber-fg)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |
| 32 | `var(--bg-app)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 35 | `var(--border)`, `var(--bg-surface)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 42 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 45 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 61 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 64 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 75 | `var(--accent-red)`, `var(--tint-red)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 79 | `var(--tint-red-fg)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 82 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 111 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/screens/Settings.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 13 | `var(--accent-red)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 14 | `var(--accent-amber)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |
| 15 | `var(--accent-green)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 57 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 58 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 63 | `var(--tint-green-fg)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 71 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 83 | `var(--accent-blue)`, `var(--text)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 84 | `var(--text-muted)`, `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 95 | `var(--accent-red)`, `var(--tint-red)`, `var(--tint-red-fg)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 105 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 108 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 115 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 118 | `var(--border)`, `var(--bg-surface-2)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 131 | `var(--bg-surface)`, `var(--text)`, `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 132 | `var(--text-muted)`, `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 142 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 145 | `var(--border)`, `var(--bg-surface-2)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 153 | `var(--bg-surface)`, `var(--text)`, `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 154 | `var(--text-muted)`, `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 167 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 172 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 183 | `var(--bg-surface-2)`, `var(--border-strong)`, `var(--text)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 185 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 191 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 202 | `var(--bg-surface-2)`, `var(--border-strong)`, `var(--text)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 207 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 214 | `var(--bg-surface-2)`, `var(--border-strong)`, `var(--text)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 224 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 229 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 242 | `var(--bg-surface-2)`, `var(--border-strong)`, `var(--text)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 246 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 259 | `var(--bg-surface-2)`, `var(--border-strong)`, `var(--text)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 263 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 274 | `var(--bg-surface-2)`, `var(--border-strong)`, `var(--text)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 281 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 316 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 317 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 324 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 327 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 358 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 361 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 364 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 374 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 392 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 393 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 406 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 410 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 411 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 414 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 415 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 416 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 417 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/screens/Upgrade.tsx`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 45 | `var(--border)`, `var(--bg-surface-2)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 46 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 49 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 55 | `var(--tint-red-fg)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 72 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 79 | `var(--tint-green)`, `var(--tint-green-fg)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 88 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 99 | `var(--bg-surface-2)`, `var(--border-strong)`, `var(--text)`, `var(--text-faint)`, `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 101 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 107 | `var(--tint-red-fg)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 112 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/lib/theme/tokens.css`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 8 | `#f3f4f6` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 9 | `#ffffff` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 10 | `#f4f5f7` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 11 | `#eaf0ff` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 12 | `#e5e7eb` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 13 | `#d1d5db` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 14 | `#0f1115` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 15 | `#6b7280` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 16 | `#9ca3af` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 17 | `#3355ff` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 18 | `#38b6f0` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 19 | `#22c55e` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 20 | `#f43f5e` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 21 | `#eab308` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 22 | `#1fb5a5` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 23 | `#ec4899` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 24 | `#7c5ce6` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 25 | `#f97316` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 29 | `#050505` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 30 | `#111213` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 31 | `#1a1b1e` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 32 | `#202128` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 33 | `#1f2125` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 34 | `#2c2e33` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 35 | `#f5f5f6` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 36 | `#8a8b91` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 37 | `#5a5b62` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |

### `app/src/tailwind.css`

| Line | Tokens / Colors Matched | Green / Teal? | Planned Role Replacement |
| ---: | --- | :---: | --- |
| 31 | `var(--bg-app)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 32 | `var(--bg-surface)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 33 | `var(--bg-surface-2)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 34 | `var(--bg-bar-idle)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 35 | `var(--border)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 36 | `var(--border-strong)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 37 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 38 | `var(--text-muted)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 39 | `var(--text-faint)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 41 | `var(--accent-blue)` | No | Primary role -> `--primary` / `--primary-text` / `--primary-tint`; non-primary chart series -> `--primary` |
| 42 | `var(--accent-sky)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 43 | `var(--accent-green)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 44 | `var(--accent-red)` | No | Error / negative -> `--red` / `--red-text` / `--chip-red` |
| 45 | `var(--accent-amber)` | No | Warning -> `--amber` with `--bg-surface-2` / `--border` / `--text` |
| 46 | `var(--accent-teal)` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 47 | `var(--accent-pink)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 48 | `var(--accent-purple)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 49 | `var(--accent-orange)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 58 | `#3355ff` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 59 | `#2944cc` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 60 | `#1f3399` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 61 | `#38b6f0` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 62 | `#22c55e` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 63 | `#f43f5e` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 64 | `#eab308` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 65 | `#1fb5a5` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 66 | `#ec4899` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 67 | `#7c5ce6` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 68 | `#f97316` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 69 | `#ffffff` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 70 | `#e5484d` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 78 | `#050505` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 79 | `#111213` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 80 | `#1a1b1e` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 81 | `#202128` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 82 | `#1f2125` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 83 | `#2c2e33` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 84 | `#f5f5f6` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 85 | `#8a8b91` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 86 | `#5a5b62` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 89 | `#111213` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 90 | `#151a2f` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 91 | `#6b87ff` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 92 | `#15252e` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 93 | `#38b6f0` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 94 | `#13271c` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 95 | `#22c55e` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 96 | `#2c171d` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 97 | `#f43f5e` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 98 | `#2b2511` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 99 | `#eab308` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 100 | `#122524` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 101 | `#1fb5a5` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 102 | `#2b1823` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 103 | `#ec4899` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 104 | `#1e1b2c` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 105 | `#9f87ff` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 106 | `#2d1e13` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 107 | `#f97316` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 113 | `#f3f4f6` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 114 | `#ffffff` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 115 | `#f4f5f7` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 116 | `#eaf0ff` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 117 | `#e5e7eb` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 118 | `#d1d5db` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 119 | `#0f1115` | No | Moved to `linear-theme.css` / `app-accent.css` or runtime CSS var lookup |
| 120 | `#6b7280` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 121 | `#9ca3af` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 122 | `rgb(16 24 40 / 5%)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 124 | `#FFFFFF` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 125 | `#ebeeff` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 126 | `#2544db` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 127 | `#ebf7fe` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 128 | `#0369a1` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 129 | `#e9f9ef` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 130 | `#15803d` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 131 | `#feecef` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 132 | `#be123c` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 133 | `#fdf8e7` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 134 | `#a16207` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 135 | `#e9f8f6` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 136 | `#0f766e` | **YES (remove)** | Teal removed; green/positive -> `--sky` / `--sky-text` / `--chip-sky` or removed |
| 137 | `#fdecf5` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 138 | `#be185d` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 139 | `#f2effd` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 140 | `#5b21b6` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 141 | `#fef1e8` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 142 | `#c2410c` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 159 | `var(--font-sans)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 160 | `var(--bg-app)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 161 | `var(--text)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
| 181 | `var(--border-strong)` | No | Repoint by UI role to `linear-theme.css` / `app-accent.css` tokens |
