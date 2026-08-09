/**
 * Checks the hand-written QR encoder against `qrencode`, and prints what it
 * produces so the harness can compare it with what the page drew.
 *
 * Two different questions, and they need separating (DECISIONS 0047):
 *
 * - **Is the encoder right?** Compared against an implementation that shares
 *   none of its assumptions, module for module. The one thing two correct
 *   encoders may disagree about is the **mask**: all eight are valid, the
 *   format bits say which was used, and choosing between them is a scoring
 *   heuristic the specification leaves open. So the assertion is that
 *   qrencode's symbol is *one of the eight this encoder can produce* — which
 *   pins the data, the error correction, the function patterns and the format
 *   bits, and leaves taste out of it. Demanding the same mask fails on about a
 *   quarter of inputs, which is how this was found: a minted phrase differs
 *   every run, so CI met a case the machine it was written on never did.
 * - **Does the component draw it?** That is the page's business, so this only
 *   prints the matrix and `smoke.mjs` does the comparing.
 *
 * Run through `node --experimental-strip-types`; `ui-test` does that for you.
 */

import { execFileSync } from 'node:child_process';
import { MASK_COUNT, SIZE, encode } from '../src/lib/qr.ts';

/** Fixed phrases, so this says the same thing on every machine — plus
 *  whatever the caller passes, which is how the app's own minted phrase gets
 *  checked as well. */
const FIXED = [
  'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about',
  'zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong',
  'season sketch slogan sniff strategy swallow thunder tornado ability absorb youth zebra',
];

function reference(text: string): boolean[][] {
  const ascii = execFileSync(
    'qrencode',
    ['-l', 'L', '-v', '6', '-m', '0', '-t', 'ASCII', '-o', '-'],
    { input: text, encoding: 'utf8' },
  );
  return ascii
    .split('\n')
    .filter((row) => row.length > 0)
    .map((row) => {
      const modules: boolean[] = [];
      for (let i = 0; i * 2 < row.length; i += 1) modules.push(row[i * 2] === '#');
      return modules;
    });
}

function same(a: boolean[][], b: boolean[][]): boolean {
  if (a.length !== SIZE || b.length !== SIZE) return false;
  for (let row = 0; row < SIZE; row += 1) {
    for (let col = 0; col < SIZE; col += 1) {
      if (a[row]?.[col] !== b[row]?.[col]) return false;
    }
  }
  return true;
}

function verify(text: string): void {
  const theirs = reference(text);
  for (let mask = 0; mask < MASK_COUNT; mask += 1) {
    if (same(encode(text, mask), theirs)) return;
  }
  throw new Error(
    `qr: no mask reproduces qrencode's symbol for ${JSON.stringify(text)} — ` +
      'the disagreement is in the data, not in the choice of mask',
  );
}

const [mode, argument] = process.argv.slice(2);

if (mode === '--print') {
  if (argument === undefined) throw new Error('qr-check: --print needs a phrase');
  const modules = encode(argument);
  console.log(modules.map((row) => row.map((dark) => (dark ? '1' : '0')).join('')).join('\n'));
} else {
  const phrases = argument === undefined ? FIXED : [...FIXED, argument];
  for (const phrase of phrases) verify(phrase);
  console.log(`  ok  the QR encoder agrees with qrencode on ${phrases.length} phrases`);
}
