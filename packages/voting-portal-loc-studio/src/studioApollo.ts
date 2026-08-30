// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {ApolloClient, InMemoryCache, createHttpLink} from "@apollo/client"
import {EVotingStatus} from "@sequentech/ui-core"
import {STUDIO_BALLOT_ID, STUDIO_EVENT_ID} from "./studioStore"
import {UploadedElectionEvent} from "./uploadedElection"

interface GraphQLRequestBody {
    operationName?: string
    variables?: {
        ballotId?: string
        electionEventId?: string
        tenantId?: string
        electionIds?: string[]
    }
}

const jsonResponse = (payload: unknown, status = 200): Response =>
    new Response(JSON.stringify(payload), {
        status,
        headers: {"Content-Type": "application/json"},
    })

const openStatus = () => ({
    is_published: true,
    voting_status: EVotingStatus.OPEN,
    kiosk_voting_status: EVotingStatus.NOT_STARTED,
    early_voting_status: EVotingStatus.NOT_STARTED,
})

const dataForOperation = (
    operationName: string,
    variantId: string,
    ballotId: string,
    uploadedEvent: UploadedElectionEvent | null
): Record<string, unknown> => {
    const eventId = uploadedEvent?.electionEventId ?? STUDIO_EVENT_ID
    const tenantId = uploadedEvent?.tenantId ?? "loc-studio-tenant"
    const eventPresentation =
        uploadedEvent?.electionEventPresentations[0] ??
        uploadedEvent?.ballotStyles[0]?.ballot_eml.election_event_presentation ??
        {}

    switch (operationName) {
        case "GetBallotStyles":
            return {sequent_backend_ballot_style: []}
        case "GetElections":
            return {
                sequent_backend_election:
                    uploadedEvent?.ballotStyles.map((ballotStyle) => ({
                        id: ballotStyle.election_id,
                        tenant_id: tenantId,
                        election_event_id: eventId,
                        description: ballotStyle.ballot_eml.description,
                        presentation: ballotStyle.ballot_eml.election_presentation,
                        status: openStatus(),
                    })) ?? [],
            }
        case "GetElectionEvent":
            return {
                sequent_backend_election_event: [
                    {
                        id: eventId,
                        tenant_id: tenantId,
                        presentation: eventPresentation,
                        status: openStatus(),
                    },
                ],
            }
        case "GetCastVotes":
            return {sequent_backend_cast_vote: []}
        case "GetCastVote":
            return {
                sequent_backend_cast_vote:
                    variantId === "found"
                        ? [{ballot_id: ballotId, content: "Encrypted ballot payload"}]
                        : [],
            }
        case "GetSupportMaterials":
            return {sequent_backend_support_material: []}
        case "GetDocument":
            return {sequent_backend_document: []}
        default:
            return {}
    }
}

export const createStudioApollo = (
    sceneId: string,
    variantId: string,
    uploadedEvent: UploadedElectionEvent | null = null
): ApolloClient => {
    const studioFetch: typeof fetch = async (_input, init) => {
        const body = JSON.parse(String(init?.body || "{}")) as GraphQLRequestBody
        const operationName = body.operationName || ""
        const ballotId = body.variables?.ballotId || STUDIO_BALLOT_ID

        if (sceneId === "election-list" && variantId === "errors") {
            return new Response("failed", {status: 500})
        }
        if (operationName === "InsertCastVote" && sceneId === "review" && variantId === "error") {
            return jsonResponse({errors: [{message: "Unable to cast vote"}]})
        }
        return jsonResponse({
            data: dataForOperation(operationName, variantId, ballotId, uploadedEvent),
        })
    }

    return new ApolloClient({
        cache: new InMemoryCache(),
        link: createHttpLink({
            uri: "/loc-studio-graphql",
            fetch: studioFetch,
        }),
        defaultOptions: {
            watchQuery: {fetchPolicy: "no-cache"},
            query: {fetchPolicy: "no-cache"},
        },
    })
}
