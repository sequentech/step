/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import {BALLOT_BOXES_VISIBLE_KEY, BallotBoxesSection} from "./BallotBoxesSection"

let mockPolicy: string | undefined = "seal-at-close"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))
jest.mock("@/hooks/useZonedFormat", () => ({
    useEventPresentation: () => ({ballot_box_seal_policy: mockPolicy}),
}))
jest.mock("./BallotBoxesCard", () => ({
    BallotBoxesCard: () => require("react").createElement("section", {"aria-label": "card"}),
}))

const SWITCH = "dashboard.ballotBoxes.show"

const renderSection = () =>
    render(<BallotBoxesSection electionEventId="event" electionId="election" />)

const toggle = () => screen.getByRole("switch", {name: SWITCH}) as HTMLInputElement

const card = () => screen.queryByRole("region", {name: "card"})

beforeEach(() => {
    mockPolicy = "seal-at-close"
    window.localStorage.clear()
})
afterEach(() => jest.restoreAllMocks())

describe("BallotBoxesSection", () => {
    it("shows the ballot boxes by default", () => {
        renderSection()
        expect(toggle().checked).toBe(true)
        expect(card()).not.toBeNull()
    })

    it("hides the ballot boxes after switching off", () => {
        renderSection()
        fireEvent.click(toggle())
        expect(toggle().checked).toBe(false)
        expect(card()).toBeNull()
    })

    it("restores the choice on remount", () => {
        const first = renderSection()
        fireEvent.click(toggle())
        first.unmount()
        renderSection()
        expect(toggle().checked).toBe(false)
        expect(card()).toBeNull()
        expect(window.localStorage.getItem(BALLOT_BOXES_VISIBLE_KEY)).not.toBeNull()
    })

    it("shows the ballot boxes again after switching back on", () => {
        renderSection()
        fireEvent.click(toggle())
        fireEvent.click(toggle())
        expect(card()).not.toBeNull()
    })

    it("shows the ballot boxes for an unknown stored value", () => {
        window.localStorage.setItem(BALLOT_BOXES_VISIBLE_KEY, "garbage")
        renderSection()
        expect(toggle().checked).toBe(true)
        expect(card()).not.toBeNull()
    })

    it("renders no switch and no card when the event doesn't seal", () => {
        mockPolicy = "no-seal"
        renderSection()
        expect(screen.queryByRole("switch")).toBeNull()
        expect(card()).toBeNull()
    })

    it("still shows the ballot boxes when the storage throws", () => {
        jest.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
            throw new Error("blocked")
        })
        jest.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
            throw new Error("blocked")
        })
        renderSection()
        expect(toggle().checked).toBe(true)
        expect(card()).not.toBeNull()
        // The switch still works for this view, without remembering it.
        fireEvent.click(toggle())
        expect(card()).toBeNull()
    })

    it("still shows the ballot boxes when the storage can't be reached", () => {
        jest.spyOn(window, "localStorage", "get").mockImplementation(() => {
            throw new Error("SecurityError")
        })
        renderSection()
        expect(toggle().checked).toBe(true)
        expect(card()).not.toBeNull()
    })
})
