// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import {MemoryRouter, Route, Routes} from "react-router-dom"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import * as verify from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import {
    auditedScenario,
    bucketAnswer,
    bucketUrlOf,
    ELECTION_ID,
    EVENT_ID,
    FakeNet,
    IAuditedScenario,
    KEYS,
    LOG_URLS,
    NOW,
    OTHER_TRACKING_ID,
    receiptUrlOf,
    TestLog,
    TRACKING_ID,
    WITNESSES,
} from "../fixtures/vitrine"
import {recordSequentCore, RecordedBallot, singleContestBallot} from "../__mocks__/auditableBallots"
import {resetSequentCore} from "../__mocks__/sequentCore"
import {GlobalSettings, SettingsContext} from "../providers/SettingsContextProvider"
import {AUDITED_ROUTE, publicConfigUrl} from "../services/vitrine/auditedBallot"
import type {IVitrineSdk} from "../services/vitrine/sdk"
import {AuditedScreen} from "./AuditedScreen"
import type {IAuditedDeps} from "./hooks/useAuditedBallot"

// The key, and the values it is given: what the screen says, and with what.
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
        i18n: {language: "en"},
    }),
}))

const sdk: IVitrineSdk = {verify, pure}
const TENANT_ID = "7c1e5d0a-2b3f-4c4d-8e5f-6a7b8c9d0e1f"
const IDS = {tenantId: TENANT_ID, eventId: EVENT_ID, electionId: ELECTION_ID}
const PAGE = `/tenant/${TENANT_ID}/event/${EVENT_ID}/election/${ELECTION_ID}/audited`
const SETTINGS: GlobalSettings = {
    DISABLE_AUTH: true,
    QUERY_POLL_INTERVAL_MS: 60000,
    DEFAULT_TENANT_ID: TENANT_ID,
    DEFAULT_EVENT_ID: EVENT_ID,
    ONLINE_VOTING_CLIENT_ID: "ballot-verifier",
    KEYCLOAK_URL: "https://keycloak.test/",
    HASURA_URL: "https://hub.test/v1/graphql",
    APP_VERSION: "test",
    APP_HASH: "test",
    PUBLIC_BUCKET_URL: "https://s3.test/public/",
    HUB_NAME: "h1",
    VOTING_PORTAL_URL: "https://vote.test",
}

const EVENTS = new TestLog(KEYS.events, KEYS.eventsTime, LOG_URLS.events)

let net: FakeNet

beforeAll(async () => {
    await verify.init()
})

beforeEach(() => {
    localStorage.clear()
    resetSequentCore()
    net = new FakeNet()
})

afterEach(() => {
    jest.restoreAllMocks()
})

/** The audited ballot the locator has: its Tracking ID is the fixtures'. */
const audited = (trackingId = TRACKING_ID): RecordedBallot => ({
    ...singleContestBallot(),
    trackingId,
})

const located = (record: RecordedBallot) =>
    JSON.stringify({
        data: {
            locate_ballot: {
                status: "found",
                ballot_id: TRACKING_ID,
                content: JSON.stringify(record.ballot),
            },
        },
    })

/** The public record of the scenario: the event's config with its policy,
 * the bucket, the log's receipt and checkpoint, and the locator's ballot. */
const serveRecord = (s: IAuditedScenario, record: RecordedBallot | undefined = audited()) => {
    net.serve(
        publicConfigUrl(SETTINGS.PUBLIC_BUCKET_URL, IDS),
        JSON.stringify({
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            election_event_presentation: {vitrine: {policy: s.policy}},
        })
    )
    net.serve(bucketUrlOf(), bucketAnswer(s.ballots))
    net.serve(receiptUrlOf(s.entry, s.index), s.receipt)
    net.serve(
        `${LOG_URLS.ballots}checkpoint`,
        s.ballots.checkpoint(s.ballots.entries.length, WITNESSES)
    )
    // The events log, read for successor policies: it has none.
    net.serve(`${LOG_URLS.events}checkpoint`, EVENTS.checkpoint(0, WITNESSES))
    if (record) {
        recordSequentCore(record)
        net.serve(SETTINGS.HASURA_URL, located(record), 200, "POST")
    }
}

