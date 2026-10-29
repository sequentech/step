// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ProfileWords} from "@vitrine/ui"

const noun = (one: string, other: string): string =>
    `entry-noun =\n    { $count ->\n        [one] ${one}\n       *[other] ${other}\n    }\n`

/** The election profile's (election/ballots/v1) name for an entry, so that
 * the kit's receipt says "ballot" where its generic words say "entry". */
export const ELECTION_PROFILE_WORDS: ProfileWords = {
    en: noun("ballot", "ballots"),
    ca: noun("vot", "vots"),
    es: noun("papeleta", "papeletas"),
    eu: noun("boto", "boto"),
    fr: noun("bulletin", "bulletins"),
    gl: noun("voto", "votos"),
    nl: noun("stembiljet", "stembiljetten"),
    tl: noun("balota", "balota"),
}
