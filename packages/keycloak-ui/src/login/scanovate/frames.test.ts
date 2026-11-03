// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {cropPixels} from "./frames"

// A 4 × 3 RGBA image whose red channel numbers the pixels row by row.
function numbered(width: number, height: number) {
    const data = new Uint8ClampedArray(width * height * 4)
    for (let i = 0; i < width * height; i++) {
        data[i * 4] = i
        data[i * 4 + 3] = 255
    }
    return {data, width, height}
}

describe("cropPixels", () => {
    it("copies the rectangle's pixels, row by row", () => {
        const crop = cropPixels(numbered(4, 3), {x: 1, y: 1, width: 2, height: 2})
        expect(crop.width).toBe(2)
        expect(crop.height).toBe(2)
        expect(Array.from(crop.data.filter((_, i) => i % 4 === 0))).toEqual([5, 6, 9, 10])
    })

    it("keeps the whole image for a rectangle covering it", () => {
        const image = numbered(4, 3)
        const crop = cropPixels(image, {x: 0, y: 0, width: 4, height: 3})
        expect(Array.from(crop.data)).toEqual(Array.from(image.data))
    })
})
