// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {
    STILL,
    STILL_THRESHOLD,
    difference,
    grayscale,
    secondsLeft,
    trackStillness,
} from "./stillness"

const flat = (value: number) => new Uint8Array(16).fill(value)

describe("grayscale", () => {
    it("weighs the channels of each pixel and drops alpha", () => {
        const rgba = new Uint8ClampedArray([
            255, 0, 0, 255, 0, 255, 0, 0, 0, 0, 255, 255, 10, 10, 10, 1,
        ])
        expect([...grayscale(rgba)]).toEqual([76, 150, 29, 10])
    })
})

describe("difference", () => {
    it("is the mean absolute difference of the samples", () => {
        expect(difference(flat(10), flat(10))).toBe(0)
        expect(difference(flat(10), flat(30))).toBe(20)
        expect(difference(new Uint8Array([0, 100]), new Uint8Array([100, 0]))).toBe(100)
    })
})

describe("trackStillness", () => {
    const hold = 2000

    it("needs two frames before it can tell the image is still", () => {
        const first = trackStillness(STILL, flat(50), 0, hold)
        expect(first.progress).toBe(0)
        expect(first.captured).toBe(false)
    })

    it("fills up while the image stays still and captures once the hold time passed", () => {
        let step = trackStillness(STILL, flat(50), 0, hold)
        step = trackStillness(step.state, flat(50), 100, hold)
        expect(step.progress).toBe(0)
        step = trackStillness(step.state, flat(51), 1100, hold)
        expect(step.progress).toBeCloseTo(0.5)
        expect(step.captured).toBe(false)
        step = trackStillness(step.state, flat(50), 2100, hold)
        expect(step.progress).toBe(1)
        expect(step.captured).toBe(true)
    })

    it("starts again when the image moves", () => {
        let step = trackStillness(STILL, flat(50), 0, hold)
        step = trackStillness(step.state, flat(50), 100, hold)
        step = trackStillness(step.state, flat(50), 1500, hold)
        expect(step.progress).toBeGreaterThan(0.5)
        step = trackStillness(step.state, flat(50 + STILL_THRESHOLD), 1600, hold)
        expect(step.progress).toBe(0)
        step = trackStillness(step.state, flat(50 + STILL_THRESHOLD), 1700, hold)
        expect(step.progress).toBe(0)
        step = trackStillness(step.state, flat(50 + STILL_THRESHOLD), 3600, hold)
        expect(step.captured).toBe(false)
        step = trackStillness(step.state, flat(50 + STILL_THRESHOLD), 3700, hold)
        expect(step.captured).toBe(true)
    })

    it("treats a change of size as movement", () => {
        const step = trackStillness(STILL, flat(50), 0, hold)
        const resized = trackStillness(step.state, new Uint8Array(8).fill(50), 100, hold)
        expect(resized.state.stillSince).toBeNull()
    })
})

describe("secondsLeft", () => {
    it("counts down whole seconds of the hold time", () => {
        expect(secondsLeft(0, 5)).toBe(5)
        expect(secondsLeft(0.5, 5)).toBe(3)
        expect(secondsLeft(0.99, 5)).toBe(1)
        expect(secondsLeft(1, 5)).toBe(0)
    })
})
