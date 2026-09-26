// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The voter's breadcrumb: which steps there are, which one is current, and what
 * happens in a host that has never heard of `breadcrumbSteps.*`.
 *
 * This was the portal's `components/Stepper.tsx`. It is here because the Election
 * Architect's Ballot Preview draws the same breadcrumb, and drew it from a second
 * list of labels the wizard kept for itself — which is how the preview came to show
 * "Ballot List" and "Confirmation" where a voter sees "Ballots" and "Confirm".
 *
 * The last case is the load-bearing one, and it is the opposite call to
 * `StartLayout`'s. That layout takes its wording as a prop because there is no
 * sensible default for a paragraph of instructions. Four one-word step labels do have
 * one, and a preview that showed `breadcrumbSteps.ballot` would be plainly broken,
 * so this component resolves its own keys and ships the portal's English as the
 * fallback.
 */

import {ThemeProvider} from "@mui/material/styles"
import {render as mount} from "@testing-library/react"
import i18next, {type i18n as I18n} from "i18next"
import React from "react"
import {I18nextProvider} from "react-i18next"

import {BallotSteps} from "./BallotSteps"
import {theme} from "../services/theme"

/** A catalogue with the portal's own `breadcrumbSteps`, as a deployment has. */
const withCatalogue = (): I18n => {
    const i18n = i18next.createInstance()
    void i18n.init({
        lng: "en",
        fallbackLng: "en",
        resources: {
            en: {
                translation: {
                    breadcrumbSteps: {
                        electionList: "Ballots",
                        ballot: "Ballot",
                        review: "Review",
                        confirmation: "Confirm",
                    },
                },
            },
        },
        interpolation: {escapeValue: false},
    })
    return i18n
}

/** A host with no `breadcrumbSteps` at all, which is the wizard. */
const withoutCatalogue = (): I18n => {
    const i18n = i18next.createInstance()
    void i18n.init({
        lng: "en",
        fallbackLng: "en",
        resources: {en: {translation: {}}},
        interpolation: {escapeValue: false},
    })
    return i18n
}

const show = (ui: React.ReactElement, i18n: I18n = withCatalogue()) =>
    mount(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>{ui}</ThemeProvider>
        </I18nextProvider>
    )

/**
 * The step labels on screen, in order.
 *
 * Read off `.selected` / `.not-selected`, the classes `BreadCrumbSteps` puts on each
 * label. Those are the names a client's stylesheet targets, so a test that reads them
 * fails if they change — which is the point.
 */
const steps = (): Array<string> =>
    Array.from(
        document.querySelectorAll(".step-container .selected, .step-container .not-selected")
    ).map((label) => (label.textContent ?? "").trim())

/** Which one is drawn as current. */
const current = (): string | undefined =>
    document.querySelector(".step-container .selected")?.textContent?.trim()

describe("the voter's breadcrumb", () => {
    it("walks the four steps of an event that offers a ballot list", () => {
        show(<BallotSteps selected={0} />)

        expect(steps()).toEqual(["Ballots", "Ballot", "Review", "Confirm"])
    })

    it("drops the list when the event has only one election to vote", () => {
        show(<BallotSteps selected={1} withElectionList={false} />)

        expect(steps()).toEqual(["Ballot", "Review", "Confirm"])
    })

    it("keeps the caller's numbering when the list is there", () => {
        show(<BallotSteps selected={2} />)

        // Third of four: the caller says "review" as 2 and means the third step.
        expect(steps()).toEqual(["Ballots", "Ballot", "Review", "Confirm"])
        expect(current()).toEqual("Review")
    })

    it("shifts the caller's numbering down when the list is not", () => {
        // A caller says `selected={2}` for review whether or not this event has a
        // list, because which screen a voter is on is a property of the flow. With
        // no list, review is the *second* of three.
        show(<BallotSteps selected={2} withElectionList={false} />)

        expect(steps()).toEqual(["Ballot", "Review", "Confirm"])
        expect(current()).toEqual("Review")
    })

    it("does not fall off the start when a bypassed event is on its first screen", () => {
        // The portal's own guard: `selected === 0` stays 0 rather than going to -1,
        // which is what `SupportMaterialsScreen` passes.
        show(<BallotSteps selected={0} withElectionList={false} />)

        expect(steps()).toEqual(["Ballot", "Review", "Confirm"])
        expect(current()).toEqual("Ballot")
    })

    it("invents no English in a host with no catalogue", () => {
        // The keys, which say *supply the catalogue*. A hard-coded English word here
        // would show the platform's own label to a client who had translated it, and
        // it is what `EA-F2-053` took out.
        show(<BallotSteps selected={0} />, withoutCatalogue())

        expect(steps()).toEqual([
            "breadcrumbSteps.electionList",
            "breadcrumbSteps.ballot",
            "breadcrumbSteps.review",
            "breadcrumbSteps.confirmation",
        ])
    })

    it("prefers a translation over that English when the host has one", () => {
        const i18n = i18next.createInstance()
        void i18n.init({
            lng: "es",
            fallbackLng: "es",
            resources: {
                es: {
                    translation: {
                        breadcrumbSteps: {
                            electionList: "Votaciones",
                            ballot: "Papeleta",
                            review: "Revisión",
                            confirmation: "Confirmar",
                        },
                    },
                },
            },
            interpolation: {escapeValue: false},
        })

        show(<BallotSteps selected={0} />, i18n)

        expect(steps()).toEqual(["Votaciones", "Papeleta", "Revisión", "Confirmar"])
    })
})
