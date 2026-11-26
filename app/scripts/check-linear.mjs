#!/usr/bin/env node
/**
 * Linear-Style Design System Gate (Locked Shared Palette + Royal Blue Primary).
 * Enforces Section 7 rules across all frontend sources:
 * 1. Colors (hex, rgb/hsl, Tailwind palette classes) only inside linear-theme.css & app-accent.css.
 * 2. Forbidden effects: backdrop-filter, blur(), translucent surface fills, glow, inset highlights.
 * 3. Allowed translucency: only --scrim, --chart-fade-up, --chart-fade-down, --shadow-card in linear-theme.css.
 * 4. Shadows: the only allowed box-shadow is var(--shadow-card) (.shadow-card).
 * 5. Retired hex list banned across all sources.
 * 6. Green hues (75°-170°) and Tailwind green/emerald/teal/lime/cyan-green classes banned.
 * 7. Token sync: linear-theme.css sha256 match and exact app-accent.css match.
 * 8. Foreign primary hex values from sibling apps banned across all sources.
 */
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join, relative } from 'node:path';

const APP_ROOT = fileURLToPath(new URL('..', import.meta.url));
const UI_SRC = join(APP_ROOT, 'src');
const INDEX_HTML = join(APP_ROOT, 'index.html');

const TOKEN_FILES = new Set(['src/styles/linear-theme.css', 'src/styles/app-accent.css']);

const EXPECTED_THEME_SHA256 = 'a542c03e83fd42bff20815cac21e37182261ebb86f1ca6b04f2341b0c2b21a4e';

const EXPECTED_APP_ACCENT = `/* src/styles/app-accent.css : Net Watch Pro (Royal blue). The only styling file that differs between apps. */
:root, [data-theme="light"] {
  --primary: #2563EB;
  --primary-hover: #2157CF;
  --primary-fg: #FFFFFF;
  --primary-text: #2563EB;
  --primary-tint: #F0F4FE;
}
[data-theme="dark"] {
  --primary: #346AFE;
  --primary-hover: #2E5DE0;
  --primary-fg: #FFFFFF;
  --primary-text: #6B9BFF;
  --primary-tint: #1F273C;
}
`;

const RETIRED_HEXES = new Set([
    '#050505',
    '#059669',
    '#08080A',
    '#0891B2',
    '#09090B',
    '#0ACC8A',
    '#0D9488',
    '#0F1115',
    '#10B981',
    '#111213',
    '#121215',
    '#15803D',
    '#17171C',
    '#1A1B1E',
    '#1A9942',
    '#1B2531',
    '#1C261E',
    '#1F2125',
    '#1FB5A5',
    '#202026',
    '#202128',
    '#22C55E',
    '#23232A',
    '#2C2E33',
    '#33333D',
    '#3355FF',
    '#3483FF',
    '#38B6F0',
    '#38C84F',
    '#5A5B62',
    '#6B7280',
    '#71717A',
    '#7C3AED',
    '#7C5CE6',
    '#831843',
    '#8A8B91',
    '#8E8E93',
    '#9CA3AF',
    '#A1A1AA',
    '#A855F7',
    '#C8C8D0',
    '#DBEAFE',
    '#DCFCE7',
    '#E2E2E7',
    '#E8E8EC',
    '#EAB308',
    '#EAF0FF',
    '#EC4899',
    '#F2F2F4',
    '#F43F5E',
    '#F4F4F5',
    '#F4F5F7',
    '#F59E0B',
    '#F5F5F6',
    '#F7F7F8',
]);

const FOREIGN_PRIMARIES = new Set([
    '#06B6D4',
    '#0C667F',
    '#0E7490',
    '#193236',
    '#22D3EE',
    '#24BFD9',
    '#2C213C',
    '#331D36',
    '#351B22',
    '#3B281B',
    '#6023BF',
    '#7A36E0',
    '#8B3DFF',
    '#8C1032',
    '#8F199A',
    '#A21CAF',
    '#A921BA',
    '#AB390B',
    '#B28CFF',
    '#B7153E',
    '#C026D3',
    '#C2410C',
    '#D01846',
    '#E879F9',
    '#EEF5F7',
    '#F5F0FC',
    '#F8EEF1',
    '#F8EFF9',
    '#F97316',
    '#FA8432',
    '#FB7185',
    '#FB923C',
    '#FBF2EE',
]);

const ALLOWED_ALPHA_VARS = new Set([
    '--scrim',
    '--chart-fade-up',
    '--chart-fade-down',
    '--shadow-card',
]);

