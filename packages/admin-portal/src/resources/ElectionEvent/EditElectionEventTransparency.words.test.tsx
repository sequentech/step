/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// With the real translations, no state of the tab shows a key instead of words.

import React from "react"
import {act, render} from "@testing-library/react"
import {useMutation, useQuery} from "@apollo/client"
import i18next from "i18next"
import {I18nextProvider} from "react-i18next"
import * as verify from "@vitrine/verify"
import {EditElectionEventTransparency} from "./EditElectionEventTransparency"
import type {ITransparencyOutput} from "@/queries/Transparency"
import cat from "@/translations/cat"
import english from "@/translations/en"
import {
    draftPolicyText,
    EVENT_ID,
    NOW,
    policyHexOf,
    policyText,
    sidecarLogs,
    sidecarStatus,
} from "@/fixtures/vitrinePolicy"

jest.mock("@apollo/client", () => ({
    gql: (strings: TemplateStringsArray) => strings.join(""),
    useQuery: jest.fn(),
    useMutation: jest.fn(),
}))

const output = (o: Partial<ITransparencyOutput> = {}): ITransparencyOutput => ({
    mode: "disabled",
    refusal: null,
    locked: false,
    logs: [],
    status: {},
    draft_policy: null,
    policy: null,
    policy_id: null,
    enrolments: [],
    explorer_url: null,
    ...o,
})

const signed = policyText({bootstrap: true})
const hex = policyHexOf(signed)
const healthy = (o: Partial<ITransparencyOutput> = {}) => {
    const status = sidecarStatus(hex) as Record<string, {witnesses: Array<{alert?: boolean}>}>
    status.ballots.witnesses.forEach((w) => {
        w.alert = false
    })
    const logs = (sidecarLogs(hex) as Array<Record<string, unknown>>).map((l) => ({
        ...l,
        alert: null,
    }))
    return output({
        mode: "public",
        locked: true,
        policy: signed,
        policy_id: hex,
        draft_policy: draftPolicyText({bootstrap: true}),
        logs,
        status,
        explorer_url: "https://explorer.acme.example/e/",
        ...o,
    })
}

const STATES: Array<[string, ITransparencyOutput]> = [
    ["off", output()],
    ["public refused", output({refusal: "no-bootstrap-hub"})],
    ["waiting for the policy", output({mode: "public", draft_policy: draftPolicyText()})],
    ["healthy", healthy()],
    [
        "needing attention",
        healthy({logs: sidecarLogs(hex) as never, status: sidecarStatus(hex) as never}),
    ],
    ["unlocked", healthy({locked: false})],
    [
        "private",
        output({
            mode: "private",
            locked: true,
            policy: policyText({visibility: "private", bootstrap: true}),
        }),
    ],
]

const LANGUAGES = {en: english, cat}

beforeAll(async () => {
    await verify.init()
})

describe.each(Object.entries(LANGUAGES))("in %s", (language, translation) => {
    it.each(STATES)("shows words, not keys, when the record is %s", async (_name, state) => {
        jest.spyOn(Date, "now").mockReturnValue((NOW + 5) * 1000)
        const i18n = i18next.createInstance()
        await i18n.init({
            lng: language,
            resources: {[language]: {translation: translation.translations}},
            interpolation: {escapeValue: false},
            showSupportNotice: false,
        })
        ;(useQuery as jest.Mock).mockReturnValue({
            data: {get_transparency: state},
            loading: false,
            refetch: jest.fn(),
        })
        ;(useMutation as jest.Mock).mockReturnValue([jest.fn(), {loading: false}])
        const view = render(
            <I18nextProvider i18n={i18n}>
                <EditElectionEventTransparency electionEventId={EVENT_ID} />
            </I18nextProvider>
        )
        await act(async () => {})
        expect(view.container.textContent).not.toMatch(
            /\b(transparencyTab|electionEventScreen)\.\w+/
        )
        view.unmount()
        jest.restoreAllMocks()
    })
})
