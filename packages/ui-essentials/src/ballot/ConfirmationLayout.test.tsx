// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What the confirmation screen puts on the page, and what it leaves off (design v42,
 * "the receipt as a ticket").
 *
 * The sibling of `ReviewLayout.test.tsx`, and pinned for the same reason: this
 * arrangement is a contract between the voting portal and the Election
 * Architect's preview rather than the inside of one route.
 */

import {ThemeProvider} from "@mui/material/styles"
import {render as mount, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import React from "react"

import theme from "../services/theme"
import {ConfirmationLayout} from "./ConfirmationLayout"

jest.mock("../components/QRCode/QRCode", () => ({
    __esModule: true,
    default: ({value, ariaLabelledby}: {value: string; ariaLabelledby?: string}) => (
        <div data-testid="stub-qr" data-value={value} aria-labelledby={ariaLabelledby} />
    ),
}))

const render = (ui: React.ReactElement) => mount(<ThemeProvider theme={theme}>{ui}</ThemeProvider>)

const props = {ballotId: "abc123"}

const ticket = (): HTMLElement => document.querySelector<HTMLElement>(".confirmation-ticket")!

describe("the confirmation screen's arrangement", () => {
    it("shows the whole identifier once, on the ticket", () => {
        render(<ConfirmationLayout {...props} />)

        expect(screen.getAllByTestId("ballot-id")).toHaveLength(1)
        expect(within(ticket()).getByText("abc123")).toBeInTheDocument()
    })

    it("renders the identifier as plain text when it links nowhere", () => {
        // A demo, and a preview: there is no tracker to send anybody to.
        const {container} = render(<ConfirmationLayout {...props} />)

        expect(container.querySelector("a[href]")).toBeNull()
    })

    it("makes the identifier a button when it only answers a click", async () => {
        // The portal's demo mode: no tracker, but a click explains why. An anchor
        // with no `href` is out of the tab order.
        const onBallotIdClick = jest.fn()
        const {container} = render(
            <ConfirmationLayout {...props} onBallotIdClick={onBallotIdClick} />
        )

        const id = screen.getByRole("button", {name: "abc123"})
        expect(container.querySelector("a[href]")).toBeNull()
        id.focus()
        await userEvent.keyboard("{Enter}")
        expect(onBallotIdClick).toHaveBeenCalledTimes(1)
    })

    it("links to the tracker when there is one, in a new tab", () => {
        render(<ConfirmationLayout {...props} ballotIdHref="https://verify.example/abc123" />)

        const link = screen.getByRole("link")
        expect(link).toHaveAttribute("href", "https://verify.example/abc123")
        expect(link).toHaveAttribute("target", "_blank")
    })

    it("leaves out the QR and the tear line when nothing was given to encode", () => {
        // The preview's case. A QR of an empty string is a scannable square
        // that leads nowhere, which is worse than no square.
        render(<ConfirmationLayout {...props} />)

        expect(screen.queryByTestId("stub-qr")).toBeNull()
        expect(document.querySelector(".confirmation-ticket-perforation")).toBeNull()
    })

    it("puts the QR under the tear line, named by its caption", () => {
        render(<ConfirmationLayout {...props} qrValue="https://verify.example/abc" />)

        const qr = screen.getByTestId("stub-qr")
        expect(qr).toHaveAttribute("data-value", "https://verify.example/abc")
        expect(ticket()).toContainElement(qr)
        const perforation = document.querySelector(".confirmation-ticket-perforation")!
        expect(perforation.compareDocumentPosition(qr)).toBe(Node.DOCUMENT_POSITION_FOLLOWING)
        expect(qr).toHaveAccessibleName("confirmationScreen.qrCaption")
    })

    it("takes the breadcrumb, the actions, the updates and the dialogs from its caller", () => {
        render(
            <ConfirmationLayout
                {...props}
                steps={<div data-testid="steps" />}
                actions={<div data-testid="actions" />}
                updates={<div data-testid="updates" />}
            >
                <div data-testid="dialogs" />
            </ConfirmationLayout>
        )

        expect(screen.getByTestId("steps")).toBeInTheDocument()
        expect(screen.getByTestId("dialogs")).toBeInTheDocument()
        const keep = document.querySelector(".confirmation-keep")!
        expect(keep).toContainElement(screen.getByTestId("actions"))
        expect(document.querySelector(".confirmation-updates")).toContainElement(
            screen.getByTestId("updates")
        )
    })

    it("draws no updates section, rule and all, when there are none", () => {
        render(<ConfirmationLayout {...props} actions={<div data-testid="actions" />} />)

        expect(document.querySelector(".confirmation-updates")).toBeNull()
    })

    it("orders the page as a phone shows it: title, ticket, keep your receipt, updates", () => {
        render(
            <ConfirmationLayout
                {...props}
                actions={<div data-testid="actions" />}
                updates={<div data-testid="updates" />}
            />
        )

        const columns = document.querySelector(".confirmation-columns")!
        expect(Array.from(columns.children).map((child) => child.classList[0])).toEqual([
            "confirmation-outcome",
            "confirmation-ticket",
            "confirmation-keep",
            "confirmation-updates",
        ])
    })

    it("has no help button by the title, only by the Tracking ID", () => {
        render(<ConfirmationLayout {...props} onBallotIdHelp={() => undefined} />)

        expect(document.querySelector(".screen-help-button")).toBeNull()
        const help = document.querySelector(".ballot-id-help-button")!
        expect(help).toHaveAttribute("aria-label")
        expect(document.querySelector(".ballot-id-container")).toContainElement(help as HTMLElement)
    })
})

describe("the title row and the ticket's band", () => {
    it("marks the outcome with a ballot box, not a check", () => {
        render(<ConfirmationLayout {...props} />)

        const mark = document.querySelector(".confirmation-outcome-icon")!
        expect(mark).toHaveAttribute("aria-hidden", "true")
        expect(mark.querySelector("svg.confirmation-outcome-mark")).not.toBeNull()
        expect(mark.querySelector("[data-icon=check]")).toBeNull()
    })

    it("names the election and the cast time under the title and in the band", () => {
        render(
            <ConfirmationLayout
                {...props}
                eventName="Municipal elections 2026"
                electionName="City council"
                castAt="1 November 2026, 14:32"
            />
        )

        expect(document.querySelector(".confirmation-meta")).toHaveTextContent(
            "City council · 1 November 2026, 14:32"
        )
        expect(document.querySelector(".confirmation-ticket-event")).toHaveTextContent(
            "Municipal elections 2026"
        )
        expect(document.querySelector(".confirmation-ticket-meta")).toHaveTextContent(
            "City council · 1 November 2026, 14:32"
        )
        expect(ticket()).toHaveAccessibleName("confirmationScreen.receipt")
    })

    it("leaves the time out until it is known, keeping the line", () => {
        const {rerender} = render(<ConfirmationLayout {...props} electionName="City council" />)

        const meta = document.querySelector(".confirmation-meta")!
        expect(meta.textContent).toBe("City council")

        rerender(
            <ThemeProvider theme={theme}>
                <ConfirmationLayout {...props} electionName="City council" castAt="14:32" />
            </ThemeProvider>
        )
        expect(document.querySelector(".confirmation-meta")).toBe(meta)
        expect(meta.textContent).toBe("City council · 14:32")
    })

    it("keeps the meta line, empty, when there is nothing to put on it yet", () => {
        render(<ConfirmationLayout {...props} />)

        const meta = document.querySelector<HTMLElement>(".confirmation-meta")!
        expect(meta.textContent).toBe("")
        expect(getComputedStyle(meta).minHeight).toBe("22px")
    })
})

describe("the Tracking ID on the ticket", () => {
    it("shows the short ID with its status and tag, and the caller's details under it", () => {
        render(
            <ConfirmationLayout
                {...props}
                shortBallotId="FTBE-MHRX"
                ballotIdStatus={<button className="stub-status">status</button>}
                ballotIdTag={<span className="stub-tag">Cast</span>}
                ballotIdDetails={<div className="stub-full">full</div>}
            />
        )

        const border = document.querySelector(".ballot-id-border")!
        expect(border.querySelector(".stub-status")).not.toBeNull()
        expect(within(border as HTMLElement).getByTestId("ballot-id")).toHaveTextContent(
            "FTBE-MHRX"
        )
        expect(document.querySelector(".ballot-id-check-icon")).toBeNull()
        const line = document.querySelector(".ballot-id-line")!
        expect(Array.from(line.children)).toEqual([border, document.querySelector(".stub-tag")])
        const details = document.querySelector(".confirmation-ticket-details")!
        expect(details.querySelector(".stub-full")).not.toBeNull()
        expect(
            document.querySelector(".ballot-id-container")!.compareDocumentPosition(details)
        ).toBe(Node.DOCUMENT_POSITION_FOLLOWING)
    })

    it("offers to copy the identifier when given the words for it", () => {
        const labels = {copy: "Copy", copied: "Copied", error: "Failed"}
        render(<ConfirmationLayout {...props} ballotIdCopyLabels={labels} />)

        expect(
            within(document.querySelector<HTMLElement>(".ballot-id-container")!).getByRole(
                "button",
                {name: "Copy"}
            )
        ).toBeInTheDocument()
    })

    it("draws the check mark as decoration, not as a nameless button", () => {
        render(<ConfirmationLayout {...props} />)

        const mark = document.querySelector(".ballot-id-status-icon")
        expect(mark).not.toBeNull()
        expect(mark?.closest("button")).toBeNull()
        expect(screen.queryAllByRole("button")).toHaveLength(0)
    })
})

describe("where this screen's words come from", () => {
    /*
     * `confirmationScreen.*`, and not props, so a client's override reaches the portal
     * and the preview alike (`EA-F3-015`). `t` is mocked in this file to return the
     * key, so a key on screen is the string having gone through i18n.
     */

    it("asks the catalogue for every one of them", () => {
        render(<ConfirmationLayout {...props} qrValue="https://verify.example/abc" />)

        for (const key of [
            "confirmationScreen.title",
            "confirmationScreen.receipt",
            "confirmationScreen.ballotId",
            "confirmationScreen.keepTitle",
            "confirmationScreen.keepLead",
            "confirmationScreen.qrCaption",
        ]) {
            expect(screen.getByText(key)).toBeInTheDocument()
        }
    })

    it("no longer says the retired words", () => {
        render(<ConfirmationLayout {...props} qrValue="https://verify.example/abc" />)

        for (const key of [
            "confirmationScreen.description",
            "confirmationScreen.verifyCastTitle",
            "confirmationScreen.verifyCastDescription",
        ]) {
            expect(screen.queryByText(key)).toBeNull()
        }
    })

    it("says a ballot was cast blank only when it was, under the title", () => {
        const plain = render(<ConfirmationLayout {...props} />)
        expect(plain.queryByText("confirmationScreen.blankBallot.description")).toBeNull()
        plain.unmount()

        render(<ConfirmationLayout {...props} isBlankBallot />)
        const blank = screen.getByText("confirmationScreen.blankBallot.description")
        expect(document.querySelector(".confirmation-outcome")).toContainElement(blank)
        expect(ticket()).not.toBeNull()
    })
})

describe("the headings", () => {
    it("titles the screen as its one level-one heading, with level-two sections", () => {
        render(
            <ConfirmationLayout {...props} qrValue="https://tracker" updates={<h2>updates</h2>} />
        )

        expect(screen.getByRole("heading", {level: 1})).toHaveTextContent(
            "confirmationScreen.title"
        )
        expect(screen.getAllByRole("heading", {level: 2}).map((h) => h.textContent)).toEqual([
            "confirmationScreen.ballotId",
            "confirmationScreen.keepTitle",
            "updates",
        ])
    })
})

describe("an election decided by acclamation", () => {
    it("shows no ticket, identifier or QR, but keeps the actions and the host's dialogs", () => {
        render(
            <ConfirmationLayout
                {...props}
                isFullyAcclaimed
                qrValue="https://tracker"
                actions={<div data-testid="actions" />}
            >
                <div data-testid="a-dialog" />
            </ConfirmationLayout>
        )

        expect(screen.getByRole("heading", {level: 1})).toHaveTextContent(
            "confirmationScreen.acclamation.title"
        )
        expect(screen.getByText("confirmationScreen.acclamation.description")).toBeInTheDocument()
        expect(ticket()).toBeNull()
        expect(screen.queryAllByTestId("ballot-id")).toHaveLength(0)
        expect(screen.queryByTestId("stub-qr")).toBeNull()
        expect(screen.queryByText("confirmationScreen.keepTitle")).toBeNull()
        expect(screen.getByTestId("actions")).toBeInTheDocument()
        expect(screen.getByTestId("a-dialog")).toBeInTheDocument()
    })
})
