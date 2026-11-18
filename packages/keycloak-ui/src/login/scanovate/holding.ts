// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {ScanovateUpload} from "../KcContext"
import {CaptureProblem} from "./types"

// Keycloak can't take files on its login actions URL, so the page uploads the
// holding photo to the identity verification endpoint first, with the page's
// one-time token, and then submits the form with only the action.
export const CAPTURE_TOKEN_HEADER = "X-Capture-Token"

/** Uploads the photo, and resolves to the problem to show, if any. */
export async function uploadHoldingPhoto(
    upload: ScanovateUpload,
    photo: Blob,
    fetcher: typeof fetch = fetch
): Promise<CaptureProblem | null> {
    let response: Response
    try {
        response = await fetcher(upload.url, {
            method: "PUT",
            headers: {
                [CAPTURE_TOKEN_HEADER]: upload.token,
                "Content-Type": "image/jpeg",
            },
            body: photo,
            credentials: "omit",
        })
    } catch {
        return CaptureProblem.Upload
    }
    if (response.status === 401) return CaptureProblem.InvalidToken
    return response.ok ? null : CaptureProblem.Upload
}

/** The frame the video shows, as a JPEG. */
export function takePhoto(video: HTMLVideoElement): Promise<Blob | null> {
    const canvas = document.createElement("canvas")
    canvas.width = video.videoWidth
    canvas.height = video.videoHeight
    const context = canvas.getContext("2d")
    if (context === null || canvas.width === 0 || canvas.height === 0) {
        return Promise.resolve(null)
    }
    context.drawImage(video, 0, 0, canvas.width, canvas.height)
    return new Promise((resolve) => canvas.toBlob(resolve, "image/jpeg", 0.92))
}