const fragment = (s: IAuditedScenario, id = TRACKING_ID) => `#id=${id}&policy=${s.policyId}`

const renderScreen = (hash: string, settings: Partial<GlobalSettings> = {}) => {
    const deps: Partial<IAuditedDeps> = {
        loadSdk: async () => sdk,
        fetch: net.fetch,
        now: () => NOW,
        location: () => `https://hub.test${PAGE}${hash}`,
    }
    return render(
        <ThemeProvider theme={theme}>
            <SettingsContext.Provider
                value={{loaded: true, globalSettings: {...SETTINGS, ...settings}}}
            >
                <MemoryRouter initialEntries={[{pathname: PAGE, hash}]}>
                    <Routes>
                        <Route path={AUDITED_ROUTE} element={<AuditedScreen deps={deps} />} />
                    </Routes>
                </MemoryRouter>
            </SettingsContext.Provider>
        </ThemeProvider>
    )
}

const stateOf = async () => screen.findByTestId("audited-state")

const checkStates = () =>
    Object.fromEntries(
        ["record", "secret", "decoded"].map((key) => [
            key,
            document.querySelector(`.audited-check-${key}`)?.getAttribute("data-state"),
        ])
    )

const locatorCalls = () => net.calls.filter((c) => c.url === SETTINGS.HASURA_URL)

