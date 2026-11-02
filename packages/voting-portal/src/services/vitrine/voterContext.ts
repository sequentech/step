// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ApolloClient} from "@apollo/client"
import {
    VITRINE_VOTER_CONTEXT,
    VitrineVoterContextQuery,
    VitrineVoterContextQueryVariables,
} from "../../queries/VitrineVoterContext"
import {readVoterContext} from "./presentation"
import type {IVitrineVoterContext} from "./types"

// The context the booth checked, kept for the receipt after the cast: the
// policy and its acknowledgement of private events come only from here.
const checked = new Map<string, IVitrineVoterContext>()

const key = (eventId: string, electionId: string) => `${eventId}/${electionId}`

/** `vitrine_voter_context` from Harvest, never from Apollo's cache: the booth
 * checks what the server says now. Undefined when it can't be fetched. */
export const fetchVoterContext = async (
    client: ApolloClient,
    eventId: string,
    electionId: string
): Promise<IVitrineVoterContext | undefined> => {
    try {
        const {data} = await client.query<
            VitrineVoterContextQuery,
            VitrineVoterContextQueryVariables
        >({
            query: VITRINE_VOTER_CONTEXT,
            variables: {electionEventId: eventId, electionId},
            fetchPolicy: "network-only",
        })
        const context = readVoterContext(data?.vitrine_voter_context)
        if (context) checked.set(key(eventId, electionId), context)
        return context
    } catch {
        return undefined
    }
}

/** The context the booth checked for this election, or a fresh one (after a
 * reload, as on the golden re-authentication path). */
export const voterContextFor = async (
    client: ApolloClient,
    eventId: string,
    electionId: string
): Promise<IVitrineVoterContext | undefined> =>
    checked.get(key(eventId, electionId)) ?? fetchVoterContext(client, eventId, electionId)
