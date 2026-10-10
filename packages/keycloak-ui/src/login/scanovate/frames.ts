// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {fitSize} from "./geometry"
import type {Size} from "./types"

export const DOCUMENT_ANALYSIS_SIDE = 480
export const FACE_ANALYSIS_SIDE = 640
export const STILL_SIDE = 1920
// The authenticator accepts photos up to 2 MiB.
export const STILL_MAX_BYTES = 1.8 * 1024 * 1024
export const STILL_QUALITIES = [0.92, 0.85, 0.78, 0.7, 0.6]

export function videoSize(video: HTMLVideoElement): Size | null {
    return video.videoWidth > 0 && video.videoHeight > 0
        ? {width: video.videoWidth, height: video.videoHeight}
        : null
}

// Downscales video frames into one reusable canvas for analysis.
export class FrameSampler {
    private canvas = document.createElement("canvas")
    private context = this.canvas.getContext("2d", {willReadFrequently: true})

    sample(video: HTMLVideoElement, longSide: number): ImageData | null {
        const size = videoSize(video)
        if (size === null || this.context === null) return null
        const {width, height} = fitSize(size, longSide)
        if (this.canvas.width !== width || this.canvas.height !== height) {
            this.canvas.width = width
            this.canvas.height = height
        }
        this.context.drawImage(video, 0, 0, width, height)
        return this.context.getImageData(0, 0, width, height)
    }
}

function encode(canvas: HTMLCanvasElement, quality: number): Promise<Blob> {
    return new Promise<Blob>((resolve, reject) => {
        canvas.toBlob(
            (blob) => (blob === null ? reject(new Error("JPEG encoding failed")) : resolve(blob)),
            "image/jpeg",
            quality
        )
    })
}

/** The current frame, at most 1920 px long, as it is uploaded. */
export function drawStill(video: HTMLVideoElement): ImageData {
    const size = videoSize(video)
    if (size === null) {
        throw new Error("The camera has no frame yet")
    }
    const {width, height} = fitSize(size, STILL_SIDE)
    const canvas = document.createElement("canvas")
    canvas.width = width
    canvas.height = height
    const context = canvas.getContext("2d", {willReadFrequently: true})
    if (context === null) {
        throw new Error("Canvas 2D is not available")
    }
    context.drawImage(video, 0, 0, width, height)
    return context.getImageData(0, 0, width, height)
}

/** A JPEG of the current frame, at most 1920 px long and within the upload limit. */
export async function grabStill(video: HTMLVideoElement): Promise<Blob> {
    return encodeStill(drawStill(video))
}

/** A JPEG of a still within the upload limit. */
export async function encodeStill(still: ImageData): Promise<Blob> {
    const canvas = document.createElement("canvas")
    canvas.width = still.width
    canvas.height = still.height
    const context = canvas.getContext("2d")
    if (context === null) {
        throw new Error("Canvas 2D is not available")
    }
    context.putImageData(still, 0, 0)
    let blob: Blob | null = null
    for (const quality of STILL_QUALITIES) {
        blob = await encode(canvas, quality)
        if (blob.size <= STILL_MAX_BYTES) return blob
    }
    throw new Error(`The photo is too large: ${blob?.size ?? 0} bytes`)
}
