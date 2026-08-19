// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The voting portal's catalogue, for the tests in this directory.
 *
 * These components draw the voter's screens, and every string on them lives in
 * `voting-portal/src/translations/<lng>.ts` under the paths clients override. So a test
 * has to supply a catalogue, the same way `Candidate.test.tsx` does — and what it
 * supplies is *test data*, deliberately not the product's own words: `EA-F2-053` was
 * caused by copies of that wording living outside the translation files, and a copy in
 * here would be the same mistake with a smaller blast radius.
 *
 * The values below are therefore recognisable stand-ins, not the shipped English. A test
 * that asserts "Clear choices" would pass against a component that hard-codes it; a test
 * that asserts *this* text can only pass if the component went through i18n.
 */

import {ThemeProvider} from "@mui/material/styles"
import {render as mount} from "@testing-library/react"
import i18next, {type i18n as I18n} from "i18next"
import React from "react"
import {I18nextProvider} from "react-i18next"

import {theme} from "../services/theme"

/** What the tests here expect on screen, keyed the way the portal keys it. */
export const PORTAL_WORDS = {
    breadcrumbSteps: {
        electionList: "The list of ballots",
        ballot: "The ballot",
        review: "The review",
        confirmation: "The confirmation",
    },
    electionSelectionScreen: {
        title: "Which ballot",
        description: "Pick the one you want",
    },
    votingScreen: {
        backButton: "Go back",
        clearButton: "Start again",
        reviewButton: "Carry on",
    },
    reviewScreen: {
        backButton: "Change it",
        auditButton: "Check the machine",
        castBallotButton: "Send it",
    },
    confirmationScreen: {
        printButton: "On paper",
        finishButton: "All done",
    },
    startScreen: {
        instructionsTitle: "How this works",
        instructionsDescription: "Three steps, in order:",
        step1Title: "One, choose",
        step1Description: "Choose who you want.",
        step2Title: "Two, check",
        step2Description: "Check what you chose.",
        step3Title: "Three, cast",
        step3Description: "Cast it, or audit it first.",
    },
} as const

/** The same catalogue in another language, for the case that was broken. */
export const OTHER_WORDS = {
    votingScreen: {
        backButton: "Atrás",
        clearButton: "Borrar",
        reviewButton: "Siguiente",
    },
    startScreen: {
        instructionsTitle: "Cómo funciona",
        instructionsDescription: "Tres pasos:",
        step1Title: "Uno",
        step1Description: "Elige.",
        step2Title: "Dos",
        step2Description: "Revisa.",
        step3Title: "Tres",
        step3Description: "Envía.",
    },
    electionSelectionScreen: {
        title: "Qué papeleta",
        description: "Elige una",
    },
    reviewScreen: {
        backButton: "Editar la papeleta",
        auditButton: "Auditar",
        castBallotButton: "Emitir la papeleta",
    },
    confirmationScreen: {
        printButton: "Imprimir",
        finishButton: "Terminar",
    },
    breadcrumbSteps: {
        electionList: "Votaciones",
        ballot: "Papeleta",
        review: "Revisión",
        confirmation: "Confirmar",
    },
} as const

/** An i18next instance carrying whichever catalogue a test wants. */
export const catalogue = (
    words: Record<string, unknown> = PORTAL_WORDS,
    language = "en"
): I18n => {
    const instance = i18next.createInstance()
    void instance.init({
        lng: language,
        fallbackLng: language,
        resources: {[language]: {translation: words}},
        interpolation: {escapeValue: false},
    })
    return instance
}

/** A host: the portal's theme, and a catalogue for the words. */
export const inAHost = (ui: React.ReactElement, i18n: I18n = catalogue()) =>
    mount(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>{ui}</ThemeProvider>
        </I18nextProvider>
    )
