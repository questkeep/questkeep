/**
 * Where a logo's solid artwork sits in its image, as shares of the image's width and height, with
 * the solid part's own aspect ratio. The game page sizes the logo by this part and lets glow,
 * shadow and transparent margins spill around it. Found from the pixels in the webview, so it
 * works for every format the webview shows (PNG, WebP, AVIF, GIF).
 */
export interface LogoFrame {
  left: number;
  top: number;
  width: number;
  height: number;
  /** Width / height of the solid part in pixels. */
  aspect: number;
}

/** Alpha from which a pixel counts as solid artwork rather than glow or shadow. */
const SOLID_ALPHA = 160;
/** Logos are measured at most this wide or tall; the frame is a share, so that is plenty. */
const MEASURE_SIDE = 400;

/** First and last+1 index with enough solid pixels (a share of the other side, at least 2). */
function span(counts: Uint32Array, otherSide: number): [number, number] | null {
  const needed = Math.max(2, Math.floor(otherSide / 200));
  const first = counts.findIndex((count) => count >= needed);
  const last = counts.findLastIndex((count) => count >= needed);
  return first < 0 ? null : [first, last + 1];
}

/**
 * The solid frame of RGBA pixels: the rows and columns with enough pixels of at least
 * {@link SOLID_ALPHA} (a few stray ones do not count). All frame when nothing is solid.
 */
export function solidFrame(rgba: ArrayLike<number>, width: number, height: number): LogoFrame {
  const columns = new Uint32Array(width);
  const rows = new Uint32Array(height);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if ((rgba[(y * width + x) * 4 + 3] ?? 0) >= SOLID_ALPHA) {
        columns[x] = (columns[x] ?? 0) + 1;
        rows[y] = (rows[y] ?? 0) + 1;
      }
    }
  }
  const horizontal = span(columns, height);
  const vertical = span(rows, width);
  if (!horizontal || !vertical || width === 0 || height === 0) {
    return { left: 0, top: 0, width: 1, height: 1, aspect: width / Math.max(1, height) };
  }
  const [left, right] = horizontal;
  const [top, bottom] = vertical;
  return {
    left: left / width,
    top: top / height,
    width: (right - left) / width,
    height: (bottom - top) / height,
    aspect: (right - left) / (bottom - top),
  };
}

const frames = new Map<string, Promise<LogoFrame | null>>();

/**
 * The frame of the logo at `url` (remembered per URL). `null` when the image cannot be read: it
 * failed to load, or its pixels are not readable.
 */
export function logoFrame(url: string): Promise<LogoFrame | null> {
  let frame = frames.get(url);
  if (!frame) {
    frame = measure(url).catch(() => null);
    frames.set(url, frame);
  }
  return frame;
}

async function measure(url: string): Promise<LogoFrame | null> {
  const image = new Image();
  // The asset protocol allows the window's origin, so the canvas stays readable.
  image.crossOrigin = "anonymous";
  image.src = url;
  await image.decode();
  const { naturalWidth: w, naturalHeight: h } = image;
  if (w === 0 || h === 0) return null;
  const scale = Math.min(1, MEASURE_SIDE / Math.max(w, h));
  const width = Math.max(1, Math.round(w * scale));
  const height = Math.max(1, Math.round(h * scale));
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) return null;
  context.drawImage(image, 0, 0, width, height);
  const frame = solidFrame(context.getImageData(0, 0, width, height).data, width, height);
  // The aspect from the scaled pixels, corrected to the real image's.
  return { ...frame, aspect: (frame.width * w) / (frame.height * h) };
}
