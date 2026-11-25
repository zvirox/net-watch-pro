#!/usr/bin/env node
/**
 * Linear-Style Design System Gate.
 * Fails on: blur, translucency, glow, shadows in dark mode, and off-token colors.
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join, relative } from 'node:path';

const APP_ROOT = fileURLToPath(new URL('..', import.meta.url));
const UI_SRC = join(APP_ROOT, 'src');

const TOKEN_FILES = new Set([
    'src/tailwind.css',
    'src/lib/theme/tokens.css',
    'src/lib/components/linear/echarts-linear-theme.ts',
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

const BLUR_REGEX = /(?:-webkit-)?backdrop-filter\s*:|backdrop-blur|filter\s*:\s*blur\(/i;
const TRANSLUCENCY_REGEX =
    /\brgba\s*\(|\bhsla\s*\(|\b(?:bg|border|text|ring|fill|stroke)-(?:white|black|\[[^\]]+\]|[a-z]+-\d+)\/\d+/i;
const GLOW_REGEX = /\bdrop-shadow|\bbox-shadow\s*:[^;]*\b0\s+0\s+[1-9]/i;
const LEGACY_PALETTE_REGEX =
    /\b(?:bg|text|border|divide|ring|fill|stroke)-(?:zinc|slate|gray|neutral|stone|cyan|emerald|rose|indigo|violet|fuchsia|lime|yellow)-\d{2,3}\b/;
const RAW_HEX_REGEX = /#[0-9a-fA-F]{3,8}\b/;

const files = findFiles(UI_SRC, ['.tsx', '.ts', '.css']);
const errors = [];

for (const file of files) {
    const rel = relative(APP_ROOT, file).replace(/\\/g, '/');
    const isTokenFile = TOKEN_FILES.has(rel);
    const content = readFileSync(file, 'utf8');
    const lines = content.split(/\r?\n/);

    lines.forEach((line, idx) => {
        const lineNo = idx + 1;
        const trimmed = line.trim();
        if (trimmed.startsWith('//') || trimmed.startsWith('*') || trimmed.startsWith('/*')) {
            return;
        }

        if (BLUR_REGEX.test(line)) {
            errors.push(`${rel}:${lineNo} [blur] ${trimmed}`);
        }

        if (
            TRANSLUCENCY_REGEX.test(line) &&
            rel !== 'src/lib/components/linear/echarts-linear-theme.ts'
        ) {
            errors.push(`${rel}:${lineNo} [translucency] ${trimmed}`);
        }

        if (GLOW_REGEX.test(line)) {
            errors.push(`${rel}:${lineNo} [glow] ${trimmed}`);
        }

        if (
            /\bshadow-(?:xs|sm|md|lg|xl|2xl|inner)\b/.test(line) &&
            !line.includes('dark:shadow-none')
        ) {
            errors.push(`${rel}:${lineNo} [dark-shadow] ${trimmed}`);
        }

        if (!isTokenFile && (RAW_HEX_REGEX.test(line) || LEGACY_PALETTE_REGEX.test(line))) {
            errors.push(`${rel}:${lineNo} [off-token-color] ${trimmed}`);
        }
    });
}

if (errors.length > 0) {
    console.error('❌ check:linear gate FAILED:');
    for (const err of errors) {
        console.error(`  - ${err}`);
    }
    process.exit(1);
} else {
    console.log(
        '✅ check:linear gate PASSED: Zero blur, translucency, glow, dark-shadow, or off-token colors.',
    );
    process.exit(0);
}
