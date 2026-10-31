// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Provider} from "react-redux"
import {MemoryRouter, Route, Routes} from "react-router-dom"
import {act, render, screen, waitFor} from "@testing-library/react"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import type {IAuditableBallot} from "@sequentech/ui-core"
import {clearVoterSession, store} from "../store/store"
import {
    clearAuditableBallot,
    selectAuditableBallot,
    setAuditableBallot,
} from "../store/auditableBallots/auditableBallotsSlice"
import {setReceivedBallot} from "../store/receivedBallots/receivedBallotsSlice"
import {EStageCheck} from "../services/vitrine/castReceipt"
import {ETransparencyMode} from "../services/vitrine/types"
import AuditScreen from "./AuditScreen"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
    Trans: () => null,
}))
jest.mock("@apollo/client/react", () => ({
    useMutation: () => [mockAudit],
    useApolloClient: () => ({}),
}))
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {BALLOT_VERIFIER_URL: "https://verifier.test/", DISABLE_AUTH: false},
    }),
}))
jest.mock("../services/BallotService", () => ({
    provideBallotService: () => ({
        hashBallot: () => "ballot-hash",
        hashMultiBallot: () => "ballot-hash",
        forgetVoterSigningKey: (...args: unknown[]) => mockForgetKey(...args),
    }),
}))
jest.mock("../services/vitrine/castReceipt", () => ({
    ...jest.requireActual("../services/vitrine/castReceipt"),
    runCastReceipt: (...args: unknown[]) => mockRunCastReceipt(...args),
}))
jest.mock("../services/vitrine/trackingId", () => ({
    trackingIdOf: () => MOCK_TRACKING_ID,
    trackingDigestHex: async () => MOCK_DIGEST,
}))
jest.mock("../services/vitrine/forward", () => ({voterForward: () => jest.fn()}))
jest.mock("../services/vitrine/voterContext", () => ({voterContextFor: jest.fn()}))
const mockAudit = jest.fn()
const mockForgetKey = jest.fn()
const mockRunCastReceipt = jest.fn()
const MOCK_TRACKING_ID = "FTBE-MHRX-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ-0123-4567-89AB"
const MOCK_DIGEST = "ab".repeat(32)
const received = {
    tenant_id: "tenant",
    election_event_id: "event",
    election_id: "election",
    ballot_id: "FTBE-MHRX",
    ballot_hash: "ballot-hash",
    voter_signing_pk: "voter-key",
    voter_ballot_signature: "signature",
    received_at: "2028-05-08T03:00:00Z",
    key_id: "box-key",
    received_signature: "receipt-signature",
}
const draw = () =>
    render(
        <ThemeProvider theme={theme}>
            <Provider store={store}>
                <MemoryRouter
                    initialEntries={["/tenant/tenant/event/event/election/election/audit"]}
                >
                    <Routes>
                        <Route
                            path="/tenant/:tenantId/event/:eventId/election/:electionId/audit"
                            element={<AuditScreen />}
                        />
                        <Route path="*" element={<div>Ballot selection</div>} />
                    </Routes>
                </MemoryRouter>
            </Provider>
        </ThemeProvider>
    )
beforeEach(() => {
    jest.clearAllMocks()
    store.dispatch(clearVoterSession())
    store.dispatch(
        setAuditableBallot({
            electionId: "election",
            isBlankBallot: false,
            auditableBallot: {
                config: {election_event_presentation: {receipts: {policy: "signed-by-ballot-box"}}},
                ballot_hash: "ballot-hash",
                voter_signing_pk: "voter-key",
                plaintext: "private-choice-and-randomness",
            } as unknown as IAuditableBallot,
        })
    )
    store.dispatch(setReceivedBallot({electionId: "election", receivedBallot: received}))
})

test("does not expose audit data before the server irreversibly marks the received ballot audited", async () => {
    mockAudit.mockReturnValue(new Promise(() => undefined))
    draw()
    expect(screen.queryByText(/private-choice-and-randomness/)).not.toBeInTheDocument()
    expect(
        screen.queryByRole("button", {name: "auditScreen.downloadButton"})
    ).not.toBeInTheDocument()
    await waitFor(() =>
        expect(mockAudit).toHaveBeenCalledWith({
            variables: {electionId: "election", ballotId: "FTBE-MHRX"},
        })
    )
})

test("a confirmed audit drops the cast key and receipt before revealing the audit copy", async () => {
    mockAudit.mockResolvedValue({data: {audit_ballot: {ballot_id: "FTBE-MHRX", status: "audited"}}})
    draw()
    await waitFor(() => expect(mockForgetKey).toHaveBeenCalledWith("election"))
    expect(store.getState().receivedBallots.election).toBeUndefined()
    expect(selectAuditableBallot("election")(store.getState())).toBeUndefined()
    expect(screen.getByText(/private-choice-and-randomness/)).toBeVisible()
})

test("a refused audit never reveals the ballot or disposes the still-usable receipt", async () => {
    mockAudit.mockRejectedValue(new Error("Already cast"))
    draw()
    await waitFor(() => expect(mockAudit).toHaveBeenCalledTimes(1))
    expect(screen.queryByText(/private-choice-and-randomness/)).not.toBeInTheDocument()
    expect(mockForgetKey).not.toHaveBeenCalled()
    expect(store.getState().receivedBallots.election).toEqual(received)
})