describe("the audited ballot screen", () => {
    it("decodes an audited ballot of the record in the browser, for the voter to compare", async () => {
        const s = auditedScenario()
        serveRecord(s)
        renderScreen(fragment(s))

        const compare = await screen.findByText("auditedScreen.compare.question")
        expect(compare).toBeInTheDocument()
        expect(checkStates()).toEqual({record: "done", secret: "done", decoded: "done"})
        // The hub's own header and words, the Audited tag and the full ID.
        expect(document.querySelector(".hub-header-name")).toHaveTextContent("h1")
        expect(document.querySelector(".hub-header-strip")).toHaveTextContent(
            'auditedScreen.header.strip.withOperator {"operator":"hubco1"}'
        )
        expect(screen.getByText("auditedScreen.tag")).toBeInTheDocument()
        expect(document.querySelector(".full-tracking-id-value")).toHaveTextContent(TRACKING_ID)
        expect(screen.getByText(/^auditedScreen\.note\.named/)).toHaveTextContent('"hub":"h1"')
        expect(screen.getByText("auditedScreen.compare.steps.decoded.title")).toBeInTheDocument()
        expect(screen.getByText(/^auditedScreen\.checks\.record\.text/)).toHaveTextContent(
            `"index":${s.index}`
        )
        // The decoded choice: the first candidate.
        const choices = document.querySelector(".audited-selections-list") as HTMLElement
        expect(within(choices).getByText("Alice Example")).toBeInTheDocument()
        expect(screen.queryByRole("button", {name: /print/i})).toBeNull()

        // Nothing went with credentials, and the hub's server heard only of
        // the whole Tracking ID, which the locator needs.
        expect(net.calls.every((c) => c.init?.credentials === "omit")).toBe(true)
        expect(net.requests()).toEqual(
            expect.arrayContaining([
                `GET ${bucketUrlOf()}`,
                `GET ${receiptUrlOf(s.entry, s.index)}`,
                `POST ${SETTINGS.HASURA_URL}`,
            ])
        )

        fireEvent.click(document.querySelector(".audit-compare-yes") as HTMLElement)
        const back = document.querySelector(".audit-compare-back") as HTMLAnchorElement
        expect(back).toHaveTextContent("auditedScreen.compare.match.back")
        expect(back.getAttribute("href")).toBe(
            `https://vote.test/tenant/${TENANT_ID}/event/${EVENT_ID}/election/${ELECTION_ID}/start`
        )
    })

    it("goes back in the browser's history without a voting site", async () => {
        const s = auditedScenario()
        serveRecord(s)
        renderScreen(fragment(s), {VOTING_PORTAL_URL: undefined, HUB_NAME: undefined})
        await screen.findByText("auditedScreen.compare.question")
        expect(document.querySelector(".hub-header-name")).toHaveTextContent(
            "auditedScreen.header.unnamed"
        )
        expect(screen.getByText(/^auditedScreen\.note\.unnamed/)).toBeInTheDocument()
        fireEvent.click(document.querySelector(".audit-compare-yes") as HTMLElement)
        const back = jest.spyOn(window.history, "back").mockImplementation(() => undefined)
        fireEvent.click(document.querySelector(".audit-compare-back") as HTMLElement)
        expect(back).toHaveBeenCalled()
    })

    it("never decodes, or fetches, a cast ballot", async () => {
        const s = auditedScenario({}, "cast")
        serveRecord(s)
        renderScreen(fragment(s))
        const state = await stateOf()
        expect(state).toHaveClass("audited-state-cast")
        expect(locatorCalls()).toEqual([])
        expect(screen.queryByText("auditedScreen.tag")).toBeNull()
    })

    it("says so when the link isn't complete, and asks nothing", async () => {
        const s = auditedScenario()
        renderScreen(`#id=${TRACKING_ID.slice(0, 9)}&policy=${s.policyId}`)
        expect(await stateOf()).toHaveClass("audited-state-bad-link")
        expect(net.calls).toEqual([])
    })

    it("says so when the election's policy can't be had", async () => {
        const s = auditedScenario()
        renderScreen(fragment(s))
        const state = await stateOf()
        expect(state).toHaveClass("audited-state-no-policy")
        expect(state.querySelector(".audited-state-retry")).not.toBeNull()
    })

    it("says so when the record has no such ballot, and tries again when asked", async () => {
        const s = auditedScenario()
        serveRecord(s)
        net.forget(bucketUrlOf())
        net.serve(bucketUrlOf(), JSON.stringify({entries: []}))
        renderScreen(fragment(s))
        const state = await stateOf()
        expect(state).toHaveClass("audited-state-not-found")
        expect(locatorCalls()).toEqual([])

        net.serve(bucketUrlOf(), bucketAnswer(s.ballots))
        fireEvent.click(state.querySelector(".audited-state-retry") as HTMLElement)
        expect(await screen.findByText("auditedScreen.compare.question")).toBeInTheDocument()
    })

    it("shows nothing of a ballot the log doesn't prove audited", async () => {
        const s = auditedScenario()
        serveRecord(s)
        // The log answers with its receipt for another entry.
        net.serve(receiptUrlOf(s.entry, s.index), s.ballots.receipt(0, 3, WITNESSES))
        renderScreen(fragment(s))
        const state = await stateOf()
        expect(state).toHaveClass("audited-state-record-failed")
        expect(state).toHaveAttribute("role", "alert")
        expect(checkStates()).toEqual({record: "failed", secret: "waiting", decoded: "waiting"})
        expect(screen.queryByText("auditedScreen.compare.question")).toBeNull()
    })

    it("fails the audit of a ballot that isn't the one its Tracking ID names", async () => {
        const s = auditedScenario()
        serveRecord(s, audited(OTHER_TRACKING_ID))
        renderScreen(fragment(s))
        const state = await stateOf()
        expect(state).toHaveClass("audited-state-mismatch")
        expect(checkStates()).toEqual({record: "done", secret: "failed", decoded: "waiting"})
        expect(screen.queryByText("Alice Example")).toBeNull()
    })

    it("says so when the locator doesn't have the audited ballot yet", async () => {
        const s = auditedScenario()
        serveRecord(s, undefined)
        net.serve(
            SETTINGS.HASURA_URL,
            JSON.stringify({data: {locate_ballot: {status: "not-found"}}}),
            200,
            "POST"
        )
        renderScreen(fragment(s))
        expect(await stateOf()).toHaveClass("audited-state-no-content")
        await waitFor(() => expect(checkStates().record).toBe("done"))
    })
})
