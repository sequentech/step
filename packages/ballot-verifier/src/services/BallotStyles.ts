// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {GetBallotStylesQuery} from "../gql/graphql"
import {AppDispatch} from "../store/store"
import {isString, IBallotStyle as IElectionDTO} from "@sequentech/ui-core"
import {IBallotStyle, setBallotStyle} from "../store/ballotStyles/ballotStylesSlice"

export enum EPublishedBallotStyleLookup {
    FOUND = "FOUND",
    NOT_LOADED = "NOT_LOADED",
    NOT_PUBLISHED = "NOT_PUBLISHED",
    UNREADABLE = "UNREADABLE",
}

export type PublishedBallotStyleLookup =
    | {status: EPublishedBallotStyleLookup.FOUND; ballotStyle: IElectionDTO}
    | {status: Exclude<EPublishedBallotStyleLookup, EPublishedBallotStyleLookup.FOUND>}

/**
 * Looks up the ballot style with the given id among those served for the
 * given election event. NOT_PUBLISHED is only returned once the ballot styles
 * have loaded and none of that election event has that id; NOT_LOADED and
 * UNREADABLE mean the lookup could not be completed.
 */
export const findPublishedBallotStyle = (
    data: GetBallotStylesQuery | undefined,
    ballotStyleId: string | undefined,
    electionEventId: string | null
): PublishedBallotStyleLookup => {
    if (!data) {
        return {status: EPublishedBallotStyleLookup.NOT_LOADED}
    }
    const ballotStyle = data.sequent_backend_ballot_style.find(
        (style) => style.id === ballotStyleId && style.election_event_id === electionEventId
    )
    if (!ballotStyle) {
        return {status: EPublishedBallotStyleLookup.NOT_PUBLISHED}
    }
    if (!isString(ballotStyle.ballot_eml)) {
        return {status: EPublishedBallotStyleLookup.UNREADABLE}
    }
    try {
        return {
            status: EPublishedBallotStyleLookup.FOUND,
            ballotStyle: JSON.parse(ballotStyle.ballot_eml),
        }
    } catch (error) {
        console.log(`Error loading EML: ${error}`)
        return {status: EPublishedBallotStyleLookup.UNREADABLE}
    }
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
        } catch (error) {
            console.log(`Error loading EML: ${error}`)
            console.log(ballotEml)
            throw error
        }
    }
}
