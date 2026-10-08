/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import type {IScheduledOutcomeExplanation} from "@sequentech/ui-core"
import {ScheduledOutcome} from "./ScheduledOutcome"
import {MyTimeZoneProvider} from "./timeZoneService"
import {MY_TIME_ZONE, testI18n} from "./__fixtures__/testI18n"
import {
    refusedDefaults,
    refusedEdited,
    refusedLooser,
    runsAuthorized,
    runsNoSignatures,
    runsUnsigned,
} from "./__fixtures__/explanations"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/services/timeZones"),
    ...jest.requireActual("../../../../ui-core/src/services/eventTimeZones"),
    ...jest.requireActual("../../../../ui-core/src/types/ScheduledOutcome"),
    ...jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation"),
}))
const mockI18n = testI18n()
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: mockI18n.t, i18n: mockI18n}),
}))

const show = (explanation: IScheduledOutcomeExplanation, zone = "Asia/Dubai") =>
    render(
        <MyTimeZoneProvider zone={MY_TIME_ZONE}>
            <ScheduledOutcome explanation={explanation} zone={zone} />
        </MyTimeZoneProvider>
    )

const why = () => {
    fireEvent.click(screen.getByRole("button", {name: "Why?"}))
    return within(screen.getByRole("dialog"))
}

describe("ScheduledOutcome", () => {
    it.each([
        [runsAuthorized(), "Will run", "Authorized by configuration K7Q-2M"],
        [runsNoSignatures(), "Will run", "No signatures needed"],
        [runsUnsigned(), "Will run without signatures", "Closes without signatures"],
        [
            refusedEdited(),
            "Will be refused",
            "Not in the signed configuration. Publish and approve the configuration.",
        ],
        [
            refusedLooser(),
            "Will be refused",
            "Changed since the published configuration, which still decides. Publish and approve the configuration.",
        ],
        [
            refusedDefaults(),
            "Will be refused",
            "Nothing published yet: the defaults apply. Publish and approve the configuration.",
        ],
    ])("shows the outcome and its note (%#)", (explanation, chip, note) => {
        show(explanation)
        const outcome = within(screen.getByTestId("scheduled-outcome"))
        expect(outcome.getByText(chip)).toBeInTheDocument()
        expect(outcome.getByText(note)).toBeInTheDocument()
    })

    it("says what would change a refusal, as the server's next step", () => {
        const explanation = {
            ...refusedEdited(),
            next_step: {message_key: "scheduledOutcome.nextStep.requireConfigurationApproval"},
        }
        show(explanation)
        expect(
            screen.getByText(
                "Not in the signed configuration. Make Approve configuration need signatures, then publish and approve the configuration."
            )
        ).toBeInTheDocument()
    })

    it("words one signature in the singular", () => {
        show({
            ...runsNoSignatures(),
            checks: [
                {
                    id: "needs-signatures" as never,
                    current: {
                        message_key: "scheduledOutcome.check.needsSignatures.yes",
                        params: {signatures: 1},
                    },
                    published: null,
                    allows: true,
                },
            ],
        })
        expect(why().getByText("Yes, 1 signature")).toBeInTheDocument()
    })

    it("explains a refusal: every check, both copies, the deciding one and the next step", () => {
        show(refusedLooser())
        const panel = why()
        expect(panel.getByRole("heading", {name: "Why it will be refused"})).toBeInTheDocument()
        const rows = panel.getAllByRole("row").slice(1)
        expect(
            rows.map((row) =>
                within(row)
                    .getAllByRole("cell")
                    .map((c) => c.textContent)
            )
        ).toEqual([
            ["Yes, 2 signatures", "Yes, 2 signatures", "Allows"],
            ["No: approval K7Q-2M doesn't include it", "–", "Blocks"],
            ["Run as system", "Refuse", "Blocks"],
            [
                "Current settings are looser: they apply after the next approved publication",
                "–",
                "Blocks",
            ],
        ])
        const deciding = panel.getByTestId("outcome-check-stricter-copy")
        expect(deciding).toHaveAttribute("aria-current", "true")
        expect(within(deciding).getByText("Deciding check")).toBeInTheDocument()
        expect(panel.getByText("Publish and approve the configuration.")).toBeInTheDocument()
    })

    it("shows the times in the explanation in the row's zone", () => {
        show(refusedEdited(), "Asia/Manila")
        expect(
            why().getByText(
                "No: edited on Apr 01, 2028, 14:30 PhST by event.admin, after approval K7Q-2M"
            )
        ).toBeInTheDocument()
    })

    it.each([
        [
            "channelsChanged",
            {code: "K7Q-2M"},
            "No: the election's voting channels changed since approval K7Q-2M",
        ],
        [
            "alreadyFired",
            {code: "K7Q-2M", fired_at: "2028-04-08T20:00:00Z"},
            "No: this transition of approval K7Q-2M already ran at Apr 09, 2028, 04:00 PhST; running it again needs signatures",
        ],
        [
            "late",
            {code: "K7Q-2M", scheduled_date: "2028-04-08T20:00:00Z"},
            "No: it is more than 15 minutes past Apr 09, 2028, 04:00 PhST (approval K7Q-2M); running it now needs signatures",
        ],
    ])("explains a covered %s refusal, times in the row's zone", (key, params, text) => {
        const explanation = refusedEdited()
        explanation.checks[1] = {
            ...explanation.checks[1],
            current: {message_key: `scheduledOutcome.check.covered.${key}`, params},
        }
        show(explanation, "Asia/Manila")
        expect(why().getByText(text)).toBeInTheDocument()
    })

    it("names who signed the configuration that authorizes it", () => {
        show(runsAuthorized())
        const panel = why()
        expect(panel.getByText("Signed by Ana P. Reyes and Jose R. Dela Cruz")).toBeInTheDocument()
        expect(panel.getByText("No action needed.")).toBeInTheDocument()
    })

    it("reads an organization's own wording of a value", () => {
        const key = "scheduledOutcome.check.unsignedClose.refuse"
        const original = mockI18n.t(key)
        mockI18n.addResource("en", "translations", key, "Keep closed")
        try {
            show(refusedLooser())
            const row = within(why().getByTestId("outcome-check-unsigned-close"))
            expect(row.getByText("Keep closed")).toBeInTheDocument()
        } finally {
            mockI18n.addResource("en", "translations", key, original)
        }
    })
})
