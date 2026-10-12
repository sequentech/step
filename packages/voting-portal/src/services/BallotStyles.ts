// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {GetBallotStylesQuery} from "../gql/graphql"
import {AppDispatch} from "../store/store"
import {isString, IBallotStyle as IElectionDTO} from "@sequentech/ui-core"
import {IBallotStyle, setBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {resetBallotSelection} from "../store/ballotSelections/ballotSelectionsSlice"
import {checkIsExplicitBlankVote, checkIsInvalidVote} from "./ElectionConfigService"

export class BallotStyleConfigurationError extends Error {
    translationKey: string
    translationParams: Record<string, string | number>

    constructor(translationKey: string, translationParams: Record<string, string | number>) {
        super(translationKey)
        this.name = "BallotStyleConfigurationError"
        this.translationKey = translationKey
        this.translationParams = translationParams
    }
}

export const getBallotStyleConfigurationError = (
    ballotStyle: IElectionDTO
): BallotStyleConfigurationError | undefined => {
    for (let contest of ballotStyle.contests) {
        const explicitInvalidCount = contest.candidates.filter(checkIsInvalidVote).length
        if (explicitInvalidCount > 1) {
            return new BallotStyleConfigurationError(
                "errors.configuration.multipleExplicitInvalidCandidates",
                {count: explicitInvalidCount}
            )
        }

        const explicitBlankCount = contest.candidates.filter(checkIsExplicitBlankVote).length
        if (explicitBlankCount > 1) {
            return new BallotStyleConfigurationError(
                "errors.configuration.multipleExplicitBlankCandidates",
                {count: explicitBlankCount}
            )
        }
    }

    return undefined
}

export const updateBallotStyleAndSelection = (
    data: GetBallotStylesQuery,
    dispatch: AppDispatch
) => {
    for (let ballotStyle of data.sequent_backend_ballot_style) {
        const ballotEml = ballotStyle.ballot_eml
        if (!isString(ballotEml)) {
            continue
        }
        try {
            const electionData: IElectionDTO = JSON.parse(ballotEml)
            const configurationError = getBallotStyleConfigurationError(electionData)
            if (configurationError) {
                throw configurationError
            }
            const formattedBallotStyle: IBallotStyle = {
                id: ballotStyle.id,
                election_id: ballotStyle.election_id,
                election_event_id: ballotStyle.election_event_id,
                tenant_id: ballotStyle.tenant_id,
                ballot_eml: electionData,
                ballot_signature: ballotStyle.ballot_signature,
                created_at: ballotStyle.created_at,
                area_id: ballotStyle.area_id,
                annotations: ballotStyle.annotations,
                labels: ballotStyle.labels,
                last_updated_at: ballotStyle.last_updated_at,
            }
            dispatch(setBallotStyle(formattedBallotStyle))
            dispatch(
                resetBallotSelection({
                    ballotStyle: formattedBallotStyle,
                    force: true,
                })
            )
        } catch (error) {
            console.log(`Error loading EML: ${error}`)
            console.log(ballotEml)
            throw error
        }
    }
}
