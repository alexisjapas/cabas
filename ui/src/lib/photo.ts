/**
 * Turning what a camera hands over into what the core accepts.
 *
 * A canvas is the only image encoder a PWA has, so the downscale and the JPEG
 * live here — but *what is allowed* is the core's word: it refuses anything
 * that is not a JPEG and anything over `maxPhotoBytes()`, and this aims below
 * that rather than promising itself something (DECISIONS 0062, Rule 9).
 *
 * # Why a file input and not `getUserMedia`
 *
 * The same reasoning that made the QR shown and never scanned (DECISIONS
 * 0047). A camera stream in an installed iOS PWA needs a permission prompt, a
 * video element and a live track, and it is historically the first thing to
 * break across iOS versions. `<input type="file" accept="image/*" capture>`
 * is one element the OS answers: it opens the camera, it costs no permission
 * of ours, and it also lets an existing photo be picked, which a stream
 * cannot.
 *
 * # Orientation
 *
 * A phone photo carries its rotation in EXIF rather than in its pixels, so a
 * naive draw puts a landscape shot on its side. `imageOrientation:
 * 'from-image'` is what applies it, and it is the whole reason the bitmap is
 * decoded through `createImageBitmap` rather than through an `<img>`.
 */

import { maxPhotoBytes } from './core';

/**
 * The longest edge a stored photo keeps.
 *
 * Sized for what the photo is for: recognising a product in an aisle or a
 * dish on a shelf, on a phone screen, at arm's length. Every pixel past that
 * is paid on the relay, in a backup, and in the transfer to the other phone.
 */
const MAX_EDGE = 1280;

/** Tried in order until one fits under the ceiling. */
const QUALITIES = [0.82, 0.7, 0.55, 0.4];

/** The last resort when even the lowest quality is too heavy. */
const SMALLER_EDGE = 800;

/**
 * A picked file, downscaled and encoded as a JPEG the core will take.
 *
 * Throws with a message meant to be read next to the button: a file that is
 * not an image, a decoder that refuses it, or — after every fallback — an
 * encode that will not fit.
 */
export async function encodePhoto(file: File): Promise<Uint8Array> {
  const bitmap = await decode(file);
  try {
    const ceiling = await maxPhotoBytes();

    for (const edge of [MAX_EDGE, SMALLER_EDGE]) {
      const canvas = draw(bitmap, edge);
      for (const quality of QUALITIES) {
        const blob = await encode(canvas, quality);
        if (blob.size <= ceiling) {
          return new Uint8Array(await blob.arrayBuffer());
        }
      }
    }
    throw new Error('Photo trop lourde, même réduite.');
  } finally {
    // Frees the decoded pixels now rather than at the next collection: a
    // 12-megapixel bitmap is ~48 MB, and a phone kills the tab for less.
    bitmap.close();
  }
}

async function decode(file: File): Promise<ImageBitmap> {
  if (!file.type.startsWith('image/')) {
    throw new Error("Ce fichier n'est pas une image.");
  }
  try {
    return await createImageBitmap(file, { imageOrientation: 'from-image' });
  } catch {
    // A format the browser cannot decode — a HEIC on a browser that does not
    // read one, a corrupt file. Nothing here can fix it, and saying so is
    // more use than a broken image later.
    throw new Error("Cette image n'a pas pu être lue.");
  }
}

/** The bitmap, scaled so that neither side exceeds `edge`. */
function draw(bitmap: ImageBitmap, edge: number): HTMLCanvasElement {
  const factor = Math.min(1, edge / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement('canvas');
  canvas.width = Math.max(1, Math.round(bitmap.width * factor));
  canvas.height = Math.max(1, Math.round(bitmap.height * factor));

  const context = canvas.getContext('2d');
  if (context === null) throw new Error("Cette image n'a pas pu être traitée.");
  context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  return canvas;
}

function encode(canvas: HTMLCanvasElement, quality: number): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (blob) => {
        if (blob === null) reject(new Error("Cette image n'a pas pu être encodée."));
        else resolve(blob);
      },
      'image/jpeg',
      quality,
    );
  });
}