function findFiles(dir, exts, acc = []) {
    for (const entry of readdirSync(dir)) {
        const full = join(dir, entry);
        const st = statSync(full);
        if (st.isDirectory()) {
            findFiles(full, exts, acc);
        } else if (exts.some((ext) => entry.endsWith(ext))) {
            acc.push(full);
        }
    }
    return acc;
}

function rgbToHsl(r, g, b) {
    const rn = r / 255;
    const gn = g / 255;
    const bn = b / 255;
    const max = Math.max(rn, gn, bn);
    const min = Math.min(rn, gn, bn);
    const d = max - min;
    if (d === 0) return { h: 0, s: 0, l: (max + min) / 2 };
    const l = (max + min) / 2;
    const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    let h = 0;
    if (max === rn) h = ((gn - bn) / d + (gn < bn ? 6 : 0)) * 60;
    else if (max === gn) h = ((bn - rn) / d + 2) * 60;
    else h = ((rn - gn) / d + 4) * 60;
    return { h, s: s * 100, l: l * 100 };
}

function hexToRgb(hex) {
    const clean = hex.slice(1);
    if (clean.length === 3 || clean.length === 4) {
        return {
            r: parseInt(clean[0] + clean[0], 16),
            g: parseInt(clean[1] + clean[1], 16),
            b: parseInt(clean[2] + clean[2], 16),
        };
    }
    if (clean.length === 6 || clean.length === 8) {
        return {
            r: parseInt(clean.slice(0, 2), 16),
            g: parseInt(clean.slice(2, 4), 16),
            b: parseInt(clean.slice(4, 6), 16),
        };
    }
    return null;
}

function isGreenHue(h, s) {
    return s >= 10 && h >= 75 && h <= 170;
}

const files = findFiles(UI_SRC, ['.tsx', '.ts', '.css']);
if (existsSync(INDEX_HTML)) files.push(INDEX_HTML);

const violations = [];

// Rule 7: Token sync verification
const themePath = join(UI_SRC, 'styles/linear-theme.css');
const accentPath = join(UI_SRC, 'styles/app-accent.css');
if (!existsSync(themePath)) {
    violations.push('src/styles/linear-theme.css [missing required token file]');
} else {
    const actualSha = createHash('sha256').update(readFileSync(themePath)).digest('hex');
    if (actualSha !== EXPECTED_THEME_SHA256) {
        violations.push(
            `src/styles/linear-theme.css [token sync] -> sha256 ${actualSha} !== expected ${EXPECTED_THEME_SHA256}`,
        );
    }
}

if (!existsSync(accentPath)) {
    violations.push('src/styles/app-accent.css [missing required token file]');
} else {
    const actualAccent = readFileSync(accentPath, 'utf8');
    if (actualAccent !== EXPECTED_APP_ACCENT) {
        violations.push('src/styles/app-accent.css [token sync] -> does not match Section 3 block');
    }
}

