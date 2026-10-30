// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** What the review screen hands to the voting screen to edit one contest. */
export interface IEditContestState {
    editContestId: string
}

/** The contest a navigation to the voting screen asks to open, if any. */
export const getEditContestId = (state: unknown): string | undefined => {
    if (typeof state !== "object" || state === null) {
        return undefined
    }
    const {editContestId} = state as Partial<Record<keyof IEditContestState, unknown>>
    return typeof editContestId === "string" ? editContestId : undefined
}
