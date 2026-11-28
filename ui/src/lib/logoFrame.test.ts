import { describe, expect, it } from "vitest";

import { solidFrame } from "./logoFrame";

/** RGBA pixels where `alpha(x, y)` sets each pixel's alpha. */
function pixels(width: number, height: number, alpha: (x: number, y: number) => number) {
  const rgba = new Uint8ClampedArray(width * height * 4);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) rgba[(y * width + x) * 4 + 3] = alpha(x, y);
  }
  return rgba;
}

describe("solidFrame", () => {
  it("finds the solid letters inside their glow, ignoring stray pixels", () => {
    const rgba = pixels(100, 60, (x, y) => {
      if (x >= 30 && x < 70 && y >= 20 && y < 40) return 255;
      if (x >= 10 && x < 90 && y >= 5 && y < 55) return 90;
      return x === 2 && y === 2 ? 255 : 0;
    });
    expect(solidFrame(rgba, 100, 60)).toEqual({
      left: 0.3,
      top: 20 / 60,
      width: 0.4,
      height: 20 / 60,
      aspect: 2,
    });
  });

  it("is the whole image when nothing is solid", () => {
    expect(
      solidFrame(
        pixels(40, 20, () => 50),
        40,
        20,
      ),
    ).toEqual({
      left: 0,
      top: 0,
      width: 1,
      height: 1,
      aspect: 2,
    });
  });
});