const TAILWIND_PALETTE_REGEX =
    /\b(?:bg|text|border|ring|fill|stroke|from|via|to|decoration|outline|divide|placeholder|accent)-(?:(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-(?:[0-9]{2,3}|950)|(?:green|emerald|teal|lime|cyan-green)(?:-[0-9]{2,3})?)\b/g;

const HEX_REGEX = /#[0-9a-fA-F]{3,8}\b/g;
const RGB_REGEX = /\brgba?\(\s*(\d+)\s*[,\s]\s*(\d+)\s*[,\s]\s*(\d+)/g;
const HSL_REGEX = /\bhsla?\(\s*(\d+(?:\.\d+)?)\s*(?:deg)?\s*[,\s]\s*(\d+(?:\.\d+)?)%/g;

for (const file of files) {
    const rel = relative(APP_ROOT, file).replace(/\\/g, '/');
    const isTokenFile = TOKEN_FILES.has(rel);
    const lines = readFileSync(file, 'utf8').split('\n');

    lines.forEach((line, idx) => {
        const loc = `${rel}:${idx + 1}`;

        // Rule 2: Blur, translucent surface fills, glow, inset highlights
        if (/(?:-webkit-)?backdrop-filter\s*:|\bblur\(|\bbackdrop-blur-/.test(line)) {
            violations.push(`${loc} [forbidden blur/backdrop-filter] -> ${line.trim()}`);
        }
        if (/\b(?:bg|border|text)-(?:white|black)\/[0-9]/.test(line)) {
            violations.push(`${loc} [translucent surface fill] -> ${line.trim()}`);
        }
        if (
            /\b(?:drop-shadow|shadow-(?:sm|md|lg|xl|2xl|inner)|ring-inset)\b|\bshadow-\[/.test(line)
        ) {
            violations.push(`${loc} [forbidden shadow/glow/arbitrary shadow] -> ${line.trim()}`);
        }
        if (/\binset\b/.test(line) && /box-shadow/i.test(line)) {
            violations.push(`${loc} [forbidden inset highlight] -> ${line.trim()}`);
        }

        // Rule 3: Allowed translucency only in --scrim, --chart-fade-up, --chart-fade-down, --shadow-card in linear-theme.css
        const hasAlphaSyntax =
            /\b(?:rgba|hsla)\(/.test(line) || /\b(?:rgb|hsl)\([^)]*\//.test(line);
        if (hasAlphaSyntax) {
            const varMatch = /^\s*(--[a-z0-9-]+)\s*:/.exec(line);
            if (
                rel !== 'src/styles/linear-theme.css' ||
                !varMatch ||
                !ALLOWED_ALPHA_VARS.has(varMatch[1])
            ) {
                violations.push(
                    `${loc} [forbidden alpha color outside allowed token vars] -> ${line.trim()}`,
                );
            }
        }

        // Rule 4: Only allowed box-shadow is var(--shadow-card)
        const boxShadowMatch = /\bbox-shadow\s*:\s*([^;}"'`]+)/.exec(line);
        if (boxShadowMatch) {
            const val = boxShadowMatch[1].trim();
            if (val !== 'var(--shadow-card)') {
                violations.push(`${loc} [forbidden box-shadow value: ${val}]`);
            }
        }

        // Rule 1, 5, 6, 8 on hexes
        HEX_REGEX.lastIndex = 0;
        let hexMatch;
        while ((hexMatch = HEX_REGEX.exec(line)) !== null) {
            const hex = hexMatch[0].toUpperCase();
            if (!isTokenFile) {
                violations.push(`${loc} [hex outside token files] -> ${hexMatch[0]}`);
            }
            if (RETIRED_HEXES.has(hex)) {
                violations.push(`${loc} [retired palette hex] -> ${hexMatch[0]}`);
            }
            if (FOREIGN_PRIMARIES.has(hex)) {
                violations.push(`${loc} [foreign app primary hex] -> ${hexMatch[0]}`);
            }
            const rgb = hexToRgb(hex);
            if (rgb) {
                const { h, s } = rgbToHsl(rgb.r, rgb.g, rgb.b);
                if (isGreenHue(h, s)) {
                    violations.push(`${loc} [green hue hex (${Math.round(h)}°)] -> ${hexMatch[0]}`);
                }
            }
        }

        // Rule 1 & 6 on rgb()/rgba()
        RGB_REGEX.lastIndex = 0;
        let rgbMatch;
        while ((rgbMatch = RGB_REGEX.exec(line)) !== null) {
            if (!isTokenFile) {
                violations.push(`${loc} [rgb/rgba outside token files] -> ${rgbMatch[0]}`);
            }
            const { h, s } = rgbToHsl(
                Number(rgbMatch[1]),
                Number(rgbMatch[2]),
                Number(rgbMatch[3]),
            );
            if (isGreenHue(h, s)) {
                violations.push(`${loc} [green hue rgb (${Math.round(h)}°)] -> ${rgbMatch[0]}`);
            }
        }

        // Rule 1 & 6 on hsl()/hsla()
        HSL_REGEX.lastIndex = 0;
        let hslMatch;
        while ((hslMatch = HSL_REGEX.exec(line)) !== null) {
            if (!isTokenFile) {
                violations.push(`${loc} [hsl/hsla outside token files] -> ${hslMatch[0]}`);
            }
            if (isGreenHue(Number(hslMatch[1]), Number(hslMatch[2]))) {
                violations.push(`${loc} [green hue hsl (${hslMatch[1]}°)] -> ${hslMatch[0]}`);
            }
        }

        // Rule 1 & 6: Tailwind palette classes & green/emerald/teal/lime/cyan-green
        if (!isTokenFile) {
            TAILWIND_PALETTE_REGEX.lastIndex = 0;
            let twMatch;
            while ((twMatch = TAILWIND_PALETTE_REGEX.exec(line)) !== null) {
                violations.push(
                    `${loc} [Tailwind palette class outside token files] -> ${twMatch[0]}`,
                );
            }
        }
    });
}

if (violations.length > 0) {
    console.error('❌ check:linear gate FAILED:');
    for (const v of violations) {
        console.error(`  - ${v}`);
    }
    process.exit(1);
} else {
    console.log(
        '✅ check:linear gate PASSED: Shared palette verified, Royal blue primary intact, zero green/retired/foreign colors.',
    );
    process.exit(0);
}
