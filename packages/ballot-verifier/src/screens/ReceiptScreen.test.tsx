// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import {MemoryRouter, Route, Routes} from "react-router-dom"
import * as verify from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import {wordsFor, type VitrineProofSteps, type VitrineReceipt} from "@vitrine/ui"
import {
    BALLOT_ID,
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
import {IReceiptScreenDeps, ReceiptScreen} from "./ReceiptScreen"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@sequentech/ui-essentials", () => {
    const React = require("react")
    return {
        PageLimit: ({children}: {children?: React.ReactNode}) =>
            React.createElement("div", null, children),
        theme: {palette: {customGrey: {main: "#000"}}},
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

/** The UI kit's receipt, once the page has given it a result. */
const receiptElement = async (): Promise<VitrineReceipt> => {
    let element: VitrineReceipt | null = null
    await waitFor(() => {
        element = document.querySelector("vitrine-receipt")
        expect(element?.result).toBeTruthy()
    })
    return element as unknown as VitrineReceipt
}

const issuedOf = (chain: unknown): string | undefined => {
    const issued = pure.parseChain(String(chain)).issued
    return issued && pure.bytes.fromUtf8(issued)
}

describe("the receipt screen", () => {
    it("offers the policy the voting portal's link names, and pins it only when asked", async () => {
        const s = receiptScenario()
        net.serve(policyUrlOf(s), s.policy)
        serveLog(s)
        renderScreen(chainLink(s.chain, s.entry, {url: policyUrlOf(s)}))

        const offer = await screen.findByTestId("policy-offer")
        const policy = verify.parsePolicy(s.policy)
        expect(within(offer).getByTestId("policy-words")).toHaveTextContent(
            verify.policyWords(policy).join(" ")
        )
        expect(within(offer).getByTestId("policy-id")).toHaveTextContent(s.policyId)
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

        const element = await receiptElement()
        expect(element.getAttribute("where")).toBe("independent")
        expect(element.state).toBe("done")
        expect(element.result?.where).toBe("independent")
        expect(element.result?.ok).toBe(true)
        expect(issuedOf(element.chain)).toBe(s.receipt)
        expect(element.expect).toEqual({entry: s.entry})
        const steps = document.querySelector("vitrine-proof-steps") as VitrineProofSteps | null
        expect(steps?.result).toBe(element.result)
        // The link names the entry: no ballot ID to type.
        expect(screen.getByTestId("ballot-looked-for")).toHaveTextContent(BALLOT_ID)
        expect(screen.queryByTestId("ballot-section")).toBeNull()

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

    it("refuses a draft policy, showing its eight draft words", async () => {
        const s = receiptScenario()
        renderScreen()
        await openPasted(s.chain)
        await loadPolicy(draftPolicyText())

        const draft = await screen.findByTestId("policy-draft")
        const words = pure.policies.draftWords(
            pure.parsePolicy(draftPolicyText(), {allowDraft: true, requireSignatures: false})
        )
        expect(within(draft).getByTestId("policy-draft-words")).toHaveTextContent(words.join(" "))
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

    it("asks for the policy again when its words don't match the notice", async () => {
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

    it("asks a receipt file for the ballot ID, and checks for that ballot", async () => {
        const s = receiptScenario()
        pin(s.policy)
        serveLog(s)
        renderScreen()
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

        type("ballot-id", "not a ballot ID")
        fireEvent.change(election, {target: {value: ELECTION_ID}})
        fireEvent.submit(form)
        expect(await screen.findByTestId("ballot-invalid")).toBeInTheDocument()
        expect(screen.queryByTestId("receipt-result")).toBeNull()

        type("ballot-id", BALLOT_ID.toUpperCase())
        fireEvent.submit(form)
        const element = await receiptElement()
        expect(element.result?.ok).toBe(true)
        expect(element.expect).toEqual({
            fields: {event_id: EVENT_ID, election_id: ELECTION_ID, ballot_id: BALLOT_ID},
        })
        expect(screen.getByTestId("ballot-looked-for")).toHaveTextContent(BALLOT_ID)

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
        type("ballot-id", OTHER_BALLOT_ID)
        fireEvent.submit(screen.getByTestId("ballot-form"))
        const element = await receiptElement()
        expect(element.result?.ok).toBe(false)
        const record = element.result?.tiers.find((t) => t.tier === "record")
        expect(record?.reasons.map((r) => r.code)).toContain("entry-mismatch")
    })

    it("checks a private event's receipt offline, with the policy its link carries", async () => {
        const p = receiptScenario({visibility: ETransparencyMode.PRIVATE})
        renderScreen(chainLink(p.chain, p.entry, {text: p.policy}))

        fireEvent.click(await screen.findByTestId("policy-pin"))
        const element = await receiptElement()
        expect(element.result?.where).toBe("independent")
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
        const element = await receiptElement()
        expect(issuedOf(element.chain)).toBe(s.receipt)
        expect(screen.getByTestId("ballot-looked-for")).toHaveTextContent(BALLOT_ID)
        expect(screen.queryByTestId("ballot-section")).toBeNull()
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
        await receiptElement()

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
        renderScreen(chainLink(s.chain, s.entry))
        await receiptElement()

        const steps = document.querySelector("vitrine-proof-steps")
        act(() => {
            steps?.dispatchEvent(
                new CustomEvent("vitrine-evidence", {detail: {step: "record", reasons: []}})
            )
        })
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

        expect(await screen.findByTestId("check-advice")).toHaveTextContent(
            "receiptScreen.result.advice.late"
        )
    })

    it("says so when the verifier can't be loaded", async () => {
        renderScreen("", {loadSdk: () => Promise.reject(new Error("offline"))})
        expect(await screen.findByTestId("receipt-load-failed")).toBeInTheDocument()
    })
})
