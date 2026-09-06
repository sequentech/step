// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {MemoryRouter, Route, Routes, useLocation} from "react-router-dom"
import {ESecurityConfirmationPolicy, IElection} from "@sequentech/ui-core"
import StartActions from "./StartActions"

// StartActions only uses `t` for button captions and `i18n.language` to pick the
// declaration translation. Echoing the key keeps the assertions about the
// accessible name, which must come from the election's own localized text.
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string): string => key,
        i18n: {language: "es"},
    }),
}))

const ENGLISH_DECLARATION = "I declare that I am eligible to vote in this election."
const SPANISH_DECLARATION = "Declaro que tengo derecho a votar en esta elección."

const buildElection = (securityConfirmationPolicy?: ESecurityConfirmationPolicy): IElection =>
    ({
        id: "election-1",
        election_event_id: "event-1",
        tenant_id: "tenant-1",
        image_document_id: "",
        contests: [],
        presentation: {
            security_confirmation_policy: securityConfirmationPolicy,
            i18n: {
                en: {security_confirmation_html: `<p>${ENGLISH_DECLARATION}</p>`},
                es: {security_confirmation_html: `<p>${SPANISH_DECLARATION}</p>`},
            },
        },
    }) as unknown as IElection

const LocationProbe = () => {
    const location = useLocation()
    return <output aria-label="Current route">{location.pathname + location.search}</output>
}

const renderStartActions = (
    election: IElection,
    {isDeclineToVotePolicyEnabled = false, onDeclineToVoteClick = jest.fn()} = {}
) => {
    render(
        <MemoryRouter
            initialEntries={[
                "/tenant/tenant-1/event/event-1/election/election-1/start?preview=true",
            ]}
        >
            <Routes>
                <Route
                    path="/tenant/:tenantId/event/:eventId/election/:electionId/*"
                    element={
                        <StartActions
                            election={election}
                            isDeclineToVotePolicyEnabled={isDeclineToVotePolicyEnabled}
                            onDeclineToVoteClick={onDeclineToVoteClick}
                        />
                    }
                />
            </Routes>
            <LocationProbe />
        </MemoryRouter>
    )
    return {onDeclineToVoteClick}
}

// The accessible name is the flattened declaration, so the surrounding markup
// (<p>, links, emphasis) does not appear in it.
const declarationCheckbox = () => screen.getByRole("checkbox", {name: SPANISH_DECLARATION})

const startButton = () => screen.getByRole("button", {name: "startScreen.startButton"})
const startLink = () => screen.getByRole("link", {name: "startScreen.startButton"})

describe("StartActions navigation", () => {
    it("has one named keyboard stop in both directions", async () => {
        const user = userEvent.setup()
        renderStartActions(buildElection(), {isDeclineToVotePolicyEnabled: true})
        const decline = screen.getByRole("button", {name: "startScreen.declineToVoteButton"})

        expect(screen.queryByRole("button", {name: "startScreen.startButton"})).toBeNull()
        expect(startLink()).toHaveClass("start-voting-button")
        await user.tab()
        expect(startLink()).toHaveFocus()
        await user.tab()
        expect(decline).toHaveFocus()
        await user.tab({shift: true})
        expect(startLink()).toHaveFocus()
    })

    it("navigates with Enter and preserves route parameters and the query string", async () => {
        const user = userEvent.setup()
        renderStartActions(buildElection())

        await user.tab()
        await user.keyboard("{Enter}")

        expect(screen.getByLabelText("Current route")).toHaveTextContent(
            "/tenant/tenant-1/event/event-1/election/election-1/vote?preview=true"
        )
    })

    it("does not expose a navigation target until the mandatory declaration is accepted", async () => {
        const user = userEvent.setup()
        renderStartActions(buildElection(ESecurityConfirmationPolicy.MANDATORY))

        expect(screen.queryByRole("link", {name: "startScreen.startButton"})).toBeNull()
        await user.tab()
        await user.keyboard("{Enter}")
        expect(startButton()).toBeDisabled()
        expect(screen.getByLabelText("Current route")).toHaveTextContent("/start?preview=true")

        await user.keyboard(" ")
        expect(startLink()).toBeInTheDocument()
        await user.keyboard(" ")
        expect(screen.queryByRole("link", {name: "startScreen.startButton"})).toBeNull()
        expect(startButton()).toBeDisabled()
    })
})

