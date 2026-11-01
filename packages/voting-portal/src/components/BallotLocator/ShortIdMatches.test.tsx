/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {MemoryRouter} from "react-router-dom"
import {EShortIdKind, IShortIdMatch, IShortIdResult} from "../../services/vitrine/shortId"
import {ShortIdMatches, ShortIdServerNote} from "./ShortIdMatches"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
    }),
}))

const match = (third: string, extra: Partial<IShortIdMatch> = {}): IShortIdMatch => ({
    ballotId: "ab".repeat(32),
    trackingId: `FTBEMHRX${third}5ZBD${"0".repeat(36)}`,
    third,
    fourth: "5ZBD",
    audited: false,
    at: 1_700_000_000,
    event: "e",
    election: "el",
    onDevice: false,
    ...extra,
})

const result = (extra: Partial<IShortIdResult>): IShortIdResult => ({
    kind: EShortIdKind.LISTED,
    typed: "FTBEMHRX",
    matches: [],
    total: 0,
    truncated: false,
    ...extra,
})

const show = (
    r: IShortIdResult,
    props: Partial<React.ComponentProps<typeof ShortIdMatches>> = {}
) =>
    render(
        <MemoryRouter>
            <ShortIdMatches
                result={r}
                formatTime={(s) => `t${s}`}
                onNarrow={jest.fn()}
                {...props}
            />
        </MemoryRouter>
    )

describe("the ballots that share a short ID", () => {
    it("lists a few, each with its third group highlighted, and asks for the third group", () => {
        show(result({matches: [match("71J7"), match("0WDY", {audited: true})], total: 2}), {
            hrefOf: (m) => `/ballot-locator/${m.trackingId}`,
        })
        expect(screen.getByText(/shortId\.head\.other.*"count":2/)).toBeTruthy()
        expect(screen.getByText("ballotLocator.shortId.sub.few")).toBeTruthy()
        const links = screen.getAllByRole("link")
        expect(links).toHaveLength(2)
        expect(links[0].getAttribute("href")).toContain("FTBEMHRX71J7")
        expect(links[0].querySelector("mark")?.textContent).toBe("71J7")
        expect(screen.getByLabelText("ballotLocator.shortId.thirdOr")).toBeTruthy()
        expect(screen.getByText("ballotLocator.shortId.why.few")).toBeTruthy()
    })

    it("says that the ballot is one when a single one starts so", () => {
        show(result({matches: [match("71J7")], total: 1}), {hrefOf: () => "/x"})
        expect(screen.getByText(/shortId\.head\.one/)).toBeTruthy()
        expect(screen.getByText("ballotLocator.shortId.sub.one")).toBeTruthy()
        expect(screen.queryByText("ballotLocator.shortId.why.few")).toBeNull()
    })

    it("lists nothing when many share it, and only asks for the third group", () => {
        show(result({kind: EShortIdKind.MANY, total: 28}))
        expect(screen.queryAllByRole("link")).toHaveLength(0)
        expect(screen.queryAllByRole("button", {name: /ballotLocator\.shortId\.on/})).toHaveLength(
            0
        )
        expect(screen.getByText("ballotLocator.shortId.sub.many")).toBeTruthy()
        expect(screen.getByLabelText("ballotLocator.shortId.third")).toBeTruthy()
        expect(screen.getByText("ballotLocator.shortId.why.many")).toBeTruthy()
    })

    it("marks the ballot of this device and keeps the look-alikes listed", () => {
        show(
            result({
                matches: [match("71J7", {onDevice: true}), match("0WDY")],
                total: 2,
                mine: match("71J7").trackingId,
            }),
            {hrefOf: (m) => `/${m.trackingId}`}
        )
        expect(screen.getByText(/shortId\.head\.mine.*"count":1/)).toBeTruthy()
        expect(screen.getByText("ballotLocator.shortId.onDevice")).toBeTruthy()
        expect(screen.getByText("ballotLocator.shortId.deviceMatches")).toBeTruthy()
        expect(screen.getAllByRole("link")).toHaveLength(2)
        // The device settles it: the voter is never asked to choose.
        expect(screen.queryByLabelText("ballotLocator.shortId.thirdOr")).toBeNull()
        expect(screen.getByText("ballotLocator.shortId.why.mine")).toBeTruthy()
    })

    it("narrows with the third group the voter types", async () => {
        const onNarrow = jest.fn()
        show(result({kind: EShortIdKind.MANY, total: 28}), {onNarrow})
        const find = screen.getByRole("button", {
            name: "ballotLocator.shortId.find",
        }) as HTMLButtonElement
        expect(find.disabled).toBe(true)
        await userEvent.type(screen.getByLabelText("ballotLocator.shortId.third"), "71j7")
        expect(find.disabled).toBe(false)
        await userEvent.click(find)
        expect(onNarrow).toHaveBeenCalledWith("FTBEMHRX71J7")
    })

    it("opens a ballot by a button where the list does not link", async () => {
        const onOpen = jest.fn()
        show(result({matches: [match("71J7")], total: 1}), {onOpen})
        await userEvent.click(screen.getByRole("button", {name: /FTBE-MHRX/}))
        expect(onOpen).toHaveBeenCalledWith(expect.objectContaining({third: "71J7"}))
    })

    it("tells that the election's server saw the ID", () => {
        render(<ShortIdServerNote />)
        expect(screen.getByText("ballotLocator.shortId.serverSees")).toBeTruthy()
    })
})
