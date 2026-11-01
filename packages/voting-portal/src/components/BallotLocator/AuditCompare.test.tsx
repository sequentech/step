/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The check at the end of a ballot audit (design boards AuditCompare and
// PhoneTrackerAudit): the purpose, the three steps, the question with two
// answers that look alike, what each answer leads to, and the FAQ.

import React from "react"
import {act, render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import axe from "axe-core"
import {MemoryRouter} from "react-router-dom"
import {EContactSource, IAuthorityContact} from "../../services/vitrine/auditEvidence"
import {AuditCompare, IAuditCompareProps} from "./AuditCompare"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, unknown>) =>
            values ? `${key} ${JSON.stringify(values)}` : key,
    }),
}))

const BALLOT_ID = "FTBEMHRX0123456789ABCDEFGHJKMNPQRSTVWXYZ0123456789AB"
const VOTE_HREF = "/tenant/t/event/e/election/x/start?lang=en"

const BOARD: IAuthorityContact = {
    name: "Electoral board",
    email: "board@example.org",
    phone: "+34 900 000 000",
    source: EContactSource.POLICY,
}

const renderCompare = (props: Partial<IAuditCompareProps> = {}) => {
    const onSaveEvidence = jest.fn()
    const view = render(
        <MemoryRouter>
            <AuditCompare
                ballotId={BALLOT_ID}
                voteHref={VOTE_HREF}
                onSaveEvidence={onSaveEvidence}
                {...props}
            >
                <p className="decoded-choices">Name of puig</p>
            </AuditCompare>
        </MemoryRouter>
    )
    return {...view, onSaveEvidence}
}

const css = (): string =>
    Array.from(document.styleSheets)
        .flatMap((sheet) => Array.from(sheet.cssRules).map((rule) => rule.cssText))
        .join("\n")

const answer = (name: "yes" | "no") =>
    screen.getByRole("button", {name: `ballotLocator.audit.${name}`})

const steps = () => Array.from(document.querySelectorAll(".audit-compare-step"))
const stateOf = (n: number) => steps()[n].getAttribute("data-state")

// jsdom cannot compute colours, so the contrast rule cannot run here.
const violations = async (container: HTMLElement) =>
    (await axe.run(container, {rules: {"color-contrast": {enabled: false}}})).violations

describe("the card", () => {
    it("is a region named by its title, which says what the voter is checking", () => {
        renderCompare()
        const card = screen.getByRole("region", {name: "ballotLocator.audit.title"})
        expect(within(card).getByRole("heading", {level: 2})).toHaveTextContent(
            "ballotLocator.audit.title"
        )
        expect(within(card).getByText("ballotLocator.audit.kicker")).toBeVisible()
    })

    it("says in a line why only the voter can do the check", () => {
        const {container} = renderCompare()
        expect(container.querySelector(".audit-compare-purpose")).toHaveTextContent(
            "ballotLocator.audit.note"
        )
    })

    it("holds the decoded choices in a group of their own", () => {
        renderCompare()
        const group = screen.getByRole("group", {name: "ballotLocator.audit.choices"})
        expect(within(group).getByText("Name of puig")).toBeVisible()
    })

    it("prints the page, from the Print button", async () => {
        const print = jest.spyOn(window, "print").mockImplementation(() => undefined)
        renderCompare()
        await userEvent.click(screen.getByRole("button", {name: "auditScreen.printButton"}))
        expect(print).toHaveBeenCalledTimes(1)
        print.mockRestore()
    })
})

describe("the three steps", () => {
    it("are encrypted, decoded and compare, in that order, under a label", () => {
        const {container} = renderCompare()
        const list = screen.getByRole("list", {name: "ballotLocator.audit.steps.label"})
        const titles = Array.from(container.querySelectorAll(".audit-compare-step-title")).map(
            (title) => title.textContent
        )
        expect(within(list).getAllByRole("listitem")).toHaveLength(3)
        expect(titles).toEqual([
            "ballotLocator.audit.steps.encrypted.title",
            "ballotLocator.audit.steps.decoded.title",
            "ballotLocator.audit.steps.compare.title",
        ])
        const texts = Array.from(container.querySelectorAll(".audit-compare-step-text")).map(
            (text) => text.textContent
        )
        expect(texts).toEqual([
            "ballotLocator.audit.steps.encrypted.text",
            "ballotLocator.audit.steps.decoded.text",
            "ballotLocator.audit.steps.compare.text",
        ])
    })

    it("have the first two done and the voter's comparison current, until the voter answers", () => {
        renderCompare()
        expect([stateOf(0), stateOf(1), stateOf(2)]).toEqual(["done", "done", "current"])
        expect(steps()[2].getAttribute("aria-current")).toBe("step")
        expect(steps()[0].getAttribute("aria-current")).toBeNull()
    })

    it("finish when the voter says the choices match", async () => {
        renderCompare()
        await userEvent.click(answer("yes"))
        expect([stateOf(0), stateOf(1), stateOf(2)]).toEqual(["done", "done", "done"])
        expect(steps()[2].getAttribute("aria-current")).toBeNull()
    })

    it("mark the comparison as the problem when the voter says they do not", async () => {
        renderCompare()
        await userEvent.click(answer("no"))
        expect([stateOf(0), stateOf(1), stateOf(2)]).toEqual(["done", "done", "problem"])
    })
})

