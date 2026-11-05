/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Who signed" and "Which hubs" (design boards WitnessList and WitnessStates).

import React from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
import axe from "axe-core"
import {ESignerKind, ESignerState, ISignerList, ISignerRow} from "../../services/vitrine/signers"
import {clockSeconds} from "../../services/vitrine/statusRows"
import {WitnessList} from "./WitnessList"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, unknown>) =>
            values ? `${key} ${JSON.stringify(values)}` : key,
    }),
}))

const AT = 1_800_000_009
const SITE = "https://explorer.example/#policy=sha256%3Aab&log=ballots&checkpoint=22&cosigner=w1"

const row = (extra: Partial<ISignerRow> = {}): ISignerRow => ({
    name: "w1",
    party: "observer-a",
    alsoRuns: [],
    state: ESignerState.DONE,
    at: AT,
    ...extra,
})

const witnesses = (rows: ISignerRow[]): ISignerList => ({
    kind: ESignerKind.WITNESSES,
    rows,
    rule: {key: "vitrine.signers.rule.witnessesAlone", values: {signed: 2, total: 3, needed: 2}},
})

const hubs = (rows: ISignerRow[]): ISignerList => ({
    kind: ESignerKind.HUBS,
    rows,
    rule: {key: "vitrine.signers.rule.hubs", values: {count: 2, independent: 1}},
})

const css = (): string =>
    Array.from(document.styleSheets)
        .flatMap((sheet) => Array.from(sheet.cssRules).map((rule) => rule.cssText))
        .join("\n")

const toggle = () => screen.getByRole("button")

// jsdom cannot compute colours, so the contrast rule cannot run here.
const violations = async (container: HTMLElement) =>
    (await axe.run(container, {rules: {"color-contrast": {enabled: false}}})).violations

describe("the toggle", () => {
    it("is a link-like button that starts shut and opens the list in place", () => {
        const {container} = render(<WitnessList list={witnesses([row()])} minutes={10} />)
        expect(toggle().textContent).toBe("vitrine.signers.witnesses.open")
        expect(toggle().getAttribute("aria-expanded")).toBe("false")
        expect(container.querySelector("ul")).toBeNull()
        fireEvent.click(toggle())
        expect(toggle().textContent).toBe("vitrine.signers.witnesses.close")
        expect(toggle().getAttribute("aria-expanded")).toBe("true")
        expect(toggle().getAttribute("aria-controls")).toBe(container.querySelector("ul")?.id)
        fireEvent.click(toggle())
        expect(container.querySelector("ul")).toBeNull()
    })

    it("says which hubs, for the hubs", () => {
        render(<WitnessList list={hubs([row({name: "hub-b"})])} minutes={10} />)
        expect(toggle().textContent).toBe("vitrine.signers.hubs.open")
        fireEvent.click(toggle())
        expect(toggle().textContent).toBe("vitrine.signers.hubs.close")
    })

    it("can start open, and lines up with the step's title through its indent", () => {
        render(<WitnessList list={witnesses([row()])} minutes={10} defaultOpen indent={36} />)
        expect(toggle().getAttribute("aria-expanded")).toBe("true")
        expect(getComputedStyle(toggle()).marginLeft).toBe("30px")
    })

    it("shows nothing without a list", () => {
        const {container} = render(<WitnessList minutes={10} />)
        expect(container.firstChild).toBeNull()
    })
})

