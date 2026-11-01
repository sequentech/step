// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
// castReceipt and ballotTracker import each other (through signers). Loading
// ballotTracker first lets it see the mocked castReceipt once it is whole.
import "../services/vitrine/ballotTracker"
import React from "react"
import {Provider} from "react-redux"
import {MemoryRouter, Route, Routes} from "react-router-dom"
import {act, render, screen, waitFor} from "@testing-library/react"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import type {IAuditableBallot} from "@sequentech/ui-core"
import type {Policy} from "@vitrine/verify"
import {clearVoterSession, store} from "../store/store"
import {
    clearAuditableBallot,
    selectAuditableBallot,
    setAuditableBallot,
} from "../store/auditableBallots/auditableBallotsSlice"
import {setReceivedBallot} from "../store/receivedBallots/receivedBallotsSlice"
import {EStageCheck} from "../services/vitrine/castReceipt"
import {ballotIdChars, groupBallotId} from "../services/vitrine/ballotId"
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
        get globalSettings() {
            return mockGlobalSettings
        },
    }),
}))
// keycloak-js is an ES module jest can't load; the provider's own Keycloak is never created here.
jest.mock("keycloak-js", () => ({__esModule: true, default: jest.fn()}))
jest.mock("../hooks/useVitrinePolicy", () => ({
    useVitrinePolicy: () => mockPolicy,
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
let mockGlobalSettings = {BALLOT_VERIFIER_URL: "https://verifier.test/", DISABLE_AUTH: false}
let mockPolicy: Policy | undefined
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
    mockGlobalSettings = {BALLOT_VERIFIER_URL: "https://verifier.test/", DISABLE_AUTH: false}
    mockPolicy = undefined
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
            data: {
                audit_ballot: {
                    ballot_id: "FTBE-MHRX",
                    status: "audited",
                    publication: "published",
                    vitrine: null,
                },
            },
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

    const POLICY_ID = "sha256:abcd"
    // A policy as a parser that knows each hub's ballot verifier would give it.
    const policyWith = (hubs: Array<[string, string, string?]>): Policy =>
        ({
            id: POLICY_ID,
            hubs: new Map(
                hubs.map(([name, operator, ballotVerifier]) => [
                    name,
                    {name, operator, ...(ballotVerifier ? {ballotVerifier} : {})},
                ])
            ),
        }) as unknown as Policy
    const audited = (base: string) =>
        `${base}tenant/tenant/event/event/election/election/audited#id=${groupBallotId(
            MOCK_TRACKING_ID
        )}&policy=${POLICY_ID}`
    const shown = async () => {
        logs(EStageCheck.CHECKED)
        const view = draw()
        await waitFor(() => expect(view.container.querySelector(".audit-check")).not.toBeNull())
        return view
    }
    const tiles = (container: HTMLElement) =>
        Array.from(container.querySelectorAll<HTMLAnchorElement>(".audit-hubs a.audit-hub"))

    test("shows the review screen's Tracking ID box, Audited, once the log holds the audit", async () => {
        const {container} = await shown()
        const box = container.querySelector(".audit-header-id .vitrine-tracking-id")
        expect(box).toHaveTextContent("vitrine.status.tag.audited")
        expect(box).not.toHaveTextContent("vitrine.status.tag.notCast")
        expect(box).toHaveTextContent(groupBallotId(MOCK_TRACKING_ID).slice(0, 9))
        expect(screen.getByRole("heading", {level: 1})).toHaveTextContent("auditScreen.check.title")
    })

    test("offers no download, encrypted ballot, verifier steps or print", async () => {
        const {container} = await shown()
        expect(screen.queryByText(/private-choice-and-randomness/)).not.toBeInTheDocument()
        expect(screen.queryByText("auditScreen.downloadButton")).not.toBeInTheDocument()
        expect(screen.queryByText("auditScreen.printButton")).not.toBeInTheDocument()
        expect(screen.queryByText("auditScreen.step2Title")).not.toBeInTheDocument()
        expect(screen.queryByText("auditScreen.bottomWarning")).not.toBeInTheDocument()
        const actions = container.querySelector(".audit-actions")!
        expect(actions.querySelectorAll("a, button")).toHaveLength(1)
        expect(screen.getByRole("link", {name: /auditScreen.voteAgain/})).toHaveAttribute(
            "href",
            "/tenant/tenant/event/event"
        )
    })

    test("shows the three steps, comparing on a hub last", async () => {
        const {container} = await shown()
        const steps = Array.from(container.querySelectorAll(".audit-steps > li"))
        expect(steps.map((step) => step.className.split(" ")[1])).toEqual([
            "audit-step-encrypted",
            "audit-step-opened",
            "audit-step-compare",
        ])
        expect(steps[2]).toHaveAttribute("aria-current", "step")
    })

    test("lists every hub of the policy that checks ballots, each linking to the audited ballot", async () => {
        mockPolicy = policyWith([
            ["h1", "hubco1", "https://h1.verify.example/"],
            ["h2", "hubco2", "https://h2.verify.example/check"],
            ["h3", "hubco3"],
        ])
        const {container} = await shown()
        const links = tiles(container)
        expect(links.map((link) => link.getAttribute("href")).sort()).toEqual([
            audited("https://h1.verify.example/"),
            audited("https://h2.verify.example/check/"),
        ])
        for (const link of links) {
            expect(link).toHaveAttribute("target", "_blank")
            expect(link).toHaveAttribute("rel", "noopener noreferrer")
        }
        expect(container.querySelector(".audit-tracker")).toBeNull()
        expect(screen.getByText("auditScreen.check.footnote")).toBeVisible()
    })

    test("puts the hubs in a random order", async () => {
        mockPolicy = policyWith([
            ["h1", "hubco1", "https://h1.verify.example/"],
            ["h2", "hubco2", "https://h2.verify.example/"],
        ])
        const names = async (random: number) => {
            const spy = jest.spyOn(Math, "random").mockReturnValue(random)
            const view = await shown()
            const order = tiles(view.container).map(
                (link) => link.querySelector(".audit-hub-name")?.textContent
            )
            view.unmount()
            spy.mockRestore()
            return order
        }
        expect(await names(0.999)).toEqual(["h1", "h2"])
        expect(await names(0)).toEqual(["h2", "h1"])
    })

    test("falls back to the election's ballot verifier when no hub checks ballots", async () => {
        mockPolicy = policyWith([["h1", "hubco1"]])
        const {container} = await shown()
        const links = tiles(container)
        expect(links).toHaveLength(1)
        expect(links[0]).toHaveAttribute("href", audited("https://verifier.test/"))
        expect(links[0]).toHaveTextContent("auditScreen.check.authorityName")
        expect(links[0]).toHaveTextContent("auditScreen.check.runByAuthority")
        expect(screen.getByText("auditScreen.check.footnoteAuthority")).toBeVisible()
    })

    test("points at the ballot tracker when there is no hub and no ballot verifier", async () => {
        mockGlobalSettings = {BALLOT_VERIFIER_URL: "", DISABLE_AUTH: false}
        const {container} = await shown()
        expect(tiles(container)).toHaveLength(0)
        expect(screen.getByText(/auditScreen.check.trackerText/)).toBeVisible()
        expect(container.querySelector(".audit-tracker-link")).toHaveAttribute(
            "href",
            `/tenant/tenant/event/event/election/election/ballot-locator/${ballotIdChars(
                MOCK_TRACKING_ID
            )}`
        )
        expect(screen.getByText("auditScreen.check.tipTracker")).toBeVisible()
    })

    test("keeps the audit copy back when the log holds no receipt for the audited entry", async () => {
        logs(EStageCheck.FAILED)
        const {container} = draw()
        expect(await screen.findByText("auditScreen.auditUnavailable")).toBeVisible()
        expect(screen.queryByText(/private-choice-and-randomness/)).not.toBeInTheDocument()
        expect(container.querySelector(".audit-check")).toBeNull()
        // The ballot box refused nothing, but the audit is not whole: keep the receipt.
        expect(mockForgetKey).not.toHaveBeenCalled()
        expect(store.getState().receivedBallots.election).toEqual(received)
    })

    test("accepts the box's audit while the log has only queued the entry", async () => {
        logs(EStageCheck.WAITING)
        const {container} = draw()
        await waitFor(() => expect(mockForgetKey).toHaveBeenCalledWith("election"))
        expect(container.querySelector(".audit-check")).not.toBeNull()
    })

    const publishCall = () =>
        mockAudit.mock.calls.find(
            ([options]) => options.variables.auditableBallot !== undefined
        )?.[0].variables

    test("publishes the auditable ballot only once the box and the log hold the audit", async () => {
        logs(EStageCheck.CHECKED)
        const {container} = draw()
        await waitFor(() => expect(mockAudit).toHaveBeenCalledTimes(2))
        expect(mockAudit.mock.calls[0][0].variables).toEqual({
            electionId: "election",
            ballotId: "FTBE-MHRX",
        })
        const published = publishCall()
        expect(published).toMatchObject({electionId: "election", ballotId: "FTBE-MHRX"})
        expect(JSON.parse(published.auditableBallot)).toMatchObject({
            ballot_hash: "ballot-hash",
            voter_signing_pk: "voter-key",
            plaintext: "private-choice-and-randomness",
        })
        // The first call ran the log check; the publish comes after it.
        expect(mockRunCastReceipt.mock.invocationCallOrder[0]).toBeLessThan(
            mockAudit.mock.invocationCallOrder[1]
        )
        await waitFor(() =>
            expect(container.querySelector(".audit-step-opened")).toHaveClass("audit-step-done")
        )
        expect(container.querySelector(".audit-step-compare")).toHaveAttribute(
            "aria-current",
            "step"
        )
    })

    test("publishes nothing while the log holds no receipt for the audit", async () => {
        logs(EStageCheck.FAILED)
        draw()
        expect(await screen.findByText("auditScreen.auditUnavailable")).toBeVisible()
        expect(mockAudit).toHaveBeenCalledTimes(1)
        expect(publishCall()).toBeUndefined()
    })

    test("says plainly when the secret isn't published, and keeps the ballot audited", async () => {
        logs(EStageCheck.CHECKED)
        mockAudit
            .mockResolvedValueOnce({
                data: {audit_ballot: {ballot_id: "FTBE-MHRX", status: "audited", vitrine: null}},
            })
            .mockRejectedValueOnce(new Error("Network error"))
        const {container} = draw()
        await waitFor(() =>
            expect(container.querySelector(".audit-step-opened")).toHaveClass("audit-step-failed")
        )
        expect(screen.getByText("auditScreen.check.opened.failedTitle")).toBeVisible()
        expect(screen.getByText("auditScreen.check.opened.failedText")).toBeVisible()
        expect(screen.queryByText("auditScreen.check.opened.title")).not.toBeInTheDocument()
        // Nothing to compare on a hub yet; the voter can still vote again.
        expect(tiles(container)).toHaveLength(0)
        expect(container.querySelector(".audit-tracker")).toBeNull()
        expect(container.querySelector(".audit-step-compare")).toHaveClass("audit-step-next")
        expect(screen.getByRole("link", {name: /auditScreen.voteAgain/})).toBeVisible()
        expect(mockForgetKey).toHaveBeenCalledWith("election")
        expect(store.getState().receivedBallots.election).toBeUndefined()
    })

    test("counts a refused auditable ballot as not published", async () => {
        logs(EStageCheck.CHECKED)
        mockAudit.mockResolvedValue({
            data: {
                audit_ballot: {
                    ballot_id: "FTBE-MHRX",
                    status: "audited",
                    publication: "not-published",
                    vitrine: null,
                },
            },
        })
        const {container} = draw()
        await waitFor(() =>
            expect(container.querySelector(".audit-step-opened")).toHaveClass("audit-step-failed")
        )
    })

    test("a retry publishes the same ballot again", async () => {
        logs(EStageCheck.CHECKED)
        mockAudit
            .mockResolvedValueOnce({
                data: {audit_ballot: {ballot_id: "FTBE-MHRX", status: "audited", vitrine: null}},
            })
            .mockRejectedValueOnce(new Error("Network error"))
        const {container} = draw()
        const retry = await screen.findByRole("button", {name: "ballotPrivacy.retry"})
        await act(async () => retry.click())
        await waitFor(() =>
            expect(container.querySelector(".audit-step-opened")).toHaveClass("audit-step-done")
        )
        expect(mockAudit).toHaveBeenCalledTimes(3)
        expect(mockAudit.mock.calls[2][0].variables).toEqual(mockAudit.mock.calls[1][0].variables)
        expect(tiles(container).length + container.querySelectorAll(".audit-tracker").length).toBe(
            1
        )
    })

    test("a retry asks the log again", async () => {
        logs(EStageCheck.FAILED)
        const {container} = draw()
        const retry = await screen.findByRole("button", {name: "ballotPrivacy.retry"})
        logs(EStageCheck.CHECKED)
        await act(async () => retry.click())
        await waitFor(() => expect(mockRunCastReceipt).toHaveBeenCalledTimes(2))
        await waitFor(() => expect(container.querySelector(".audit-check")).not.toBeNull())
    })
})

test("an event with no transparency log asks the log nothing and shows no tag", async () => {
    mockAudit.mockResolvedValue({data: {audit_ballot: {ballot_id: "FTBE-MHRX", status: "audited"}}})
    const {container} = draw()
    await waitFor(() => expect(mockForgetKey).toHaveBeenCalledWith("election"))
    expect(mockRunCastReceipt).not.toHaveBeenCalled()
    // Without a public record the plaintext never leaves the device.
    expect(mockAudit).toHaveBeenCalledTimes(1)
    // No public record or hub to check on: the download is the way to audit.
    expect(container.querySelector(".audit-check")).toBeNull()
    expect(screen.getByText(/private-choice-and-randomness/)).toBeVisible()
    expect(screen.getByRole("button", {name: /auditScreen.downloadButton/})).toBeVisible()
})
