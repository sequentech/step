// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** The message the database raises when a write would change a sealed ballot box (VOTE-FREEZE). */
export const BALLOT_BOX_SEALED_ERROR = "ballot_box_sealed"

/** The texts of an error: its message, and what Hasura or Apollo carry along. */
const errorTexts = (error: unknown, seen = new Set<unknown>()): string[] => {
    if (error === null || error === undefined || seen.has(error)) return []
    if (typeof error === "string") return [error]
    if (typeof error !== "object") return []
    seen.add(error)
    const texts: string[] = []
    for (const value of Object.values(error as Record<string, unknown>)) {
        texts.push(...errorTexts(value, seen))
    }
    if (error instanceof Error) texts.push(error.message)
    return texts
}

/**
 * Whether the database refused a write because a ballot box is sealed, e.g.
 * deleting an election with seals. The refusal is matched anywhere in the
 * error: Hasura may put it in the message or in the extensions.
 */
export const isBallotBoxSealedError = (error: unknown): boolean =>
    errorTexts(error).some((text) => text.includes(BALLOT_BOX_SEALED_ERROR))

/**
 * The translation key of a known server text about sealing (VOTE-FREEZE),
 * so the portal shows it in the admin's language; undefined for any other
 * text, which is then shown as it is.
 */
const KNOWN_TEXTS: Array<[RegExp, string]> = [
    // why a seal failed (`failure_reason`)
    [/^a ballot does not match its Ballot ID/i, "dashboard.ballotBoxes.failure.ballotIdMismatch"],
    [/has no content or no Ballot ID/i, "dashboard.ballotBoxes.failure.missingContent"],
    [/can't be read:/i, "dashboard.ballotBoxes.failure.unreadable"],
    [/is still in progress$/i, "dashboard.ballotBoxes.failure.inProgress"],
    [/already on the bulletin board/i, "dashboard.ballotBoxes.failure.alreadyOnBoard"],
    [/has no bulletin board/i, "dashboard.ballotBoxes.failure.noBoard"],
    // refusals of actions
    [/^Voting can't start again/i, "publish.sealRefusals.startAgain"],
    [
        /^This election event has sealed ballot boxes and cannot be deleted/i,
        "sideMenu.menuActions.messages.notification.error.deleteSealedEvent",
    ],
    [
        /can only change before voting opens/i,
        "electionEventScreen.field.ballotBoxSealPolicy.refused",
    ],
    // the hardening migration's refusals (1791000001700)
    [
        /(contest_encryption_policy|delegated_voting_policy|weighted_voting_policy) of election event .* can't change after voting has opened/i,
        "electionEventScreen.field.ballotBoxSealPolicy.settingRefused",
    ],
    [
        /The bulletin board of election event .* can't change after voting has opened/i,
        "electionEventScreen.field.ballotBoxSealPolicy.boardRefused",
    ],
    [/ballot_box_seal_closed_is_final/, "publish.sealRefusals.closedIsFinal"],
]

export const sealTextKey = (text?: string | null): string | undefined =>
    text ? KNOWN_TEXTS.find(([pattern]) => pattern.test(text))?.[1] : undefined

/** A server text about sealing in the admin's language, or as it is when it isn't a known one. */
export const sealText = (t: (key: string) => string, text?: string | null): string => {
    const key = sealTextKey(text)
    return key ? t(key) : (text ?? "")
}

/** A known seal refusal found anywhere in an error, in the admin's language; undefined otherwise. */
export const sealErrorText = (t: (key: string) => string, error: unknown): string | undefined => {
    const key = errorTexts(error)
        .map((text) => sealTextKey(text))
        .find(Boolean)
    return key ? t(key) : undefined
}