describe("the question", () => {
    it("is asked once, with two answers in a group the question names", () => {
        renderCompare()
        expect(screen.getByText("ballotLocator.audit.question")).toBeVisible()
        const group = screen.getByRole("group", {name: "ballotLocator.audit.question"})
        expect(within(group).getAllByRole("button")).toHaveLength(2)
    })

    it("gives both answers the same look, so neither is the easy one", () => {
        renderCompare()
        const yes = getComputedStyle(answer("yes"))
        const no = getComputedStyle(answer("no"))
        for (const property of [
            "minHeight",
            "padding",
            "border",
            "borderRadius",
            "backgroundColor",
            "color",
            "fontSize",
            "fontWeight",
        ] as const) {
            expect(yes[property]).toBeTruthy()
            expect(yes[property]).toBe(no[property])
        }
        expect(document.querySelector(".audit-compare-yes")?.className).not.toMatch(
            /primary|secondary|danger/
        )
    })

    it("gives way to the panel of the answer", async () => {
        renderCompare()
        await userEvent.click(answer("yes"))
        expect(screen.queryByText("ballotLocator.audit.question")).toBeNull()
        expect(screen.queryByRole("button", {name: "ballotLocator.audit.no"})).toBeNull()
    })
})

describe("when the choices match", () => {
    it("says so, in a status the voter hears, and that this audited ballot is spent", async () => {
        renderCompare()
        await userEvent.click(answer("yes"))
        const panel = screen.getByRole("status")
        expect(within(panel).getByRole("heading", {level: 3})).toHaveTextContent(
            "ballotLocator.audit.match.title"
        )
        expect(panel).toHaveTextContent("ballotLocator.audit.match.text")
        expect(screen.queryByRole("alert")).toBeNull()
    })

    it("sends the voter back to vote, on the same election", async () => {
        renderCompare()
        await userEvent.click(answer("yes"))
        const link = screen.getByRole("link", {name: "ballotLocator.audit.match.vote"})
        expect(link).toHaveAttribute("href", VOTE_HREF)
        expect(link).not.toHaveAttribute("target")
    })

    it("moves the keyboard to the panel", async () => {
        renderCompare()
        await userEvent.click(answer("yes"))
        expect(screen.getByRole("status")).toHaveFocus()
    })

    it("lets the voter change their answer, and puts the keyboard back on the first answer", async () => {
        renderCompare()
        await userEvent.click(answer("yes"))
        await userEvent.click(screen.getByRole("button", {name: "ballotLocator.audit.change"}))
        expect(screen.queryByRole("status")).toBeNull()
        expect(answer("yes")).toHaveFocus()
        expect(stateOf(2)).toBe("current")
    })
})

