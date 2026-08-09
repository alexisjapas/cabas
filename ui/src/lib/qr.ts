/**
 * A QR encoder, for one payload: a 12-word recovery phrase.
 *
 * Pairing shows the phrase and its QR side by side (DECISIONS 0021), so this
 * has exactly one job and is fixed to it — **version 6, error correction L,
 * byte mode**. That is not a shortcut, it is what removes the risk: the format
 * is a wall of per-version tables, and a wrong row is a code that does not
 * scan rather than a code that fails to build. One version means one row, and
 * `ui-test` compares what this produces against `qrencode`, module for module
 * (DECISIONS 0047).
 *
 * Version 6 is 41×41 modules and holds 136 data codewords at level L. The
 * longest phrase BIP39 can produce is twelve eight-letter words and eleven
 * spaces — 107 bytes, plus two for the header. Version 5 would not hold it and
 * version 7 would drag in the version-information block, which only exists
 * from 7 up. So the smallest version that fits is also the last one that stays
 * simple.
 *
 * Written by hand, like the service worker and the test harness: the
 * alternative was a dependency for something that is a page of arithmetic, and
 * this way `ui/` keeps a dependency list a person can read.
 *
 * Everything below indexes flat arrays, which `noUncheckedIndexedAccess` makes
 * `number | undefined`. Rather than assert at every step, the accessors treat
 * a miss as zero: every index here is in range by construction, since the
 * loops are bounded by the arrays they walk.
 */

/** The only version this encodes. See the module docs for why it is fixed. */
const VERSION = 6;
/** `17 + 4 × version`, the module count along one side. */
export const SIZE = 17 + 4 * VERSION;

/** Version 6 at level L: two blocks of 68 data codewords, 18 EC each. */
const BLOCKS = 2;
const DATA_PER_BLOCK = 68;
const EC_PER_BLOCK = 18;
const DATA_CODEWORDS = BLOCKS * DATA_PER_BLOCK;

/**
 * Bits left over after the codewords, placed as zeros. Seven for versions 2 to
 * 6; it changes with the version and is the last thing anyone thinks to check.
 */
const REMAINDER_BITS = 7;

/** Alignment pattern centres for version 6. Three of the four combinations sit
 *  inside a finder, which owns that corner already. */
const ALIGNMENT = [6, 34];

/** Level L, as the two bits the format information carries. */
const EC_LEVEL_BITS = 0b01;

// --- GF(256), the field Reed-Solomon works in ------------------------------

const EXP = new Uint8Array(512);
const LOG = new Uint8Array(256);
{
  let x = 1;
  for (let i = 0; i < 255; i += 1) {
    EXP[i] = x;
    LOG[x] = i;
    // ×2 in the field QR specifies: modulo x⁸ + x⁴ + x³ + x² + 1.
    x <<= 1;
    if (x & 0x100) x ^= 0x11d;
  }
  for (let i = 255; i < 512; i += 1) EXP[i] = EXP[i - 255] ?? 0;
}

const exp = (i: number): number => EXP[i] ?? 0;
const log = (i: number): number => LOG[i] ?? 0;

function mul(a: number, b: number): number {
  if (a === 0 || b === 0) return 0;
  return exp(log(a) + log(b));
}

/** The generator polynomial for `degree` error-correction codewords:
 *  (x − α⁰)(x − α¹)…, coefficients from the highest power down. */
function generator(degree: number): Uint8Array {
  let poly = Uint8Array.of(1);
  for (let i = 0; i < degree; i += 1) {
    const next = new Uint8Array(poly.length + 1);
    for (let j = 0; j < poly.length; j += 1) {
      const coefficient = poly[j] ?? 0;
      // Multiplying by (x − αⁱ): the x term keeps the coefficient's position,
      // the constant term pushes it one power down. Swapping these two writes
      // the polynomial backwards, which is still a polynomial and produces a
      // symbol no scanner will read.
      next[j] = (next[j] ?? 0) ^ coefficient;
      next[j + 1] = (next[j + 1] ?? 0) ^ mul(coefficient, exp(i));
    }
    poly = next;
  }
  return poly;
}

/** The remainder of the data polynomial divided by the generator — the error
 *  correction codewords, in order. */
