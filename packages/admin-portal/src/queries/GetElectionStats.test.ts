// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {buildClientSchema, execute, IntrospectionQuery, validate} from "graphql"
import schemaJson from "../../graphql.schema.json"
import {GetElectionStatsDocument} from "@/gql/graphql"
import {GET_ELECTION_STATS} from "./GetElectionStats"

const schema = buildClientSchema(schemaJson as unknown as IntrospectionQuery)
const variables = {
    tenantId: "tenant",
    electionEventId: "event",
    electionId: "election",
    electionAlias: "election-alias",
    startDate: "2026-09-01",
    endDate: "2026-09-14",
    userTimezone: "UTC",
    timeResolution: "day",
    bucketCount: 14,
}

interface CountBody {
    tenant_id: string
    election_event_id: string
    election_id: string
    authorized_to_election_alias?: string
    enabled?: boolean
}

describe.each([
    ["dashboard query", GET_ELECTION_STATS],
    ["generated query", GetElectionStatsDocument],
] as const)("%s eligible voters", (_, document) => {
    it("excludes disabled voters and reflects enable/disable changes", async () => {
        expect(validate(schema, document)).toEqual([])
        const voters = [{enabled: true}, {enabled: true}, {enabled: false}]
        const countUsers = jest.fn(({body}: {body: CountBody}) => ({
            count: voters.filter(
                (voter) => body.enabled === undefined || voter.enabled === body.enabled
            ).length,
        }))
        const count = async () => {
            const result = await execute({
                schema,
                document,
                variableValues: variables,
                rootValue: {count_users: countUsers, sequent_backend_election: []},
            })
            expect(result.errors).toBeUndefined()
            return result.data?.users
        }

        expect(await count()).toEqual({count: 2})
        expect(countUsers.mock.calls[0][0].body).toEqual({
            tenant_id: variables.tenantId,
            election_event_id: variables.electionEventId,
            election_id: variables.electionId,
            authorized_to_election_alias: variables.electionAlias,
            enabled: true,
        })
        voters[2].enabled = true
        expect(await count()).toEqual({count: 3})
        voters.forEach((voter) => (voter.enabled = false))
        expect(await count()).toEqual({count: 0})
    })

    it("keeps the enabled filter when the election has no alias", async () => {
        const countUsers = jest.fn(() => ({count: 0}))
        const {electionAlias, ...withoutAlias} = variables
        const result = await execute({
            schema,
            document,
            variableValues: withoutAlias,
            rootValue: {count_users: countUsers, sequent_backend_election: []},
        })
        expect(result.errors).toBeUndefined()
        expect(countUsers).toHaveBeenCalledWith(
            {body: expect.objectContaining({enabled: true, election_id: variables.electionId})},
            undefined,
            expect.anything()
        )
    })
})
