// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    bindReturnedPage,
    captureGoldReturn,
    signingReturnPath,
    type ITabMarker,
} from "./operationalGoldReturn"

const tab: ITabMarker = {
    tenant: "00000000-0000-4000-8000-000000000001",
    event: "00000000-0000-4000-8000-000000000002",
    request: "00000000-0000-4000-8000-000000000003",
    official: "00000000-0000-4000-8000-000000000004",
    attempt: "00000000-0000-4000-8000-000000000005",
    page: {
        path: "election-show",
        panel: "publish",
        election_id: "00000000-0000-4000-8000-000000000006",
    },
}
const identity = {tenant: tab.tenant, official: tab.official}

describe("same-tab Gold return", () => {
    it("scrubs credentials and rejects duplicated or query proof", () => {
        const path = {pathname: "/operational-signing/return", search: "", protocol: "https:"}
        const state = "f".repeat(43)
        const clean = jest.fn()
        expect(captureGoldReturn({...path, hash: `#state=${state}&code=v`}, clean)).toEqual({
            state,
            code: "v",
        })
        expect(clean).toHaveBeenCalledWith(path.pathname)
        expect(
            captureGoldReturn({...path, hash: `#state=${state}&code=v&code=w`}, clean)
        ).toBeUndefined()
        expect(
            captureGoldReturn({...path, search: "?code=v", hash: `#state=${state}&code=v`}, clean)
        ).toBeUndefined()
    })

    it("restores only the matching request, actor, tenant, and exact Post", () => {
        expect(bindReturnedPage(tab, tab, identity)).toEqual(tab.page)
        expect(signingReturnPath(tab.event, tab.page)).toBe(
            "/sequent_backend_election/00000000-0000-4000-8000-000000000006/show"
        )
        expect(bindReturnedPage(tab, tab, {...identity, official: tab.event})).toBeUndefined()
        expect(bindReturnedPage(tab, {...tab, request: tab.event}, identity)).toBeUndefined()
        expect(
            bindReturnedPage(tab, {...tab, page: {...tab.page, election_id: tab.event}}, identity)
        ).toBeUndefined()
    })
})
