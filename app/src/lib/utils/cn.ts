/**
 * Join class names, skipping falsy values.
 *
 * Hand-rolled replacement for the clsx + tailwind-merge helper that
 * shadcn-svelte normally vendors — Tailwind is intentionally not in the
 * mandated stack (ARCHITECTURE.md §2), so components merge plain class
 * strings only. Callers must not pass conflicting utilities; last wins.
 *
 * Accepts Svelte's `ClassValue` shape (the type of the `class` attribute,
 * which svelte/elements takes from clsx) so component props typed from
 * `HTML*Attributes` flow through unchanged.
 */
export function cn(...classes: ClassValue[]): string {
  return flatten(classes).filter(Boolean).join(' ');
}

/** Mirrors clsx's `ClassValue` (re-exported by svelte/elements). */
type ClassValue = string | number | bigint | boolean | ClassDictionary | ClassArray | null | undefined;
type ClassDictionary = Record<string, unknown>;
type ClassArray = ClassValue[];

/** Flatten nested class values depth-first, honoring dictionary truthiness. */
function flatten(classes: ClassArray): Array<string | number | bigint | boolean | null | undefined> {
  const out: Array<string | number | bigint | boolean | null | undefined> = [];
  for (const c of classes) {
    if (Array.isArray(c)) {
      out.push(...flatten(c));
    } else if (c && typeof c === 'object') {
      for (const [key, on] of Object.entries(c as ClassDictionary)) {
        if (on) out.push(key);
      }
    } else {
      out.push(c);
    }
  }
  return out;
}