function correction(data: Uint8Array, count: number): Uint8Array {
  const gen = generator(count);
  const remainder = new Uint8Array(count);
  for (const byte of data) {
    const factor = byte ^ (remainder[0] ?? 0);
    remainder.copyWithin(0, 1);
    remainder[count - 1] = 0;
    for (let i = 0; i < count; i += 1) {
      remainder[i] = (remainder[i] ?? 0) ^ mul(gen[i + 1] ?? 0, factor);
    }
  }
  return remainder;
}

// --- the payload -----------------------------------------------------------

/**
 * Mode indicator, length, the bytes, a terminator and the pad pattern the
 * specification names — 0xEC and 0x11, alternating, which exist to give the
 * decoder something recognisable rather than a run of zeros.
 */
function codewords(text: string): Uint8Array {
  const bytes = new TextEncoder().encode(text);
  if (bytes.length + 2 > DATA_CODEWORDS) {
    throw new Error(`qr: ${bytes.length} bytes is more than version ${VERSION} holds`);
  }

  const bits: number[] = [];
  const push = (value: number, width: number): void => {
    for (let i = width - 1; i >= 0; i -= 1) bits.push((value >> i) & 1);
  };

  push(0b0100, 4); // byte mode
  push(bytes.length, 8); // one byte of length, for versions 1 to 9
  for (const byte of bytes) push(byte, 8);
  // Terminator, then out to a whole codeword.
  for (let i = 0; i < 4 && bits.length < DATA_CODEWORDS * 8; i += 1) bits.push(0);
  while (bits.length % 8 !== 0) bits.push(0);

  const out = new Uint8Array(DATA_CODEWORDS);
  for (let i = 0; i * 8 < bits.length; i += 1) {
    let byte = 0;
    for (let b = 0; b < 8; b += 1) byte = (byte << 1) | (bits[i * 8 + b] ?? 0);
    out[i] = byte;
  }
  for (let i = bits.length / 8; i < DATA_CODEWORDS; i += 1) {
    out[i] = (i - bits.length / 8) % 2 === 0 ? 0xec : 0x11;
  }
  return out;
}

/**
 * The final bit stream: data and error correction, interleaved block by block
 * the way the specification requires — so that damage to one region of the
 * symbol is spread across blocks instead of destroying one of them.
 */
function stream(text: string): Uint8Array {
  const data = codewords(text);
  const blocks: Uint8Array[] = [];
  const checks: Uint8Array[] = [];
  for (let b = 0; b < BLOCKS; b += 1) {
    const block = data.subarray(b * DATA_PER_BLOCK, (b + 1) * DATA_PER_BLOCK);
    blocks.push(block);
    checks.push(correction(block, EC_PER_BLOCK));
  }

  const ordered: number[] = [];
  for (let i = 0; i < DATA_PER_BLOCK; i += 1) for (const b of blocks) ordered.push(b[i] ?? 0);
  for (let i = 0; i < EC_PER_BLOCK; i += 1) for (const c of checks) ordered.push(c[i] ?? 0);

  const bits = new Uint8Array(ordered.length * 8 + REMAINDER_BITS);
  ordered.forEach((byte, index) => {
    for (let i = 0; i < 8; i += 1) bits[index * 8 + i] = (byte >> (7 - i)) & 1;
  });
  return bits;
}

// --- the symbol ------------------------------------------------------------

/** Modules and whether each one belongs to a function pattern, flat. */
type Grid = { dark: Uint8Array; fixed: Uint8Array };

const at = (grid: Uint8Array, row: number, col: number): boolean =>
  grid[row * SIZE + col] === 1;

function put(grid: Grid, row: number, col: number, dark: boolean): void {
  grid.dark[row * SIZE + col] = dark ? 1 : 0;
  grid.fixed[row * SIZE + col] = 1;
}

/** Finders, separators, timing, alignment, and the one module that is always
 *  dark. A scanner finds the symbol by these, so they are never masked and
 *  never carry data. */
