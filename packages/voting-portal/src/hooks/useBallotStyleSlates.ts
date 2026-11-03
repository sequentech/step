// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useMemo} from "react"
import {
    getBallotStyleSlates,
    getBallotStyleSlatesCoverage,
    ISlatesConfig,
} from "@sequentech/ui-core"

import {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {IBallotSlates, resolveSlates} from "../services/Slates"

export interface IBallotStyleSlates {
    /** The configuration as the ballot style carries it. */
    config: ISlatesConfig | null
    /** The slates with their members looked up in the ballot. */
    resolved: IBallotSlates | null
}

const NO_SLATES: IBallotStyleSlates = {config: null, resolved: null}

/**
 * The slates of a ballot style, read once per ballot style.
 *
 * A ballot style with an invalid slate configuration is refused when it is
 * loaded, so one that fails here is treated as having no slates.
 */
export const useBallotStyleSlates = (ballotStyle: IBallotStyle | undefined): IBallotStyleSlates =>
    useMemo(() => {
        if (!ballotStyle) {
            return NO_SLATES
        }
        try {
            const config = getBallotStyleSlates(ballotStyle.ballot_eml)
            if (!config) {
                return NO_SLATES
            }
            const coverage = getBallotStyleSlatesCoverage(ballotStyle.ballot_eml)
            return {
                config,
                resolved: resolveSlates(config, ballotStyle.ballot_eml.contests, coverage),
            }
        } catch {
            return NO_SLATES
        }
    }, [ballotStyle])
