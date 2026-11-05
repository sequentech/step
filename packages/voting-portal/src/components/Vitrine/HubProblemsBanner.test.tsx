/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen} from "@testing-library/react"
import type {Policy} from "@vitrine/verify"
import type {IHubProblems} from "../../services/vitrine/hubProblems"
import {HubProblemsBanner} from "./HubProblemsBanner"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
    }),
}))

let mockFound: IHubProblems | undefined
const mockEnabled = jest.fn()
jest.mock("../../hooks/useHubProblems", () => ({
    useHubProblems: (_policy: unknown, enabled: boolean) => {
        mockEnabled(enabled)
        return mockFound
    },
}))
jest.mock("../../services/vitrine/hubUpdates", () => ({
    offeredHubs: () => [
        {name: "hub-a", party: "Sequent", site: "https://hub-a.example.org/"},
        {name: "hub-b", party: "Observer", site: "https://hub-b.example.org/"},
    ],
}))

const POLICY = {id: `sha256:${"a".repeat(64)}`, hubs: new Map([["hub-a", {}]])} as unknown as Policy
const problem = (n: number, kind: string, hubs: string[]) => ({
    id: `sha256:${n.toString(16).padStart(64, "0")}`,
    kind,
    loggedAt: 1_793_270_000,
    entry: n,
    hubs,
})

beforeEach(() => {
    mockFound = undefined
    mockEnabled.mockReset()
})

describe("the notice of problems that hubs list", () => {
    it("shows nothing before the hubs answer, or when they list none", () => {
        const {container, rerender} = render(<HubProblemsBanner policy={POLICY} />)
        expect(container).toBeEmptyDOMElement()
        mockFound = {problems: [], unreachable: ["hub-b"]}
        rerender(<HubProblemsBanner policy={POLICY} />)
        expect(container).toBeEmptyDOMElement()
    })

    it("names each problem in plain words, with the hubs that list it and a link to the proof", () => {
        mockFound = {
            problems: [
                problem(2, "split-view", ["hub-a", "hub-b"]),
                problem(1, "unlogged-code", ["hub-b"]),
            ],
            unreachable: [],
        }
        render(<HubProblemsBanner policy={POLICY} />)
        expect(screen.getByText(/hubProblems\.title\.other.*"count":2/)).toBeTruthy()
        expect(screen.getByText("vitrine.hubProblems.kind.splitView")).toBeTruthy()
        expect(screen.getByText("vitrine.hubProblems.kind.unloggedCode")).toBeTruthy()
        expect(screen.getByText(/listedBy.*hub-a, hub-b/)).toBeTruthy()
        const links = screen.getAllByRole("link", {name: "vitrine.hubProblems.proof"})
        expect(links).toHaveLength(2)
        expect(links[0].getAttribute("href")).toBe(
            `https://hub-a.example.org/#policy=sha256%3A${"a".repeat(64)}&problem=${"0".repeat(63)}2`
        )
        expect(links[1].getAttribute("href")).toContain("https://hub-b.example.org/#")
    })

    it("says that it is a convenience, and that the hubs say it without this site", () => {
        mockFound = {problems: [problem(1, "broken-promise", ["hub-a"])], unreachable: []}
        render(<HubProblemsBanner policy={POLICY} />)
        expect(screen.getByText(/hubProblems\.title\.one/)).toBeTruthy()
        expect(screen.getByText("vitrine.hubProblems.note")).toBeTruthy()
    })

    it("asks the hubs only while it is enabled", () => {
        render(<HubProblemsBanner policy={POLICY} enabled={false} />)
        expect(mockEnabled).toHaveBeenCalledWith(false)
    })
})