describe("StartActions security confirmation", () => {
    describe("when the policy is MANDATORY", () => {
        it("names the checkbox with the displayed, localized declaration", () => {
            renderStartActions(buildElection(ESecurityConfirmationPolicy.MANDATORY))

            // Located by role and accessible name: this is the regression guard.
            // Before the fix the checkbox had no label of any kind and only
            // `getByRole("checkbox")` with no name would have matched.
            expect(declarationCheckbox()).toBeInTheDocument()
            expect(screen.getByText(SPANISH_DECLARATION)).toBeInTheDocument()
            // The name follows the language in use, not the "en" fallback.
            expect(
                screen.queryByRole("checkbox", {name: ENGLISH_DECLARATION})
            ).not.toBeInTheDocument()
        })

        it("falls back to the English declaration when the language has none", () => {
            const election = buildElection(ESecurityConfirmationPolicy.MANDATORY)
            delete (election.presentation as unknown as {i18n: Record<string, unknown>}).i18n.es

            renderStartActions(election)

            expect(screen.getByRole("checkbox", {name: ENGLISH_DECLARATION})).toBeInTheDocument()
        })

        it("toggles exactly once when the declaration text is clicked", async () => {
            const user = userEvent.setup()
            renderStartActions(buildElection(ESecurityConfirmationPolicy.MANDATORY))

            expect(declarationCheckbox()).not.toBeChecked()

            // The row is clickable for mouse users and the checkbox stops the
            // event propagating, so the two handlers must not cancel each other.
            await user.click(screen.getByText(SPANISH_DECLARATION))
            expect(declarationCheckbox()).toBeChecked()

            await user.click(screen.getByText(SPANISH_DECLARATION))
            expect(declarationCheckbox()).not.toBeChecked()
        })

        it("toggles exactly once when the checkbox itself is clicked", async () => {
            const user = userEvent.setup()
            renderStartActions(buildElection(ESecurityConfirmationPolicy.MANDATORY))

            await user.click(declarationCheckbox())
            expect(declarationCheckbox()).toBeChecked()

            await user.click(declarationCheckbox())
            expect(declarationCheckbox()).not.toBeChecked()
        })

        it("stays operable with the keyboard", async () => {
            const user = userEvent.setup()
            renderStartActions(buildElection(ESecurityConfirmationPolicy.MANDATORY))

            await user.tab()
            expect(declarationCheckbox()).toHaveFocus()

            await user.keyboard(" ")
            expect(declarationCheckbox()).toBeChecked()

            await user.keyboard(" ")
            expect(declarationCheckbox()).not.toBeChecked()
        })

        it("gates the start and decline-to-vote buttons on the checkbox", async () => {
            const user = userEvent.setup()
            renderStartActions(buildElection(ESecurityConfirmationPolicy.MANDATORY), {
                isDeclineToVotePolicyEnabled: true,
            })

            const declineButton = screen.getByRole("button", {
                name: "startScreen.declineToVoteButton",
            })
            expect(startButton()).toBeDisabled()
            expect(declineButton).toBeDisabled()

            await user.click(declarationCheckbox())

            expect(startLink()).toBeInTheDocument()
            expect(
                screen.getByRole("button", {name: "startScreen.declineToVoteButton"})
            ).toBeEnabled()
        })

        it("only offers the decline-to-vote button when that policy is enabled", () => {
            renderStartActions(buildElection(ESecurityConfirmationPolicy.MANDATORY))

            expect(
                screen.queryByRole("button", {name: "startScreen.declineToVoteButton"})
            ).not.toBeInTheDocument()
        })
    })

    describe("when the policy is not MANDATORY", () => {
        it.each([
            ["NONE", ESecurityConfirmationPolicy.NONE],
            ["unset", undefined],
        ])("renders no declaration and an enabled start link (%s)", (_label, policy) => {
            renderStartActions(buildElection(policy), {isDeclineToVotePolicyEnabled: true})

            expect(screen.queryByRole("checkbox")).not.toBeInTheDocument()
            expect(screen.queryByText(SPANISH_DECLARATION)).not.toBeInTheDocument()
            expect(startLink()).toBeInTheDocument()
            expect(
                screen.getByRole("button", {name: "startScreen.declineToVoteButton"})
            ).toBeEnabled()
        })
    })
})
