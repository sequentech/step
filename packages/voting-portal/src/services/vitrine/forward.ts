// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ApolloClient} from "@apollo/client"
import {
    VITRINE_VOTER_FORWARD,
    VitrineVoterForwardMutation,
    VitrineVoterForwardMutationVariables,
} from "../../queries/VitrineVoterForward"
import type {TForward} from "./castReceipt"

/** Harvest's `vitrine_voter_forward` for the event (private events). An
 * answer that didn't come reads as status 0. */
export const voterForward =
    (client: ApolloClient, eventId: string): TForward =>
    async (route, body, hub) => {
        try {
            const {data} = await client.mutate<
                VitrineVoterForwardMutation,
                VitrineVoterForwardMutationVariables
            >({
                mutation: VITRINE_VOTER_FORWARD,
                variables: {electionEventId: eventId, route, body, ...(hub ? {hub} : {})},
            })
            const answer = data?.vitrine_voter_forward
            return answer
                ? {status: answer.status, ...(answer.body ? {body: answer.body} : {})}
                : {status: 0}
        } catch {
            return {status: 0}
        }
    }