function patterns(grid: Grid): void {
  const finder = (top: number, left: number): void => {
    for (let row = -1; row <= 7; row += 1) {
      for (let col = -1; col <= 7; col += 1) {
        const r = top + row;
        const c = left + col;
        if (r < 0 || r >= SIZE || c < 0 || c >= SIZE) continue;
        const ring = Math.max(Math.abs(row - 3), Math.abs(col - 3));
        put(grid, r, c, ring !== 2 && ring <= 3);
      }
    }
  };
  finder(0, 0);
  finder(0, SIZE - 7);
  finder(SIZE - 7, 0);

  for (let i = 8; i < SIZE - 8; i += 1) {
    put(grid, 6, i, i % 2 === 0);
    put(grid, i, 6, i % 2 === 0);
  }

  for (const row of ALIGNMENT) {
    for (const col of ALIGNMENT) {
      // Asking whether the centre is already spoken for says which three to
      // skip, without a table of exceptions.
      if (grid.fixed[row * SIZE + col] === 1) continue;
      for (let dr = -2; dr <= 2; dr += 1) {
        for (let dc = -2; dc <= 2; dc += 1) {
          put(grid, row + dr, col + dc, Math.max(Math.abs(dr), Math.abs(dc)) !== 1);
        }
      }
    }
  }

  // Always dark, and the reason a symbol is never entirely light.
  put(grid, SIZE - 8, 8, true);

  // Reserve where the format information goes, so the data placement steps
  // over it. The bits themselves are written once a mask is chosen.
  for (let i = 0; i <= 8; i += 1) {
    if (grid.fixed[8 * SIZE + i] !== 1) put(grid, 8, i, false);
    if (grid.fixed[i * SIZE + 8] !== 1) put(grid, i, 8, false);
  }
  for (let i = 0; i < 8; i += 1) {
    if (grid.fixed[8 * SIZE + (SIZE - 1 - i)] !== 1) put(grid, 8, SIZE - 1 - i, false);
    if (grid.fixed[(SIZE - 1 - i) * SIZE + 8] !== 1) put(grid, SIZE - 1 - i, 8, false);
  }
}

/** Two modules wide, bottom to top and back again, skipping the vertical
 *  timing column and everything the function patterns already own. */
function place(grid: Grid, bits: Uint8Array): void {
  let index = 0;
  let upward = true;
  for (let right = SIZE - 1; right >= 1; right -= 2) {
    if (right === 6) right = 5; // the timing column is not a data column
    for (let step = 0; step < SIZE; step += 1) {
      for (let side = 0; side < 2; side += 1) {
        const col = right - side;
        const row = upward ? SIZE - 1 - step : step;
        if (grid.fixed[row * SIZE + col] === 1) continue;
        grid.dark[row * SIZE + col] = bits[index] ?? 0;
        index += 1;
      }
    }
    upward = !upward;
  }
}

const MASKS: ((row: number, col: number) => boolean)[] = [
  (r, c) => (r + c) % 2 === 0,
  (r) => r % 2 === 0,
  (_r, c) => c % 3 === 0,
  (r, c) => (r + c) % 3 === 0,
  (r, c) => (Math.floor(r / 2) + Math.floor(c / 3)) % 2 === 0,
  (r, c) => ((r * c) % 2) + ((r * c) % 3) === 0,
  (r, c) => (((r * c) % 2) + ((r * c) % 3)) % 2 === 0,
  (r, c) => (((r + c) % 2) + ((r * c) % 3)) % 2 === 0,
];

/**
 * The four penalties the specification defines, summed. Their job is to pick
 * the mask that leaves the fewest scanner-confusing shapes — long runs, solid
 * blocks, anything resembling a finder, and an overall bias to dark or light.
 */
function penalty(dark: Uint8Array): number {
  let score = 0;

  const runs = (get: (i: number, j: number) => boolean): void => {
    for (let i = 0; i < SIZE; i += 1) {
      let run = 1;
      for (let j = 1; j < SIZE; j += 1) {
        if (get(i, j) === get(i, j - 1)) {
          run += 1;
          continue;
        }
        if (run >= 5) score += 3 + (run - 5);
        run = 1;
      }
      if (run >= 5) score += 3 + (run - 5);
    }
  };
  runs((i, j) => at(dark, i, j));
  runs((i, j) => at(dark, j, i));

  for (let r = 0; r < SIZE - 1; r += 1) {
    for (let c = 0; c < SIZE - 1; c += 1) {
      const first = at(dark, r, c);
      if (
        first === at(dark, r, c + 1) &&
        first === at(dark, r + 1, c) &&
        first === at(dark, r + 1, c + 1)
      ) {
        score += 3;
      }
    }
  }

  // 1:1:3:1:1 with four light modules beside it — the finder's own signature,
  // which must not appear anywhere else. "Preceded **or** followed", as the
  // specification words it, so a run with clear space on both sides counts
  // once here; implementations that read it as two patterns score it twice.
  // Neither is wrong, and the difference only moves which mask wins — every
  // mask produces a symbol that scans, which is why the check on this file
  // compares against all eight rather than against one (DECISIONS 0047).
  const FINDERISH = [true, false, true, true, true, false, true];
  const finderish = (get: (k: number) => boolean, start: number): boolean => {
    for (let k = 0; k < 7; k += 1) if (get(start + k) !== FINDERISH[k]) return false;
    const before = [-4, -3, -2, -1].every((d) => start + d < 0 || !get(start + d));
    const after = [7, 8, 9, 10].every((d) => start + d >= SIZE || !get(start + d));
    return before || after;
  };
  for (let i = 0; i < SIZE; i += 1) {
    for (let start = 0; start + 7 <= SIZE; start += 1) {
      if (finderish((k) => at(dark, i, k), start)) score += 40;
      if (finderish((k) => at(dark, k, i), start)) score += 40;
    }
  }

  let count = 0;
  for (const module of dark) if (module === 1) count += 1;
  const percent = (count * 100) / (SIZE * SIZE);
  score += Math.floor(Math.abs(percent - 50) / 5) * 10;

  return score;
}

