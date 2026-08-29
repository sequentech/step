// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IBallotStyle} from "@sequentech/ui-core"

export const getBallotStyleDefaultLanguageCode = (
    ballotStyle: IBallotStyle | null | undefined
): string | undefined =>
    ballotStyle?.election_presentation?.language_conf?.default_language_code ??
    ballotStyle?.election_event_presentation?.language_conf?.default_language_code
