// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ResultsManifest} from "@/types/results"

const normalizeCss = (css: string | null | undefined): string | null => {
    if (typeof css !== "string") {
        return null
    }

    const trimmed = css.trim()
    return trimmed.length ? css : null
}

export const manifestCustomCss = (
    manifest: ResultsManifest | undefined,
    electionId?: string
): string => {
    const eventCss = normalizeCss(manifest?.custom_css?.election_event)
    const electionCss = electionId
        ? normalizeCss(manifest?.custom_css?.elections?.[electionId])
        : null

    return [eventCss, electionCss].filter(Boolean).join("\n")
}
