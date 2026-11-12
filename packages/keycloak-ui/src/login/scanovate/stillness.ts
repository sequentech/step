// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The holding photo is taken once the camera image has been still for the
// realm's holding time: small grayscale samples of successive frames are
// compared, and any movement starts the count again.

// Size of the samples the camera image is reduced to.
export const SAMPLE_WIDTH = 32
export const SAMPLE_HEIGHT = 24

// Mean absolute difference (0–255) between two samples below which the image
// counts as still: sensor noise stays under it, a moving hand or face doesn't.
export const STILL_THRESHOLD = 6

export type Stillness = {
    previous: Uint8Array | null
    // When the image became still, or null while it moves.
    stillSince: number | null
}

export const STILL: Stillness = {previous: null, stillSince: null}

/** Luma of each RGBA pixel. */
export function grayscale(rgba: Uint8ClampedArray): Uint8Array {
    const gray = new Uint8Array(rgba.length / 4)
    for (let pixel = 0; pixel < gray.length; pixel++) {
        const i = pixel * 4
        gray[pixel] = Math.round(0.299 * rgba[i] + 0.587 * rgba[i + 1] + 0.114 * rgba[i + 2])
    }
    return gray
}

/** Mean absolute difference of two samples of the same size. */
export function difference(a: Uint8Array, b: Uint8Array): number {
    let total = 0
    for (let i = 0; i < a.length; i++) total += Math.abs(a[i] - b[i])
    return a.length === 0 ? 0 : total / a.length
}

/**
 * Takes the next sample of the camera image: how much of the holding time the
 * image has been still for (0 to 1), and whether it is time to take the photo.
 */
export function trackStillness(
    state: Stillness,
    sample: Uint8Array,
    now: number,
    holdMs: number
): {state: Stillness; progress: number; captured: boolean} {
    const still =
        state.previous !== null &&
        state.previous.length === sample.length &&
        difference(state.previous, sample) < STILL_THRESHOLD
    const stillSince = still ? (state.stillSince ?? now) : null
    const progress = stillSince === null ? 0 : Math.min((now - stillSince) / holdMs, 1)
    return {state: {previous: sample, stillSince}, progress, captured: progress >= 1}
}

/** Whole seconds left of the holding time. */
export function secondsLeft(progress: number, seconds: number): number {
    return Math.max(0, Math.ceil((1 - progress) * seconds - 1e-9))
}
