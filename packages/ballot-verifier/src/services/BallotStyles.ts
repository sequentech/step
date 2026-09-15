// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {GetBallotStylesQuery} from "../gql/graphql"
import {AppDispatch} from "../store/store"
import {isString, IBallotStyle as IElectionDTO} from "@sequentech/ui-core"
import {IBallotStyle, setBallotStyle} from "../store/ballotStyles/ballotStylesSlice"

type BallotStyleRow = GetBallotStylesQuery["sequent_backend_ballot_style"][number]

export interface GetPublishedBallotStylesQuery {
    sequent_backend_ballot_publication: Array<{id: string; published_at: string}>
    sequent_backend_ballot_style: Array<BallotStyleRow & {ballot_publication_id: string}>
}

export const updateBallotStyleAndSelection = (
    data: GetPublishedBallotStylesQuery,
    dispatch: AppDispatch
) => {
    const publishedPublications = new Map(
        data.sequent_backend_ballot_publication.map((publication) => [
            publication.id,
            publication.published_at,
        ])
    )
    const publishedBallotStyles = data.sequent_backend_ballot_style
        .filter((ballotStyle) => publishedPublications.has(ballotStyle.ballot_publication_id))
        .sort((left, right) => {
            const publishedAtOrder = publishedPublications
                .get(left.ballot_publication_id)!
                .localeCompare(publishedPublications.get(right.ballot_publication_id)!)
            return (
                publishedAtOrder ||
                left.ballot_publication_id.localeCompare(right.ballot_publication_id)
            )
        })

    for (let ballotStyle of publishedBallotStyles) {
        const ballotEml = ballotStyle.ballot_eml
        if (!isString(ballotEml)) {
            continue
        }
        try {
            const electionData: IElectionDTO = JSON.parse(ballotEml)
            const formattedBallotStyle: IBallotStyle = {
                id: ballotStyle.id,
                ballot_publication_id: ballotStyle.ballot_publication_id,
                publication_published_at: publishedPublications.get(
                    ballotStyle.ballot_publication_id
                )!,
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
