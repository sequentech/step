// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {OverridesByLanguage} from "./i18n"
import {
    prepareUploadedEventForExport,
    UploadedElectionEvent,
} from "./uploadedElection"

export interface LocStudioSaveResult {
    presentationI18n: OverridesByLanguage
    electionEventJson?: unknown
}

export const buildLocStudioSaveResult = (
    uploadedEvent: UploadedElectionEvent | null,
    overrides: OverridesByLanguage
): LocStudioSaveResult => {
    if (!uploadedEvent) {
        return {presentationI18n: overrides}
    }
    return {
        presentationI18n: overrides,
        electionEventJson: prepareUploadedEventForExport(uploadedEvent, overrides),
    }
}

const readPresentationI18n = (electionEventJson: unknown): OverridesByLanguage | undefined => {
    if (!electionEventJson || typeof electionEventJson !== "object") {
        return undefined
    }
    const obj = electionEventJson as Record<string, unknown>
    const electionEvent = obj.election_event as Record<string, unknown> | undefined
    const presentation = electionEvent?.presentation as {i18n?: OverridesByLanguage} | undefined
    return presentation?.i18n
}

export const mergePresentationI18n = (
    existing: OverridesByLanguage | undefined,
    incoming: OverridesByLanguage
): OverridesByLanguage => {
    const merged: OverridesByLanguage = {...existing}
    Object.entries(incoming).forEach(([language, values]) => {
        merged[language] = {...merged[language], ...values}
    })
    return merged
}

export const presentationI18nFromSaveResult = (result: LocStudioSaveResult): OverridesByLanguage => {
    const fromExport = result.electionEventJson
        ? readPresentationI18n(result.electionEventJson)
        : undefined
    return mergePresentationI18n(fromExport, result.presentationI18n)
}
