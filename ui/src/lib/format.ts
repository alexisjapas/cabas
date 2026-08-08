/**
 * Where a rendered number meets a French word.
 *
 * The arithmetic already happened: `amount` arrives as text because the value
 * behind it is an exact rational and any number type here would be a float
 * (Rule 4). What is left is presentation, and presentation is this side's job
 * — the decimal comma, the "≈" on a rounded amount, the plural of "pincée"
 * (DECISIONS 0035).
 */

import type { QuantityView } from './bindings/QuantityView';
import { UNIT_LABEL, UNIT_PLURAL } from './labels';

/**
 * Alphabetical, to a French reader — accents collate where a person expects
 * rather than where their code points fall.
 *
 * Every list on screen is sorted through this. The core sorts by id, which is
 * stable across replicas and is what keeps two devices showing the same
 * library in the same order (`store` sorts every keyed read); stable is not
 * the same as legible, and which of the two you want depends on whether the
 * reader is a person.
 */
export function byName(a: { name: string }, b: { name: string }): number {
  return a.name.localeCompare(b.name, 'fr');
}

/** "28.35" → "28,35". The core renders with a point; French reads a comma. */
export function decimal(amount: string): string {
  return amount.replace('.', ',');
}

/**
 * French pluralises from two upwards, so "1,5 pincée" is singular and
 * "2 pincées" is not.
 *
 * Reading the leading value back out of rendered text looks like parsing, and
 * would be a mistake if anything numeric depended on it — nothing does. The
 * answer picks a word.
 */
function isPlural(amount: string): boolean {
  const bare = /^(\d+)\/(\d+)$/.exec(amount.trim());
  if (bare) {
    const numerator = Number(bare[1]);
    const denominator = Number(bare[2]);
    return denominator !== 0 && numerator / denominator >= 2;
  }
  const leading = /^\d+/.exec(amount.trim());
  return leading !== null && Number(leading[0]) >= 2;
}

/** The unit alone, already pluralised — for a row that shows it separately. */
export function unitLabel(quantity: QuantityView): string {
  const plural = UNIT_PLURAL[quantity.unit];
  return plural !== undefined && isPlural(quantity.amount) ? plural : UNIT_LABEL[quantity.unit];
}

/**
 * One amount, ready to read: "1 1/2 kg", "5", "≈ 28,35 g", "au goût".
 *
 * The "≈" is a choice made here. The core says only *that* the rendering was
 * rounded; it is still adding up the exact value underneath.
 */
export function formatQuantity(quantity: QuantityView): string {
  if (quantity.unit === 'to_taste') return UNIT_LABEL.to_taste;

  const label = unitLabel(quantity);
  const amount = decimal(quantity.amount);
  const text = label === '' ? amount : `${amount} ${label}`;
  return quantity.approximate ? `≈ ${text}` : text;
}

/**
 * Several amounts on one line: "300 g + 2 c. à s.".
 *
 * Not a failure — it is the honest rendering when two contributions cannot be
 * merged because the ingredient declares no density (Rule 5). Two true lines
 * beat one invented number.
 */
export function formatAmounts(amounts: readonly QuantityView[]): string {
  return amounts.map(formatQuantity).join(' + ');
}

const DIVISIONS: readonly { amount: number; unit: Intl.RelativeTimeFormatUnit }[] = [
  { amount: 60, unit: 'second' },
  { amount: 60, unit: 'minute' },
  { amount: 24, unit: 'hour' },
  { amount: 7, unit: 'day' },
  { amount: 4.34524, unit: 'week' },
  { amount: 12, unit: 'month' },
  { amount: Number.POSITIVE_INFINITY, unit: 'year' },
];

const RELATIVE = new Intl.RelativeTimeFormat('fr', { numeric: 'auto' });

/**
 * "il y a 2 heures". The core sends an instant in milliseconds and never a
 * phrase, because the clock this is relative to is the reader's.
 */
export function relativeTime(at: number, now: number = Date.now()): string {
  let delta = (at - now) / 1000;
  for (const division of DIVISIONS) {
    if (Math.abs(delta) < division.amount) {
      return RELATIVE.format(Math.round(delta), division.unit);
    }
    delta /= division.amount;
  }
  return RELATIVE.format(Math.round(delta), 'year');
}
