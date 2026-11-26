// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    bindReturnedPage,
    captureGoldReturn,
    observedPage,
    decodeTabMarker,
    markBeforeFullRedirect,
    readTabMarker,
    clearTabMarker,
    readDisplayReturn,
    saveDisplayReturn,
    discardDisplayReturn,
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

describe("safe signing resume hints", () => {
    it.each(["", "/show", "/edit"])(
        "round trips exact event and election pages (%s), not other resources",
        (suffix) => {
            const election = tab.page.election_id!
            const eventUrl = "/sequent_backend_election_event/" + tab.event + suffix
            const postUrl = "/sequent_backend_election/" + election + suffix
            expect(
                signingReturnPath(
                    tab.event,
                    observedPage(eventUrl, tab.event, "signatures-requests")!
                )
            ).toBe(eventUrl)
            expect(
                signingReturnPath(tab.event, observedPage(postUrl, tab.event, "publish", election)!)
            ).toBe(postUrl)
            expect(observedPage("/wrong", tab.event, "publish", election)).toBeUndefined()
            expect(observedPage(eventUrl, tab.official, "publish")).toBeUndefined()
            expect(observedPage(postUrl, tab.event, undefined, election)).toBeUndefined()
            expect(
                observedPage(postUrl, tab.event, "signatures-requests", election)
            ).toBeUndefined()
        }
    )

    it("discards corrupt, expired, mismatched or forged display-only hints", () => {
        const data = new Map<string, string>()
        const storage = {
            setItem: (k: string, v: string) => void data.set(k, v),
            getItem: (k: string) => data.get(k),
            removeItem: (k: string) => void data.delete(k),
        } as unknown as Storage
        const path = signingReturnPath(tab.event, tab.page)
        expect(readDisplayReturn(storage, path)).toBeUndefined()
        saveDisplayReturn(storage, {marker: tab, result: "setup-returned", path: "/foreign"})
        expect(data.size).toBe(0)
        saveDisplayReturn(storage, {marker: tab, result: "setup-returned", path})
        expect(readDisplayReturn(storage, path, identity)?.result).toBe("setup-returned")
        expect(readDisplayReturn(storage, path, {...identity, tenant: tab.event})).toBeUndefined()
        expect(readDisplayReturn(storage, "/foreign")).toBeUndefined()
        const key = Array.from(data.keys())[0]
        const healthy = JSON.parse(data.get(key)!)
        for (const patch of [
            {until: 0},
            {until: Date.now() + 130_000},
            {until: "soon"},
            {result: "approve"},
            {marker: {...tab, official: "invalid"}},
        ]) {
            data.set(key, JSON.stringify({...healthy, ...patch}))
            expect(readDisplayReturn(storage, path)).toBeUndefined()
        }
        data.set(key, "malformed")
        expect(readDisplayReturn(storage, path)).toBeUndefined()
        discardDisplayReturn(storage)
        expect(data.size).toBe(0)
    })

    it("doesn't act when a browser cannot retain a valid tab marker", () => {
        const unavailable = {
            getItem: () => {
                throw new Error()
            },
            setItem: () => {
                throw new Error()
            },
            removeItem: () => {
                throw new Error()
            },
        } as unknown as Storage
        expect(decodeTabMarker({...tab, page: {...tab.page, panel: "bad"}})).toBeUndefined()
        expect(
            decodeTabMarker({...tab, page: {...tab.page, href: "https://foreign.invalid/"}})
        ).toBeUndefined()
        expect(
            decodeTabMarker({...tab, tenant: "00000000-0000-0000-0000-000000000000"})
        ).toBeUndefined()
        expect(markBeforeFullRedirect(unavailable, tab)).toBe(false)
        expect(readTabMarker(unavailable)).toBeUndefined()
        expect(() => clearTabMarker(unavailable)).not.toThrow()
        expect(() => discardDisplayReturn(unavailable)).not.toThrow()
    })
})