describe("the witnesses", () => {
    const open = (rows: ISignerRow[]) =>
        render(<WitnessList list={witnesses(rows)} minutes={10} defaultOpen />)

    it("lists each with its name and who runs it, and no label or badge", () => {
        const {container} = open([row(), row({name: "w2", party: "partner", alsoRuns: ["hub-b"]})])
        const items = container.querySelectorAll("li")
        expect(items).toHaveLength(2)
        expect(items[0].textContent).toContain("w1")
        expect(items[0].textContent).toContain('vitrine.signers.runBy {"party":"observer-a"}')
        expect(items[1].textContent).toContain(
            'vitrine.signers.runByAlso {"party":"partner","hubs":"hub-b"}'
        )
        expect(container.textContent).not.toMatch(/independent|platform badge/i)
    })

    it("says when each signed, Not yet for one that has not, Late for one that is", () => {
        open([
            row(),
            row({name: "w2", state: ESignerState.PENDING, at: undefined}),
            row({name: "w3", state: ESignerState.LATE, at: undefined}),
            row({name: "w4", at: undefined}),
        ])
        const when = Array.from(document.querySelectorAll(".witness-list-when")).map(
            (el) => el.textContent
        )
        expect(when).toEqual([
            `vitrine.signers.signed ${JSON.stringify({time: clockSeconds(AT * 1000)})}`,
            "vitrine.signers.notYet",
            "vitrine.signers.late",
            "vitrine.signers.signedNoTime",
        ])
    })

    it("offers Check on its site only to a row with a site, in a new tab", () => {
        open([row({site: SITE}), row({name: "w2", site: undefined})])
        const links = screen.getAllByRole("link")
        expect(links).toHaveLength(1)
        expect(links[0].getAttribute("href")).toBe(SITE)
        expect(links[0].getAttribute("target")).toBe("_blank")
        expect(links[0].getAttribute("rel")).toBe("noopener noreferrer")
        expect(links[0].textContent).toBe("vitrine.signers.check")
        expect(links[0].getAttribute("aria-label")).toBe(
            'vitrine.signers.checkWitness {"name":"w1"}'
        )
    })

    it("ends with the rule the policy states, and says where the links lead when there are any", () => {
        const {container} = open([row()])
        const foot = container.querySelector(".witness-list-foot")?.textContent
        expect(foot).toBe('vitrine.signers.rule.witnessesAlone {"signed":2,"total":3,"needed":2}')
        const linked = open([row({site: SITE})])
        const feet = linked.container.querySelectorAll(".witness-list-foot")
        expect(feet[0].textContent).toBe(
            'vitrine.signers.rule.witnessesAlone {"signed":2,"total":3,"needed":2} vitrine.signers.linksNote'
        )
    })
})

describe("the hubs", () => {
    it("says when each anchored, how long the others have, and Late", () => {
        render(
            <WitnessList
                minutes={7}
                defaultOpen
                list={hubs([
                    row({name: "hub-b", party: "partner"}),
                    row({name: "hub-c", state: ESignerState.PENDING, at: undefined}),
                    row({name: "hub-d", state: ESignerState.LATE, at: undefined}),
                    row({name: "hub-e", at: undefined}),
                ])}
            />
        )
        const when = Array.from(document.querySelectorAll(".witness-list-when")).map(
            (el) => el.textContent
        )
        expect(when).toEqual([
            `vitrine.signers.anchored ${JSON.stringify({time: clockSeconds(AT * 1000)})}`,
            'vitrine.signers.within {"minutes":7}',
            "vitrine.signers.late",
            "vitrine.signers.anchoredNoTime",
        ])
    })

    it("checks that the hub holds the record, on its own site", () => {
        render(
            <WitnessList minutes={10} defaultOpen list={hubs([row({name: "hub-b", site: SITE})])} />
        )
        expect(screen.getByRole("link").getAttribute("aria-label")).toBe(
            'vitrine.signers.checkHub {"name":"hub-b"}'
        )
    })

    it("lists where browsers check as a hub of its own", () => {
        render(
            <WitnessList
                minutes={10}
                defaultOpen
                list={hubs([
                    row({name: "hub-b"}),
                    {alsoRuns: [], state: ESignerState.PENDING, standIn: true},
                ])}
            />
        )
        const items = screen.getAllByRole("listitem")
        expect(within(items[1]).getByText("vitrine.signers.standIn.name")).toBeTruthy()
        expect(within(items[1]).getByText("vitrine.signers.standIn.who")).toBeTruthy()
    })
})

describe("on narrow lists", () => {
    it("moves the time and the link under Run by, so names keep the width", () => {
        render(<WitnessList list={witnesses([row({site: SITE})])} minutes={10} defaultOpen />)
        expect(css()).toMatch(/@container \(max-width: 360px\)/)
        expect(getComputedStyle(screen.getByRole("list")).containerType).toBe("inline-size")
    })
})

describe("accessibility", () => {
    it("has no axe violations, shut or open", async () => {
        const {container} = render(
            <WitnessList
                minutes={10}
                list={witnesses([
                    row({site: SITE}),
                    row({name: "w2", state: ESignerState.PENDING}),
                ])}
            />
        )
        expect(await violations(container)).toEqual([])
        fireEvent.click(toggle())
        expect(await violations(container)).toEqual([])
    })
})
