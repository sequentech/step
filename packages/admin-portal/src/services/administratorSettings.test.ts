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
