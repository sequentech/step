// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import {MemoryRouter, Route, Routes} from "react-router-dom"
import * as verify from "@vitrine/verify"
import type {OverallStatus, TierName, TierStatus} from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import {wordsFor} from "@vitrine/ui"
import {
    ballotEntry,
    BALLOT_ID,
    TRACKING_ID,
    OTHER_TRACKING_ID,
    chainLink,
    draftPolicyText,
    ELECTION_ID,
    EVENT_ID,
    FakeNet,
    IReceiptScenario,
    LOG_URLS,
    MIRROR_URL,
    MONITOR_URL,
    NOW,
    OTHER_BALLOT_ID,
    OTHER_ELECTION_ID,
    policyText,
    policyUrlOf,
    receiptScenario,
    referenceOf,
    referenceReceiptUrl,
    WITNESSES,
} from "../fixtures/vitrine"
import {MAX_FULL_ROUND_DELAY_MS, MIN_FULL_ROUND_DELAY_MS} from "../services/vitrine/check"
import {CHAIN_MEDIA_TYPE} from "../services/vitrine/input"
import {EPolicyRefusal, examinePolicy, PINNED_POLICIES_KEY} from "../services/vitrine/policies"
import {ETransparencyMode} from "../services/vitrine/presentation"
import type {IVitrineSdk} from "../services/vitrine/sdk"
import {clockTime} from "../components/Vitrine/ReceiptStatus"
import {IReceiptScreenDeps, ReceiptScreen} from "./ReceiptScreen"

// The key, and the values it is given: what the screen says, and with what.
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
        i18n: {language: "en"},
    }),
}))
jest.mock("@sequentech/ui-essentials", () => {
    const React = require("react")
    return {
        PageLimit: ({children}: {children?: React.ReactNode}) =>
            React.createElement("div", null, children),
        ActionsContainer: ({children}: {children?: React.ReactNode}) =>
            React.createElement("div", null, children),
        StyledButton: require("@mui/material/Button").default,
        theme: {
            palette: {
                customGrey: {main: "#000", light: "#eee", dark: "#666", contrastText: "#111"},
                red: {light: "#fcc"},
                green: {light: "#cfc"},
                brandColor: "#00f",
            },
        },
        DropFile: ({handleFiles}: {handleFiles: (files: FileList) => void}) =>
            React.createElement("input", {
                "type": "file",
                "data-testid": "receipt-file",
                "onChange": (event: React.ChangeEvent<HTMLInputElement>) =>
                    event.target.files && handleFiles(event.target.files),
            }),
    }
})

const sdk: IVitrineSdk = {verify, pure}

interface IJob {
    run: () => void
    ms: number
    cancelled: boolean
}

let net: FakeNet
let jobs: IJob[]

beforeAll(async () => {
    await verify.init()
})

beforeEach(() => {
    localStorage.clear()
    net = new FakeNet()
    jobs = []
})

afterEach(() => {
    jest.restoreAllMocks()
})

const deps = (extra: Partial<IReceiptScreenDeps> = {}): Partial<IReceiptScreenDeps> => ({
    loadSdk: async () => sdk,
    fetch: net.fetch,
    now: () => NOW,
    random: () => 0.5,
    schedule: (run, ms) => {
        const job = {run, ms, cancelled: false}
        jobs.push(job)
        return () => {
            job.cancelled = true
        }
    },
    ...extra,
})

const renderScreen = (hash = "", extra?: Partial<IReceiptScreenDeps>) =>
    render(
        <MemoryRouter initialEntries={[{pathname: "/receipt", hash}]}>
            <Routes>
                <Route path="/receipt/*" element={<ReceiptScreen deps={deps(extra)} />} />
            </Routes>
        </MemoryRouter>
    )

const pin = (...texts: string[]) => localStorage.setItem(PINNED_POLICIES_KEY, JSON.stringify(texts))
const pinnedTexts = (): unknown => JSON.parse(localStorage.getItem(PINNED_POLICIES_KEY) ?? "[]")

/** The ballots log answers its checkpoint: the voter's entry and one more. */
const serveLog = (s: IReceiptScenario) =>
    net.serve(`${LOG_URLS.ballots}checkpoint`, s.ballots.checkpoint(2, WITNESSES))

