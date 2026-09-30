// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Keycloak can't take files on its login actions URL, so the page uploads each
// capture to the scanovate-authenticator first, with the one-time token of the
// page, and then submits the form with only the action.
import type {ScanovateUpload} from "../KcContext"
import type {CapturePart, CaptureUpload} from "./form"
import {CaptureProblem} from "./types"

export const UPLOAD_TOKEN_HEADER = "X-Scanovate-Capture"

export enum UploadRejection {
    // Keycloak no longer knows the token: only a new page gets a new one.
    InvalidToken = "INVALID_TOKEN",
    Failed = "FAILED",
}

export class UploadError extends Error {
    constructor(readonly rejection: UploadRejection) {
        super(`Capture upload failed: ${rejection}`)
    }
}

export interface CaptureUploader {
    upload(part: CapturePart, blob: Blob): Promise<void>
}

export type UploadConnector = (settings: ScanovateUpload) => CaptureUploader

export const fetchUploads: UploadConnector = ({url, token}) => {
    const base = url.replace(/\/+$/, "")
    return {
        async upload(part, blob) {
            let response: Response
            try {
                response = await fetch(`${base}/${part}`, {
                    method: "PUT",
                    headers: {
                        [UPLOAD_TOKEN_HEADER]: token,
                        "Content-Type": blob.type.split(";")[0] || "application/octet-stream",
                    },
                    body: blob,
                    credentials: "omit",
                })
            } catch {
                throw new UploadError(UploadRejection.Failed)
            }
            if (response.status === 401) throw new UploadError(UploadRejection.InvalidToken)
            if (!response.ok) throw new UploadError(UploadRejection.Failed)
        },
    }
}

/** Uploads the captures in order, and resolves to the problem to show, if any. */
export async function uploadCaptures(
    uploader: CaptureUploader,
    uploads: CaptureUpload[]
): Promise<CaptureProblem | null> {
    try {
        for (const {part, blob} of uploads) {
            await uploader.upload(part, blob)
        }
        return null
    } catch (error) {
        return error instanceof UploadError && error.rejection === UploadRejection.InvalidToken
            ? CaptureProblem.CaptureExpired
            : CaptureProblem.UploadFailed
    }
}