test("returning to the audit route after cast-data disposal cannot reveal the cast ballot", async () => {
    store.dispatch(clearAuditableBallot("election"))
    draw()
    expect(await screen.findByText("Ballot selection")).toBeVisible()
    expect(screen.queryByText(/private-choice-and-randomness/)).not.toBeInTheDocument()
    expect(mockAudit).not.toHaveBeenCalled()
})

test("a delayed audit acknowledgement preserves a newer ballot and its receipt", async () => {
    let acknowledge!: (value: {data: {audit_ballot: {ballot_id: string; status: string}}}) => void
    mockAudit.mockReturnValue(
        new Promise((resolve) => {
            acknowledge = resolve
        })
    )
    const view = draw()
    await waitFor(() => expect(mockAudit).toHaveBeenCalledTimes(1))
    view.unmount()
    const previous = store.getState().auditableBallots.election!.auditableBallot!
    store.dispatch(
        setAuditableBallot({
            electionId: "election",
            isBlankBallot: false,
            auditableBallot: {...previous, ballot_hash: "new-ballot-hash"},
        })
    )
    const newerReceipt = {...received, ballot_hash: "new-ballot-hash", ballot_id: "ABCD-EFGH"}
    store.dispatch(setReceivedBallot({electionId: "election", receivedBallot: newerReceipt}))
    await act(async () => {
        acknowledge({data: {audit_ballot: {ballot_id: "FTBE-MHRX", status: "audited"}}})
    })
    expect(selectAuditableBallot("election")(store.getState())?.ballot_hash).toBe("new-ballot-hash")
    expect(store.getState().receivedBallots.election).toEqual(newerReceipt)
    expect(mockForgetKey).not.toHaveBeenCalled()
})

describe("an event that keeps a transparency log", () => {
    beforeEach(() => {
        const previous = store.getState().auditableBallots.election!.auditableBallot!
        store.dispatch(
            setAuditableBallot({
                electionId: "election",
                isBlankBallot: false,
                auditableBallot: {
                    ...previous,
                    config: {
                        election_event_presentation: {
                            receipts: {policy: "signed-by-ballot-box"},
                            transparency: ETransparencyMode.PUBLIC,
                        },
                    },
                } as unknown as IAuditableBallot,
            })
        )
        mockAudit.mockResolvedValue({
            data: {audit_ballot: {ballot_id: "FTBE-MHRX", status: "audited", vitrine: null}},
        })
    })
    const logs = (check: EStageCheck) =>
        mockRunCastReceipt.mockImplementation(async (options: {onChecked?: Function}) => {
            options.onChecked?.(check)
        })

    test("asks the log for the audited entry of the ballot, by its Tracking ID", async () => {
        logs(EStageCheck.CHECKED)
        draw()
        await waitFor(() => expect(mockRunCastReceipt).toHaveBeenCalledTimes(1))
        expect(mockRunCastReceipt).toHaveBeenCalledWith(
            expect.objectContaining({
                target: {
                    eventId: "event",
                    electionId: "election",
                    stage: "audited",
                    ballotId: MOCK_DIGEST,
                },
                checkOnly: true,
            })
        )
    })

    test("shows the short Tracking ID with the Audited tag once the log holds the audit", async () => {
        logs(EStageCheck.CHECKED)
        const {container} = draw()
        await waitFor(() =>
            expect(container.querySelector(".audit-tracking-id-short")).toHaveTextContent(
                "FTBE-MHRX"
            )
        )
        expect(container.querySelector(".audit-tracking-id")).toHaveTextContent(
            "vitrine.status.tag.audited"
        )
        expect(container.querySelector(".audit-tracking-id")).not.toHaveTextContent(
            "vitrine.status.tag.cast"
        )
        expect(screen.getByText(/private-choice-and-randomness/)).toBeVisible()
    })

    test("keeps the audit copy back when the log holds no receipt for the audited entry", async () => {
        logs(EStageCheck.FAILED)
        const {container} = draw()
        expect(await screen.findByText("auditScreen.auditUnavailable")).toBeVisible()
        expect(screen.queryByText(/private-choice-and-randomness/)).not.toBeInTheDocument()
        expect(container.querySelector(".audit-tracking-id")).toBeNull()
        // The ballot box refused nothing, but the audit is not whole: keep the receipt.
        expect(mockForgetKey).not.toHaveBeenCalled()
        expect(store.getState().receivedBallots.election).toEqual(received)
    })

    test("accepts the box's audit while the log has only queued the entry", async () => {
        logs(EStageCheck.WAITING)
        const {container} = draw()
        await waitFor(() => expect(mockForgetKey).toHaveBeenCalledWith("election"))
        expect(container.querySelector(".audit-tracking-id-short")).toHaveTextContent("FTBE-MHRX")
    })

    test("a retry asks the log again", async () => {
        logs(EStageCheck.FAILED)
        draw()
        const retry = await screen.findByRole("button", {name: "ballotPrivacy.retry"})
        logs(EStageCheck.CHECKED)
        await act(async () => retry.click())
        await waitFor(() => expect(mockRunCastReceipt).toHaveBeenCalledTimes(2))
        expect(await screen.findByText(/private-choice-and-randomness/)).toBeVisible()
    })
})

test("an event with no transparency log asks the log nothing and shows no tag", async () => {
    mockAudit.mockResolvedValue({data: {audit_ballot: {ballot_id: "FTBE-MHRX", status: "audited"}}})
    const {container} = draw()
    await waitFor(() => expect(mockForgetKey).toHaveBeenCalledWith("election"))
    expect(mockRunCastReceipt).not.toHaveBeenCalled()
    expect(container.querySelector(".audit-tracking-id")).toBeNull()
})