const underLogs = (url: string) =>
    url.startsWith(LOG_URLS.ballots) || url.startsWith(LOG_URLS.events)

const type = (testId: string, value: string) =>
    fireEvent.change(screen.getByTestId(testId), {target: {value}})

const openPasted = async (text: string) => {
    await screen.findByTestId("receipt-paste")
    type("receipt-paste", text)
    fireEvent.click(screen.getByTestId("receipt-open"))
}

const loadPolicy = async (text: string) => {
    await screen.findByTestId("policy-input")
    type("policy-input", text)
    fireEvent.click(screen.getByTestId("policy-load"))
}

const openFile = async (chain: string) => {
    const file = new File([chain], "receipt.vitrine-receipt", {type: CHAIN_MEDIA_TYPE})
    file.text = () => Promise.resolve(chain)
    const input = await screen.findByTestId("receipt-file")
    // The screen reads the file before it opens the receipt.
    await act(async () => {
        fireEvent.change(input, {target: {files: [file]}})
    })
}

/** The status the screen shows for the receipt, once it has a result. */
const statusOf = async (): Promise<HTMLElement> => {
    await screen.findByTestId(/^round-/)
    return screen.getByTestId("receipt-status")
}

/** The verifier, with the result's tiers as the test needs them: the fixtures'
 * policy has an events log that nothing serves, so that no tier verifies. */
const withTiers = (
    status: OverallStatus,
    tiers: Partial<Record<TierName, TierStatus>>
): IVitrineSdk => ({
    pure,
    verify: {
        ...verify,
        checkChain: async (...args: Parameters<typeof verify.checkChain>) => {
            const checked = await verify.checkChain(...args)
            const result = {
                ...checked.result,
                status,
                tiers: checked.result.tiers.map((t) => ({...t, status: tiers[t.tier] ?? t.status})),
            }
            return {...checked, result}
        },
    },
})

const LOG_ROUND_TIERS = {record: "verified", witnesses: "verified", hubs: "pending"} as const

const rowOf = (step: string): HTMLElement => screen.getByTestId(`status-row-${step}`)

