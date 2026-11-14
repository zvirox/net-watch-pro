/**
 * Join class names, skipping falsy values.
 *
 * Hand-rolled replacement for the clsx + tailwind-merge helper that
 * shadcn-svelte normally vendors — Tailwind is intentionally not in the
 * mandated stack (ARCHITECTURE.md §2), so components merge plain class
 * strings only. Callers must not pass conflicting utilities; last wins.
 */
export function cn(...classes: Array<string | false | null | undefined>): string {
  return classes.filter(Boolean).join(' ');
}
