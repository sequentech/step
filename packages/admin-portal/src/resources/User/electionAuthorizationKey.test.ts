// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {electionAuthorizationKey} from "./electionAuthorizationKey"

const id = "b8250fa1-e03f-4ef4-8269-b40f4a5f03ed"

describe("electionAuthorizationKey", () => {
    it("uses the election's alias", () => {
        expect(electionAuthorizationKey({id, alias: "election-a"})).toBe("election-a")
    })

    it("falls back to the election id without an alias", () => {
        expect(electionAuthorizationKey({id, alias: null})).toBe(id)
        expect(electionAuthorizationKey({id})).toBe(id)
        expect(electionAuthorizationKey({id, alias: ""})).toBe(id)
    })
})