describe("the receipt screen", () => {
    it("offers the policy the voting portal's link names, and pins it only when asked", async () => {
        const s = receiptScenario()
        net.serve(policyUrlOf(s), s.policy)
        serveLog(s)
        renderScreen(chainLink(s.chain, s.entry, {url: policyUrlOf(s)}), {
            loadSdk: async () => withTiers("pending", LOG_ROUND_TIERS),
        })

        const offer = await screen.findByTestId("policy-offer")
        const policy = verify.parsePolicy(s.policy)
        // The policy ID in the 13 groups of a Tracking ID, its first four
        // groups set apart, and no words.
        expect(within(offer).getByTestId("policy-id")).toHaveTextContent(
            verify.policyIdDisplay(policy.id)
        )
        expect(within(offer).getByTestId("policy-short-id")).toHaveTextContent(
            verify.shortPolicyId(policy.id)
        )
        expect(within(offer).queryByTestId("policy-words")).toBeNull()
        expect(offer).toHaveTextContent("acme-2026")
        expect(offer).toHaveTextContent("w2")
        // Nothing is checked or kept before the user compares it with the notice.
        expect(pinnedTexts()).toEqual([])
        expect(screen.queryByTestId("receipt-result")).toBeNull()
        expect(net.requests()).toEqual([`GET ${policyUrlOf(s)}`])
        expect(net.calls[0].init).toMatchObject({
            credentials: "omit",
            referrerPolicy: "no-referrer",
            redirect: "error",
        })

        fireEvent.click(screen.getByTestId("policy-pin"))
        expect(await screen.findByTestId("policy-pinned")).toBeInTheDocument()
        expect(pinnedTexts()).toEqual([s.policy])

        // The log round: the record and the witnesses are done, the anchors wait.
        const status = await statusOf()
        expect(status).toHaveAttribute("data-status", "witnessed")
        expect(rowOf("record")).toHaveAttribute("data-state", "done")
        expect(rowOf("record")).toHaveTextContent(
            `receiptScreen.status.rows.recordDone {"index":0}`
        )
        await waitFor(() => expect(rowOf("record")).toHaveTextContent(clockTime(NOW, "en", true)))
        expect(rowOf("witnesses")).toHaveAttribute("data-state", "done")
        expect(rowOf("witnesses")).toHaveTextContent('"signed":2')
        expect(rowOf("witnesses")).toHaveTextContent('"needed":2')
        expect(rowOf("anchors")).toHaveAttribute("data-state", "active")
        expect(rowOf("anchors")).toHaveTextContent("receiptScreen.status.rows.anchorsActive")
        expect(screen.getByTestId("receipt-file-card")).toBeInTheDocument()
        // The link names the entry: no tracking ID to type or change.
        expect(screen.getByTestId("ballot-looked-for")).toHaveTextContent(TRACKING_ID)
        expect(screen.queryByTestId("ballot-form")).toBeNull()
        expect(screen.queryByTestId("ballot-change")).toBeNull()

        // The first round asks only the policy's logs; the full round waits.
        expect(await screen.findByTestId("round-log")).toBeInTheDocument()
        expect(screen.getByTestId("check-everything")).toHaveTextContent(
            "receiptScreen.result.checkEverything"
        )
        const round = net.calls.slice(1)
        expect(round.length).toBeGreaterThan(0)
        expect(round.filter((c) => !underLogs(c.url))).toEqual([])
        expect(jobs).toHaveLength(1)
        expect(jobs[0].ms).toBe((MIN_FULL_ROUND_DELAY_MS + MAX_FULL_ROUND_DELAY_MS) / 2)
    })

    it("runs the full round at its moment, which also asks a mirror and the monitors", async () => {
        const s = receiptScenario()
        pin(s.policy)
        serveLog(s)
        renderScreen(chainLink(s.chain, s.entry))
        await screen.findByTestId("round-log")
        const before = net.calls.length
        expect(
            net.requests().filter((r) => r.includes(MIRROR_URL) || r.includes(MONITOR_URL))
        ).toEqual([])

        await act(async () => jobs[0].run())
        expect(await screen.findByTestId("round-full")).toBeInTheDocument()
        expect(net.requests(before)).toEqual(
            expect.arrayContaining([`GET ${MIRROR_URL}checkpoint`, `POST ${MONITOR_URL}gossip`])
        )
        expect(screen.getByTestId("check-everything")).toHaveTextContent(
            "receiptScreen.result.checkAgain"
        )
        expect(jobs).toHaveLength(1)
    })

    it("runs the full round at once when the user asks", async () => {
        const s = receiptScenario()
        pin(s.policy)
        serveLog(s)
        renderScreen(chainLink(s.chain, s.entry))
        await screen.findByTestId("round-log")

        fireEvent.click(screen.getByTestId("check-everything"))
        expect(await screen.findByTestId("round-full")).toBeInTheDocument()
        expect(jobs[0].cancelled).toBe(true)
        expect(net.requests()).toContain(`GET ${MIRROR_URL}checkpoint`)
    })

    it("refuses a draft policy, showing its draft hash in the groups of a policy ID", async () => {
        const s = receiptScenario()
        renderScreen()
        await openPasted(s.chain)
        await loadPolicy(draftPolicyText())

        const draft = await screen.findByTestId("policy-draft")
        const parsed = pure.parsePolicy(draftPolicyText(), {
            allowDraft: true,
            requireSignatures: false,
        })
        expect(within(draft).getByTestId("policy-draft-id")).toHaveTextContent(
            verify.policyIdDisplay(parsed.id)
        )
        expect(screen.queryByTestId("policy-pin")).toBeNull()
        expect(pinnedTexts()).toEqual([])
        expect(net.calls).toEqual([])
    })

    it("won't pin a policy the receipt isn't for", async () => {
        const s = receiptScenario()
        renderScreen()
        await openPasted(s.chain)
        await loadPolicy(policyText({name: "other-2026"}))

        expect(await screen.findByTestId("policy-other")).toBeInTheDocument()
        expect(screen.queryByTestId("policy-pin")).toBeNull()
        expect(pinnedTexts()).toEqual([])
    })

    it("asks for the policy again when its policy ID doesn't match the notice", async () => {
        const s = receiptScenario()
        net.serve(policyUrlOf(s), s.policy)
        renderScreen(chainLink(s.chain, s.entry, {url: policyUrlOf(s)}))

        fireEvent.click(await screen.findByTestId("policy-reject"))
        expect(await screen.findByTestId("policy-rejected")).toBeInTheDocument()
        expect(screen.getByTestId("policy-input")).toBeInTheDocument()
        expect(pinnedTexts()).toEqual([])
        expect(net.requests()).toEqual([`GET ${policyUrlOf(s)}`])
    })

    it("says when the link's policy can't be fetched", async () => {
        const s = receiptScenario()
        renderScreen(chainLink(s.chain, s.entry, {url: policyUrlOf(s)}))

        expect(await screen.findByTestId("policy-fetch-failed")).toHaveTextContent(
            "receiptScreen.policy.fetchProblems.status"
        )
        expect(screen.getByTestId("policy-input")).toBeInTheDocument()
    })

    it("asks a receipt file for the tracking ID, and checks for that ballot", async () => {
        const s = receiptScenario()
        pin(s.policy)
        serveLog(s)
        renderScreen("", {loadSdk: async () => withTiers("pending", LOG_ROUND_TIERS)})
        await openFile(s.chain)

        const form = await screen.findByTestId("ballot-form")
        expect(screen.getByTestId("policy-pinned")).toBeInTheDocument()
        const event = screen.getByTestId("ballot-event") as HTMLInputElement
        expect(event.value).toBe(EVENT_ID)
        expect(event.readOnly).toBe(true)
        // The receipt never chooses the election: the voter does.
        const election = screen.getByTestId("ballot-election") as HTMLSelectElement
        expect(election.value).toBe("")
        expect(Array.from(election.options).map((o) => o.value)).toEqual([
            "",
            ELECTION_ID,
            OTHER_ELECTION_ID,
        ])
        expect(screen.queryByTestId("receipt-result")).toBeNull()

        type("ballot-id", "not a tracking ID")
        fireEvent.change(election, {target: {value: ELECTION_ID}})
        fireEvent.submit(form)
        expect(await screen.findByTestId("ballot-invalid")).toBeInTheDocument()
        expect(screen.queryByTestId("receipt-result")).toBeNull()

        type("ballot-id", TRACKING_ID.toLowerCase())
        expect((screen.getByTestId("ballot-id") as HTMLInputElement).value).toBe(
            TRACKING_ID
        )
        fireEvent.submit(form)
        expect(await statusOf()).toHaveAttribute("data-status", "witnessed")
        expect(screen.getByTestId("receipt-file-name")).toHaveTextContent("receipt.vitrine-receipt")
        expect(screen.getByTestId("ballot-looked-for")).toHaveTextContent(TRACKING_ID)
        expect(screen.getByTestId("short-ballot-id")).toHaveTextContent("R3ZY-XG7Z")

        fireEvent.click(screen.getByTestId("ballot-change"))
        expect(await screen.findByTestId("ballot-form")).toBeInTheDocument()
        expect(screen.queryByTestId("receipt-result")).toBeNull()
    })

    it("shows that the receipt is for another ballot than the one typed", async () => {
        const s = receiptScenario({electionIds: [ELECTION_ID]})
        pin(s.policy)
        serveLog(s)
        renderScreen()
        await openFile(s.chain)

        const election = (await screen.findByTestId("ballot-election")) as HTMLInputElement
        expect(election.value).toBe(ELECTION_ID)
        expect(election.readOnly).toBe(true)
        type("ballot-id", OTHER_TRACKING_ID)
        fireEvent.submit(screen.getByTestId("ballot-form"))
        expect(await statusOf()).toHaveAttribute("data-status", "problem")
        expect(screen.getByTestId("status-headline")).toHaveTextContent(
            "receiptScreen.status.problem.head"
        )
        expect(rowOf("record")).toHaveAttribute("data-state", "failed")
        expect(rowOf("witnesses")).toHaveAttribute("data-state", "skipped")
        expect(rowOf("anchors")).toHaveAttribute("data-state", "skipped")
        expect(screen.getByTestId("problem-callout")).toBeInTheDocument()
    })

    it("checks a private event's receipt offline, with the policy its link carries", async () => {
        const p = receiptScenario({visibility: ETransparencyMode.PRIVATE})
        renderScreen(chainLink(p.chain, p.entry, {text: p.policy}))

        fireEvent.click(await screen.findByTestId("policy-pin"))
        await statusOf()
        expect(await screen.findByTestId("round-offline")).toBeInTheDocument()
        expect(screen.queryByTestId("check-everything")).toBeNull()
        expect(net.calls).toEqual([])
        expect(jobs).toEqual([])
    })

    it("rebuilds a receipt from its reference, asking only the log", async () => {
        const s = receiptScenario()
        pin(s.policy)
        serveLog(s)
        const ref = referenceOf(s)
        renderScreen(ref.slice(ref.indexOf("#")))

        // The log doesn't know the entry yet.
        const problem = await screen.findByTestId("check-problem")
        expect(problem).toHaveTextContent("receiptScreen.result.problems.not-found")
        expect(net.requests()).toEqual([`GET ${referenceReceiptUrl(s)}`])

        net.serve(referenceReceiptUrl(s), s.receipt)
        fireEvent.click(within(problem).getByRole("button"))
        await statusOf()
        expect(screen.getByTestId("ballot-looked-for")).toHaveTextContent(TRACKING_ID)
        expect(screen.queryByTestId("ballot-form")).toBeNull()
        expect(screen.queryByTestId("check-problem")).toBeNull()
        expect(net.calls.filter((c) => !underLogs(c.url))).toEqual([])
    })

    it("can't rebuild a private event's receipt from its reference", async () => {
        const p = receiptScenario({visibility: ETransparencyMode.PRIVATE})
        pin(p.policy)
        const ref = referenceOf(p)
        renderScreen(ref.slice(ref.indexOf("#")))

        expect(await screen.findByTestId("check-problem")).toHaveTextContent(
            "receiptScreen.result.problems.private-reference"
        )
        expect(net.calls).toEqual([])
    })

    it("treats what it is given as data", async () => {
        const markup = '<img src="x" onerror="window.__vitrinePwned = 1">'
        const s = receiptScenario()
        net.serve(policyUrlOf(s), `<html><body>${markup}</body></html>`)
        renderScreen()
        await openPasted(markup)
        await waitFor(() =>
            expect(screen.getByTestId("receipt-problem")).toHaveTextContent(
                "receiptScreen.receipt.problems.unrecognised"
            )
        )

        // A link can't send the screen to fetch a policy from just anywhere.
        await openPasted(chainLink(s.chain, s.entry, {url: "javascript:alert(1)"}))
        await waitFor(() =>
            expect(screen.getByTestId("receipt-problem")).toHaveTextContent(
                "receiptScreen.receipt.problems.not-https"
            )
        )
        expect(net.calls).toEqual([])

        // What the policy's address serves is read as a policy, never as a page.
        await openPasted(chainLink(s.chain, s.entry, {url: policyUrlOf(s)}))
        const invalid = await screen.findByTestId("policy-invalid")
        const examined = examinePolicy(sdk, `<html><body>${markup}</body></html>`)
        if (examined.ok || examined.refusal !== EPolicyRefusal.INVALID) throw new Error("valid")
        await waitFor(() =>
            expect(invalid).toHaveTextContent(wordsFor("en").reason(examined.reason))
        )
        expect(document.querySelector("img")).toBeNull()
        expect(document.body.innerHTML).not.toContain("onerror")
        expect((window as unknown as {__vitrinePwned?: number}).__vitrinePwned).toBeUndefined()
    })

    it("keeps the pinned policy until the user forgets it", async () => {
        const s = receiptScenario()
        pin(s.policy)
        serveLog(s)
        renderScreen(chainLink(s.chain, s.entry))
        await statusOf()

        fireEvent.click(screen.getByTestId("policy-unpin"))
        expect(await screen.findByTestId("policy-input")).toBeInTheDocument()
        expect(pinnedTexts()).toEqual([])
        expect(screen.queryByTestId("receipt-result")).toBeNull()

        fireEvent.click(screen.getByTestId("receipt-another"))
        expect(await screen.findByTestId("receipt-paste")).toBeInTheDocument()
        expect(screen.queryByTestId("policy-section")).toBeNull()
    })

    it("saves the receipt chain when the user asks for the evidence", async () => {
        const s = receiptScenario()
        pin(s.policy)
        serveLog(s)
        const urls = URL as unknown as {
            createObjectURL?: (blob: Blob) => string
            revokeObjectURL?: (url: string) => void
        }
        const saved: Blob[] = []
        urls.createObjectURL = (blob) => {
            saved.push(blob)
            return "blob:receipt"
        }
        urls.revokeObjectURL = () => undefined
        const names: string[] = []
        jest.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
            this: HTMLAnchorElement
        ) {
            names.push(this.download)
        })
        // The link names another ballot's entry: the record doesn't match.
        renderScreen(chainLink(s.chain, ballotEntry(OTHER_BALLOT_ID)))
        await statusOf()

        expect(screen.queryByTestId("report-advice")).toBeNull()
        fireEvent.click(screen.getByTestId("how-to-report"))
        expect(screen.getByTestId("report-advice")).toHaveTextContent(
            "receiptScreen.result.advice.failed"
        )
        fireEvent.click(screen.getByTestId("save-evidence"))
        expect(await screen.findByTestId("evidence-saved")).toBeInTheDocument()
        expect(saved).toHaveLength(1)
        expect(saved[0].type).toBe(CHAIN_MEDIA_TYPE)
        expect(names).toEqual([verify.chainFileName(`${EVENT_ID}/${ELECTION_ID}`)])
    })

    it("says what to do next when the check is late", async () => {
        const s = receiptScenario()
        pin(s.policy)
        serveLog(s)
        const late: IVitrineSdk = {
            pure,
            verify: {
                ...verify,
                checkChain: async (...args: Parameters<typeof verify.checkChain>) => {
                    const checked = await verify.checkChain(...args)
                    return {...checked, result: {...checked.result, status: "late"}}
                },
            },
        }
        renderScreen(chainLink(s.chain, s.entry), {loadSdk: async () => late})

        expect(await screen.findByTestId("late-callout")).toHaveTextContent(
            "receiptScreen.result.advice.late"
        )
        expect(screen.getByTestId("receipt-status")).toHaveAttribute("data-status", "late")
        expect(screen.queryByTestId("problem-callout")).toBeNull()
    })

    it("says that damaged data isn't evidence, and offers to check again", async () => {
        const s = receiptScenario()
        pin(s.policy)
        serveLog(s)
        const damaged = withTiers("invalid", {record: "verified", witnesses: "invalid"})
        renderScreen(chainLink(s.chain, s.entry), {loadSdk: async () => damaged})

        await statusOf()
        expect(screen.getByTestId("status-headline")).toHaveTextContent(
            "receiptScreen.status.damaged.head"
        )
        expect(rowOf("record")).toHaveAttribute("data-state", "done")
        expect(rowOf("witnesses")).toHaveAttribute("data-state", "damaged")
        expect(rowOf("anchors")).toHaveAttribute("data-state", "skipped")
        expect(screen.queryByTestId("problem-callout")).toBeNull()
        expect(screen.getByTestId("check-everything")).toBeInTheDocument()
    })

    it("tells the receipt's file apart from the ballot it is checked for", async () => {
        const s = receiptScenario()
        pin(s.policy)
        serveLog(s)
        renderScreen(chainLink(s.chain, s.entry))
        await statusOf()

        expect(screen.getByTestId("receipt-file-name")).toHaveTextContent(
            "receiptScreen.receipt.pasteLabel"
        )
        expect(screen.getByTestId("receipt-file-card")).toHaveTextContent("acme-2026")
        expect(screen.getByTestId("full-ballot-id").querySelectorAll("b")).toHaveLength(2)
    })

    it("says so when the verifier can't be loaded", async () => {
        renderScreen("", {loadSdk: () => Promise.reject(new Error("offline"))})
        expect(await screen.findByTestId("receipt-load-failed")).toBeInTheDocument()
    })
})
