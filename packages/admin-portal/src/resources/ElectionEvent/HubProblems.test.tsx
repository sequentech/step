/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The hub problems notice of the Transparency tab (DECISIONS D8).

import React from "react"
import {act, render, screen, within} from "@testing-library/react"
import * as verify from "@vitrine/verify"
import type {Policy} from "@vitrine/verify"
import {policyText} from "@/fixtures/vitrinePolicy"
import {HubProblemsBanner} from "./HubProblems"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
        i18n: {language: "en"},
    }),
}))

const KEYS = "transparencyTab.hubProblems"

let policy: Policy
const fetchMock = jest.fn()

const evidence = (n: number) => `sha256:${n.toString(16).padStart(64, "0")}`
const row = (n: number, kind = "split-view") => ({
    id: evidence(n),
    kind,
    subject: "logs.acme.example/e/ballots",
    policy: policy.id,
    logged_at: 1_800_000_000 + n,
    entry: n,
})
const ok = (problems: unknown[]) =>
    ({
        status: 200,
        headers: new Headers(),
        text: async () => JSON.stringify({hub: "h", problems, more: false}),
    }) as unknown as Response

const renderBanner = async () => {
    const view = render(<HubProblemsBanner policy={policy} />)
    await act(async () => {})
    return view
}

beforeAll(async () => {
    await verify.init()
})

beforeEach(() => {
    jest.clearAllMocks()
    policy = verify.pinPolicy(policyText({hubExplorers: true}))
    ;(globalThis as {fetch: unknown}).fetch = fetchMock
})

describe("the notice", () => {
    it("is silent when no hub lists a problem", async () => {
        fetchMock.mockResolvedValue(ok([]))
        await renderBanner()
        expect(screen.queryByTestId("hub-problems")).toBeNull()
    })

    it("is silent when the hubs do not answer: silence is not a problem", async () => {
        fetchMock.mockRejectedValue(new Error("offline"))
        await renderBanner()
        expect(screen.queryByTestId("hub-problems")).toBeNull()
    })

    it("says what the hubs list, in plain words, with a link to the proof on the hub's site", async () => {
        fetchMock.mockImplementation(async (url: string) =>
            url.startsWith("https://hub1.example/")
                ? ok([row(2, "broken-promise"), row(1)])
                : ok([row(1)])
        )
        await renderBanner()
        const banner = screen.getByTestId("hub-problems")
        expect(banner.getAttribute("role")).toBe("status")
        expect(screen.getByTestId("hub-problems-title").textContent).toBe(
            `${KEYS}.title ${JSON.stringify({count: 2})}`
        )
        const rows = screen.getAllByTestId("hub-problem")
        expect(rows).toHaveLength(2)
        expect(within(rows[0]).getByText(`${KEYS}.kind.brokenPromise`)).toBeTruthy()
        expect(within(rows[1]).getByText(`${KEYS}.kind.splitView`)).toBeTruthy()
        expect(rows[1].textContent).toContain(
            `${KEYS}.listedBy ${JSON.stringify({hubs: "h1, h2"})}`
        )
        const link = within(rows[0]).getByText(`${KEYS}.proof`) as HTMLAnchorElement
        expect(link.getAttribute("href")).toBe(
            `https://explorer1.example/#policy=${encodeURIComponent(policy.id)}&problem=${"0".repeat(63)}2`
        )
        expect(link.getAttribute("rel")).toBe("noopener noreferrer")
        expect(screen.getByText(`${KEYS}.note`)).toBeTruthy()
    })

    it("never shows a hub's code for the kind, nor a link for a hub without a site", async () => {
        policy = verify.pinPolicy(policyText({hubExplorers: false}))
        fetchMock.mockResolvedValue(ok([{...row(1, "brand-new-kind"), policy: policy.id}]))
        await renderBanner()
        const item = screen.getByTestId("hub-problem")
        expect(item.textContent).toContain(`${KEYS}.kind.other`)
        expect(item.textContent).not.toContain("brand-new-kind")
        expect(within(item).queryByRole("link")).toBeNull()
    })

    it("asks each hub only for the sealed policy's problems", async () => {
        fetchMock.mockResolvedValue(ok([]))
        await renderBanner()
        expect(fetchMock).toHaveBeenCalled()
        for (const [url] of fetchMock.mock.calls) {
            expect(url).toContain(`policy=${encodeURIComponent(policy.id)}`)
        }
    })

    it("asks nothing without a policy", async () => {
        render(<HubProblemsBanner policy={undefined} />)
        await act(async () => {})
        expect(fetchMock).not.toHaveBeenCalled()
        expect(screen.queryByTestId("hub-problems")).toBeNull()
    })
})