describe("when the choices do not match", () => {
    it("tells the voter not to vote on this device, in an alert, and what the browser kept", async () => {
        renderCompare()
        await userEvent.click(answer("no"))
        const panel = screen.getByRole("alert")
        expect(within(panel).getByRole("heading", {level: 3})).toHaveTextContent(
            "ballotLocator.audit.mismatch.title"
        )
        expect(panel).toHaveTextContent("ballotLocator.audit.mismatch.text")
        expect(panel).toHaveTextContent("ballotLocator.audit.mismatch.then")
        expect(screen.queryByRole("status")).toBeNull()
        expect(screen.queryByRole("link", {name: "ballotLocator.audit.match.vote"})).toBeNull()
    })

    it("moves the keyboard to the panel", async () => {
        renderCompare()
        await userEvent.click(answer("no"))
        expect(screen.getByRole("alert")).toHaveFocus()
    })

    it("saves the evidence on this device, from a button, and sends nothing", async () => {
        const {onSaveEvidence} = renderCompare()
        await userEvent.click(answer("no"))
        await userEvent.click(
            screen.getByRole("button", {name: "ballotLocator.audit.mismatch.save"})
        )
        expect(onSaveEvidence).toHaveBeenCalledTimes(1)
    })

    it("lets the voter change their answer", async () => {
        renderCompare()
        await userEvent.click(answer("no"))
        await userEvent.click(screen.getByRole("button", {name: "ballotLocator.audit.change"}))
        expect(screen.queryByRole("alert")).toBeNull()
        expect(answer("no")).toBeVisible()
        expect(answer("yes")).toHaveFocus()
    })

    describe("with no one to report to", () => {
        it("tells the voter to report it to the election authority, with no box and no email", async () => {
            const {container} = renderCompare()
            await userEvent.click(answer("no"))
            expect(screen.getByText("ballotLocator.audit.mismatch.reportNoContact")).toBeVisible()
            expect(container.querySelector(".audit-compare-contact")).toBeNull()
            expect(screen.queryByText("ballotLocator.audit.mismatch.report")).toBeNull()
            expect(
                screen.queryByRole("link", {name: "ballotLocator.audit.mismatch.email"})
            ).toBeNull()
            expect(screen.queryByText("ballotLocator.audit.mismatch.fromPolicy")).toBeNull()
            expect(screen.queryByText("ballotLocator.audit.mismatch.fromSettings")).toBeNull()
        })

        it("does the same for a contact that has nothing in it", async () => {
            const {container} = renderCompare({contact: {source: EContactSource.SETTINGS}})
            await userEvent.click(answer("no"))
            expect(container.querySelector(".audit-compare-contact")).toBeNull()
            expect(screen.getByText("ballotLocator.audit.mismatch.reportNoContact")).toBeVisible()
        })
    })

    describe("with a contact from the election's policy", () => {
        it("shows who it is, and says it is from the published policy, not from this page", async () => {
            const {container} = renderCompare({contact: BOARD})
            await userEvent.click(answer("no"))
            const box = container.querySelector(".audit-compare-contact") as HTMLElement
            expect(within(box).getByText("ballotLocator.audit.mismatch.report")).toBeVisible()
            expect(box).toHaveTextContent("Electoral board · board@example.org · +34 900 000 000")
            expect(within(box).getByText("ballotLocator.audit.mismatch.fromPolicy")).toBeVisible()
            expect(screen.queryByText("ballotLocator.audit.mismatch.fromSettings")).toBeNull()
            expect(screen.queryByText("ballotLocator.audit.mismatch.reportNoContact")).toBeNull()
        })

        it("offers an email to the authority, with the Tracking ID in it", async () => {
            renderCompare({contact: BOARD})
            await userEvent.click(answer("no"))
            const href =
                screen
                    .getByRole("link", {name: "ballotLocator.audit.mismatch.email"})
                    .getAttribute("href") ?? ""
            expect(href.startsWith("mailto:board@example.org?")).toBe(true)
            const query = new URLSearchParams(href.slice(href.indexOf("?") + 1))
            expect(query.get("subject")).toBe("ballotLocator.audit.mismatch.emailSubject")
            expect(query.get("body")).toBe(
                `ballotLocator.audit.mismatch.emailBody ${JSON.stringify({id: BALLOT_ID})}`
            )
        })
    })

    describe("with the election's support contact", () => {
        it("says it is from the election's settings", async () => {
            const {container} = renderCompare({
                contact: {
                    name: "Support",
                    email: "help@example.org",
                    source: EContactSource.SETTINGS,
                },
            })
            await userEvent.click(answer("no"))
            const box = container.querySelector(".audit-compare-contact") as HTMLElement
            expect(within(box).getByText("ballotLocator.audit.mismatch.fromSettings")).toBeVisible()
            expect(screen.queryByText("ballotLocator.audit.mismatch.fromPolicy")).toBeNull()
        })
    })

    describe("with a contact the voter cannot email", () => {
        it("shows the details but offers no email for a phone number alone", async () => {
            const {container} = renderCompare({
                contact: {phone: "+34 900 000 000", source: EContactSource.SETTINGS},
            })
            await userEvent.click(answer("no"))
            expect(container.querySelector(".audit-compare-contact")).toHaveTextContent(
                "+34 900 000 000"
            )
            expect(
                screen.queryByRole("link", {name: "ballotLocator.audit.mismatch.email"})
            ).toBeNull()
        })

        it("offers no email for an address that would add recipients or fields", async () => {
            renderCompare({
                contact: {
                    email: "a@example.org,b@example.org?cc=c@example.org",
                    source: EContactSource.SETTINGS,
                },
            })
            await userEvent.click(answer("no"))
            expect(
                screen.queryByRole("link", {name: "ballotLocator.audit.mismatch.email"})
            ).toBeNull()
        })
    })
})

