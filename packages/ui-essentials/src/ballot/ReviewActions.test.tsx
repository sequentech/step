// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The row under the review screen, lifted out of the portal's `ReviewScreen`.
 *
 * Reported against the Election Architect: *"The buttons in the Review screen do NOT look
 * or feel like it does in the Voting Portal."* They did not — the preview drew three plain
 * buttons from a table of its own, with no icons, no warning colour and no spinner. This
 * is the portal's row, so there is one of it.
 *
 * What the tests pin is what "looks like the portal" comes down to in markup: the class
 * names a client's stylesheet targets, the warning variant on *Audit ballot*, the icons at
 * each end, and the spinner that replaces the chevron while a ballot is being cast.
 */

import {screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import React from "react"

import {ReviewActions} from "./ReviewActions"
import {catalogue, inAHost, OTHER_WORDS, PORTAL_WORDS} from "./testCatalogue"

const render = inAHost
const WORDS = PORTAL_WORDS.reviewScreen

describe("the buttons under a review", () => {
    it("offers Back and Cast, and no Audit unless the event asks for it", () => {
        render(<ReviewActions />)

        expect(screen.getByText(WORDS.backButton)).toBeInTheDocument()
        expect(screen.getByText(WORDS.castBallotButton)).toBeInTheDocument()
        // `EVotingPortalAuditButtonCfg` decides, and it is the caller's to read.
        expect(screen.queryByText(WORDS.auditButton)).toBeNull()
    })

    it("adds Audit where the policy shows it, as the warning it is", () => {
        // The property a client is checking on this screen: Audit is the unusual choice
        // and looks it. `variant="warning"` is what says so, and the preview's own row
        // had no variants at all.
        render(<ReviewActions withAudit onAudit={() => undefined} />)

        const audit = screen.getByText(WORDS.auditButton).closest("button")
        expect(audit).toHaveClass("audit-button")
        expect(audit?.className).toContain("warning")
    })

    it("carries the class names a client's stylesheet targets", () => {
        render(<ReviewActions withAudit onAudit={() => undefined} onCast={() => undefined} />)

        expect(document.querySelector(".actions-container")).not.toBeNull()
        expect(document.querySelector(".audit-button")).not.toBeNull()
        expect(document.querySelector(".cast-ballot-button")).not.toBeNull()
    })

    it("puts an icon at each end", () => {
        // A chevron back, a flame on Audit, a chevron on Cast.
        const {container} = render(<ReviewActions withAudit onAudit={() => undefined} />)

        expect(container.querySelectorAll("svg").length).toBeGreaterThanOrEqual(3)
    })

    it("waits with a spinner while a ballot is being cast", () => {
        // The portal swaps the chevron for a spinner and disables the button, so nobody
        // casts twice.
        render(<ReviewActions casting onCast={() => undefined} />)

        expect(document.querySelector(".MuiCircularProgress-root")).not.toBeNull()
        expect(screen.getByText(WORDS.castBallotButton).closest("button")).toBeDisabled()
    })

    it("navigates Back through whatever the host gave it", () => {
        render(<ReviewActions backComponent="a" backTo="/somewhere" />)

        expect(document.querySelector("a")).not.toBeNull()
    })

    it("reports Audit and Cast to the host", async () => {
        const audit = jest.fn()
        const cast = jest.fn()
        render(<ReviewActions withAudit onAudit={audit} onCast={cast} />)

        await userEvent.click(screen.getByText(WORDS.auditButton))
        await userEvent.click(screen.getByText(WORDS.castBallotButton))

        expect(audit).toHaveBeenCalled()
        expect(cast).toHaveBeenCalled()
    })

    it("draws Cast dead when there is nothing to cast to", () => {
        // A preview's position. Better than a live-looking button that does nothing.
        render(<ReviewActions />)

        expect(screen.getByText(WORDS.castBallotButton).closest("button")).toBeDisabled()
    })

    it("says what the catalogue says, in whatever language it says it", () => {
        render(<ReviewActions withAudit />, catalogue(OTHER_WORDS, "es"))

        expect(screen.getByText("Editar la papeleta")).toBeInTheDocument()
        expect(screen.getByText("Emitir la papeleta")).toBeInTheDocument()
        expect(screen.queryByText(WORDS.castBallotButton)).toBeNull()
    })
})