/** The 15 format bits: two of error-correction level, three of mask, ten of
 *  BCH, the whole thing XORed so it is never all zero. */
function formatBits(mask: number): number[] {
  const data = (EC_LEVEL_BITS << 3) | mask;
  let remainder = data << 10;
  for (let i = 4; i >= 0; i -= 1) {
    if (remainder & (1 << (i + 10))) remainder ^= 0x537 << i;
  }
  const value = ((data << 10) | remainder) ^ 0x5412;
  const bits: number[] = [];
  for (let i = 0; i < 15; i += 1) bits.push((value >> i) & 1);
  return bits;
}

function writeFormat(dark: Uint8Array, mask: number): void {
  const bits = formatBits(mask);
  const on = (i: number): number => bits[i] ?? 0;
  const set = (row: number, col: number, value: number): void => {
    dark[row * SIZE + col] = value;
  };
  for (let i = 0; i < 6; i += 1) set(i, 8, on(i));
  set(7, 8, on(6));
  set(8, 8, on(7));
  set(8, 7, on(8));
  for (let i = 9; i < 15; i += 1) set(8, 14 - i, on(i));

  for (let i = 0; i < 8; i += 1) set(8, SIZE - 1 - i, on(i));
  for (let i = 8; i < 15; i += 1) set(SIZE - 15 + i, 8, on(i));
  set(SIZE - 8, 8, 1);
}

/**
 * The symbol for `text`, as rows of modules — `true` is dark.
 *
 * `mask` forces one of the eight patterns instead of scoring them. All eight
 * are valid symbols — the format bits say which was used, and a scanner reads
 * any of them — so which one wins is a quality heuristic and not a fact about
 * the data. Two correct encoders routinely disagree about it, which is why the
 * parameter exists: it is what lets the check compare against a reference
 * implementation without having to share its taste (DECISIONS 0047).
 *
 * No quiet zone: that is four modules of nothing on every side, and it belongs
 * to whoever draws this, because it is padding rather than data.
 */
export function encode(text: string, mask?: number): boolean[][] {
  const grid: Grid = { dark: new Uint8Array(SIZE * SIZE), fixed: new Uint8Array(SIZE * SIZE) };
  patterns(grid);
  place(grid, stream(text));

  const masked = (which: number): Uint8Array => {
    const shape = MASKS[which] ?? MASKS[0]!;
    const candidate = new Uint8Array(grid.dark);
    for (let row = 0; row < SIZE; row += 1) {
      for (let col = 0; col < SIZE; col += 1) {
        const index = row * SIZE + col;
        if (grid.fixed[index] === 1) continue;
        candidate[index] = (grid.dark[index] ?? 0) ^ (shape(row, col) ? 1 : 0);
      }
    }
    writeFormat(candidate, which);
    return candidate;
  };

  let best = masked(mask ?? 0);
  if (mask === undefined) {
    let bestScore = penalty(best);
    for (let which = 1; which < MASKS.length; which += 1) {
      const candidate = masked(which);
      const score = penalty(candidate);
      if (score < bestScore) {
        bestScore = score;
        best = candidate;
      }
    }
  }

  return Array.from({ length: SIZE }, (_row, row) =>
    Array.from({ length: SIZE }, (_col, col) => at(best, row, col)),
  );
}

/** How many masks [`encode`] can be asked for. */
export const MASK_COUNT = MASKS.length;