describe("How does this work?", () => {
    const toggle = () => screen.getByRole("button", {name: "ballotLocator.audit.how"})

    it("starts shut", () => {
        const {container} = renderCompare()
        expect(toggle()).toHaveAttribute("aria-expanded", "false")
        expect(toggle()).not.toHaveAttribute("aria-controls")
        expect(container.querySelector(".audit-compare-faq")).toBeNull()
    })

    it("opens in place with the four questions the board gives and the tip, and shuts again", async () => {
        const {container} = renderCompare()
        await userEvent.click(toggle())
        const faq = container.querySelector(".audit-compare-faq") as HTMLElement
        expect(toggle()).toHaveAttribute("aria-expanded", "true")
        expect(toggle()).toHaveAttribute("aria-controls", faq.id)
        const questions = Array.from(faq.querySelectorAll("dt")).map((dt) => dt.textContent)
        const answers = Array.from(faq.querySelectorAll("dd")).map((dd) => dd.textContent)
        const keys = ["why", "catch", "spent", "differ"]
        expect(questions).toEqual(keys.map((key) => `ballotLocator.audit.faq.${key}.question`))
        expect(answers).toEqual(keys.map((key) => `ballotLocator.audit.faq.${key}.answer`))
        expect(within(faq).getByText("ballotLocator.audit.faq.tip")).toBeVisible()
        await userEvent.click(toggle())
        expect(container.querySelector(".audit-compare-faq")).toBeNull()
    })

    it("stays open while the voter answers", async () => {
        renderCompare()
        await userEvent.click(toggle())
        await userEvent.click(answer("yes"))
        expect(toggle()).toHaveAttribute("aria-expanded", "true")
    })
})

describe("printing", () => {
    it("leaves the question, the panels and the FAQ out of the printed page", async () => {
        const {container} = renderCompare()
        await userEvent.click(screen.getByRole("button", {name: "ballotLocator.audit.how"}))
        const screenOnly = (selector: string) =>
            expect(container.querySelector(selector)?.className).toContain(
                "audit-compare-screen-only"
            )
        screenOnly(".audit-compare-question")
        screenOnly(".audit-compare-how")
        screenOnly(".audit-compare-faq")
        await userEvent.click(answer("yes"))
        screenOnly(".audit-compare-match")
        await userEvent.click(screen.getByRole("button", {name: "ballotLocator.audit.change"}))
        await userEvent.click(answer("no"))
        screenOnly(".audit-compare-mismatch")
        const printed = Array.from(document.styleSheets)
            .flatMap((sheet) => Array.from(sheet.cssRules))
            .filter((rule) => rule.cssText.startsWith("@media print"))
            .map((rule) => rule.cssText)
            .join("")
        expect(printed).toContain(".audit-compare-screen-only")
        expect(printed).toContain(":not(:has(.tracker-selections))")
    })

    it("keeps the title, the purpose, the steps and the choices on it", () => {
        const {container} = renderCompare()
        for (const selector of [
            ".audit-compare-title",
            ".audit-compare-purpose",
            ".audit-compare-steps",
            ".audit-compare-choices",
        ]) {
            const el = container.querySelector(selector) as HTMLElement
            expect(el.closest(".tracker-selections")).not.toBeNull()
            expect(el.className).not.toContain("audit-compare-screen-only")
        }
    })
})

describe("on a phone", () => {
    it("stacks the steps and the answers once the card is narrow", () => {
        renderCompare()
        expect(css()).toMatch(/@container \(max-width: 560px\)/)
        const card = document.querySelector(".audit-compare") as HTMLElement
        expect(getComputedStyle(card).containerType).toBe("inline-size")
    })
})

describe("accessibility", () => {
    it("has no axe violations, at each stage", async () => {
        const {container} = renderCompare({contact: BOARD})
        expect(await violations(container)).toEqual([])
        await userEvent.click(screen.getByRole("button", {name: "ballotLocator.audit.how"}))
        expect(await violations(container)).toEqual([])
        await userEvent.click(answer("yes"))
        expect(await violations(container)).toEqual([])
        await userEvent.click(screen.getByRole("button", {name: "ballotLocator.audit.change"}))
        await userEvent.click(answer("no"))
        await act(async () => undefined)
        expect(await violations(container)).toEqual([])
    })
})
