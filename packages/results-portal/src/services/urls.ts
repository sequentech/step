// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const joinUrl = (base: string, path: string): string => {
    if (/^https?:\/\//i.test(path)) {
        return path
    }

    return `${base.replace(/\/+$/, "")}/${path.replace(/^\/+/, "")}`
}

export const publicBucketUrl = (publicBucketBaseUrl: string, path?: string): string | undefined => {
    if (!path) {
        return undefined
    }

    return joinUrl(publicBucketBaseUrl, path)
}
