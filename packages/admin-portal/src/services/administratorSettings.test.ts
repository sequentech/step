// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {parse, print} from "graphql"
import {administratorSettingsQuery} from "./administratorSettings"

describe("administrator settings queries", () => {
    it("keeps settings navigation but never asks for execution or ballot data", () => {
        const query = administratorSettingsQuery(
            parse(`query {sequent_backend_election_event {
            id tenant_id presentation statistics status public_key elections {id presentation}
        }}`)
        )
        expect(print(query)).toContain("presentation: administration_settings")
        for (const hidden of ["statistics", "status", "public_key", "elections"]) {
            expect(print(query)).not.toContain(hidden)
        }
        expect(print(query)).toContain("tenant_id")
    })
    it("preserves settings mutation input and aliases the permitted response", () => {
        const query = administratorSettingsQuery(
            parse(`mutation {update_sequent_backend_election_event(
            where:{id:{_eq:"event"}},_set:{presentation:{otp:{policy:"required"}}}) {
            affected_rows returning {id administration_settings}
        }}`)
        )
        expect(print(query)).toContain("_set: {presentation:")
        expect(print(query)).toContain("presentation: administration_settings")
        expect(print(query)).toContain("affected_rows")
    })
    it("leaves identity and signing actions unchanged", () => {
        const query = parse(`query {get_roles(tenant_id:"tenant") {items{id name}}}`)
        expect(print(administratorSettingsQuery(query))).toBe(print(query))
    })
})

it("normalizes actual list ordering and projection name filters without weakening unknown predicates", () => {
    const variables = {
        order_by: {created_at: "desc"},
        where: {
            _and: [
                {tenant_id: {_eq: "tenant"}},
                {
                    _or: [
                        {external_id: {_ilike: "%Context%"}},
                        {presentation: {_cast: {String: {_ilike: "%Context%"}}}},
                    ],
                },
            ],
        },
    }
    const query =
        parse(`query List($order_by:[sequent_backend_election_event_order_by!],$where:sequent_backend_election_event_bool_exp) {
      items:sequent_backend_election_event(order_by:$order_by,where:$where){id presentation}
    }`)
    administratorSettingsQuery(query, variables)
    expect(variables.order_by).toEqual({id: "asc"})
    expect(variables.where._and[1]._or).toEqual([
        {id: {_is_null: true}},
        {administration_settings: {_cast: {String: {_ilike: "%Context%"}}}},
    ])
})

it("projects fragments and drops variables used only by removed nested fields", () => {
    const query = administratorSettingsQuery(
        parse(`query List($secret:String,$tenant:uuid){
      sequent_backend_election_event(where:{tenant_id:{_eq:$tenant}}){...EventFields}
    }
    fragment EventFields on sequent_backend_election_event {
      id presentation elections(where:{description:{_eq:$secret}}){id}
    }`)
    )
    expect(print(query)).toContain("presentation: administration_settings")
    expect(print(query)).not.toContain("$secret")
    expect(print(query)).not.toContain("elections")
    expect(print(query)).toContain("$tenant")
})
