// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, waitFor} from "@testing-library/react"
import {ThemeProvider} from "@mui/material/styles"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import * as pure from "@vitrine/verify/pure"
import type {VitrineReceipt, VitrineStatus} from "@vitrine/ui"
import theme from "../../../ui-essentials/src/services/theme"
import {
    ballotEntry,
    ELECTION_ID,
    EVENT_ID,
    KEYS,
    LOG_URLS,
    NOW,
    scenario,
} from "../fixtures/vitrine"
import type {THttpInit} from "../services/vitrine/http"
import {readTransparency} from "../services/vitrine/presentation"
import type {IVitrineCast} from "../services/vitrine/types"
import type {IConfirmationVitrine} from "../store/castVotes/confirmationScreenDataSlice"
import {RootState, store} from "../store/store"
import ConfirmationScreen from "./ConfirmationScreen"

jest.mock("react-i18next", () => ({
    Trans: ({i18nKey}: {i18nKey: string}) => i18nKey,
    useTranslation: () => ({
        t: (key: string, values?: {ballotId?: string}) =>
            key === "ballotHash" ? `Ballot ID: ${values?.ballotId?.slice(0, 8)}` : key,
        i18n: {language: "en"},
    }),
}))
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    ...jest.requireActual("../../../ui-core/src/services/acclamation"),
    downloadUrl: jest.fn(),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        PageLimit: jest.requireActual("../../../ui-essentials/src/components/PageLimit/PageLimit")
            .default,
        Icon: jest.requireActual("../../../ui-essentials/src/components/Icon/Icon").default,
        IconButton: jest.requireActual(
            "../../../ui-essentials/src/components/IconButton/IconButton"
        ).default,
        DecorativeIconBox: jest.requireActual(
            "../../../ui-essentials/src/components/Icon/DecorativeIconBox"
        ).default,
        VisuallyHidden: jest.requireActual(
            "../../../ui-essentials/src/components/VisuallyHidden/VisuallyHidden"
        ).default,
        ...jest.requireActual(
            "../../../ui-essentials/src/components/ConfirmationActions/ConfirmationActions"
        ),
        BallotHashCopyButton: jest.requireActual(
            "../../../ui-essentials/src/components/BallotHash/BallotHash"
        ).BallotHashCopyButton,
        theme: jest.requireActual("../../../ui-essentials/src/services/theme").default,
        Dialog: () => null,
        QRCode: jest.requireActual("../../../ui-essentials/src/components/QRCode/QRCode").default,
    }),
    {virtual: true}
)
jest.mock("../store/hooks", () => ({
    useAppSelector: (selector: (state: RootState) => unknown) => selector(mockState),
    useAppDispatch: () => jest.fn(),
}))
jest.mock("../providers/AuthContextProvider", () => ({
    AuthContext: jest.requireActual<typeof React>("react").createContext({
        logout: jest.fn(),
        isKiosk: () => mockIsKiosk,
        userId: "voter-1",
    }),
}))
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: jest.requireActual<typeof React>("react").createContext({
        globalSettings: {
            DISABLE_AUTH: true,
            QUERY_POLL_INTERVAL_MS: 1000,
            BALLOT_VERIFIER_URL: "https://verifier.example/",
        },
    }),
}))
jest.mock("../services/BallotService", () => ({
    provideBallotService: () => ({hashBallot: jest.fn(), hashMultiBallot: jest.fn()}),
}))
jest.mock("../hooks/public-document-url", () => ({
    useGetPublicDocumentUrl: () => ({getDocumentUrl: jest.fn()}),
}))
jest.mock("../hooks/useVoterContext", () => ({useVoterContext: () => ({loading: false})}))
jest.mock("../components/Stepper", () => ({__esModule: true, default: () => null}))
jest.mock("@apollo/client/react", () => {
    // Apollo gives every component the same client.
    const client = {}
    return {
        useMutation: () => [jest.fn()],
        useQuery: () => ({data: undefined, startPolling: jest.fn(), stopPolling: jest.fn()}),
        useApolloClient: () => client,
    }
})
// Receipts kept on the device: what the screen asks to keep, and nothing else.
jest.mock("../services/vitrine/storage", () => {
    const actual = jest.requireActual("../services/vitrine/storage")
    return {
        ...actual,
        receiptKeeping: () => actual.EReceiptKeeping.KEEP,
        setReceiptKeeping: jest.fn(),
        keepReceipt: (...args: unknown[]) => mockKeepReceipt(...args),
        forgetReceipt: jest.fn(async () => undefined),
    }
})

// Saving a file makes no request: the page only has to ask for it.
jest.mock("../services/vitrine/file", () => ({
    ...jest.requireActual("../services/vitrine/file"),
    saveTextFile: (...args: unknown[]) => mockSaveTextFile(...args),
}))

let mockState: RootState
let mockIsKiosk = false
const mockKeepReceipt = jest.fn(async (..._args: unknown[]) => true)
const mockSaveTextFile = jest.fn((..._args: unknown[]) => undefined)

const WITNESSES = KEYS.witnesses.slice(0, 2)
const PATH = `/tenant/tenant-1/event/${EVENT_ID}/election/${ELECTION_ID}`

// Each test casts its own ballot: observations go once per ballot and page load.
let ballots = 0
const nextBallotId = (): string => (0xbb00 + ++ballots).toString(16).padStart(64, "0")

let calls: {url: string; method: string}[] = []
const savedFetch = global.fetch

beforeAll(() => {
    // The logs answer nothing, the monitors take everything.
    global.fetch = jest.fn(async (input: string, init?: THttpInit) => {
        const method = init?.method ?? "GET"
        calls.push({url: String(input), method})
        return {
            status: method === "POST" ? 202 : 404,
            headers: {get: () => null},
            body: null,
            arrayBuffer: async () => new ArrayBuffer(0),
        } as unknown as Response
    }) as typeof fetch
})

afterAll(() => {
    global.fetch = savedFetch
})

beforeEach(() => {
    calls = []
    mockIsKiosk = false
    mockKeepReceipt.mockClear()
    jest.spyOn(Date, "now").mockReturnValue(NOW * 1000)
})

afterEach(() => {
    jest.restoreAllMocks()
})

const confirm = (
    ballotId: string,
    vitrine: IConfirmationVitrine | undefined,
    isDemo = false
): RootState => ({
    ...store.getState(),
    confirmationScreenData: {
        [ELECTION_ID]: {ballotId, isDemo, ...(vitrine ? {vitrine} : {})},
    },
})

const renderScreen = () => {
    const router = createMemoryRouter(
        [
            {
                path: "/tenant/:tenantId/event/:eventId/election/:electionId/*",
                element: <ConfirmationScreen />,
            },
            {
                path: "/tenant/:tenantId/event/:eventId/election-chooser",
                element: <div>Chooser</div>,
            },
        ],
        {initialEntries: [`${PATH}/confirmation`]}
    )
    return render(
        <ThemeProvider theme={theme}>
            <RouterProvider router={router} />
        </ThemeProvider>
    )
}

/** A public event whose log holds the voter's entry, and Harvest's answer. */
const casting = (answer: (s: ReturnType<typeof scenario>, ballotId: string) => IVitrineCast) => {
    const s = scenario()
    const ballotId = nextBallotId()
    const cast = answer(s, ballotId)
    mockState = confirm(ballotId, {cast, transparency: readTransparency(s.presentation)})
    return {s, ballotId, cast}
}

const receiptOf = (s: ReturnType<typeof scenario>, ballotId: string): IVitrineCast => {
    const index = s.ballots.add(ballotEntry(ballotId))
    s.ballots.add(ballotEntry(nextBallotId()))
    return {kind: "receipt", text: s.ballots.receipt(index, 2, WITNESSES)}
}

const promiseOf = (s: ReturnType<typeof scenario>, ballotId: string): IVitrineCast => {
    const index = s.ballots.add(ballotEntry(ballotId))
    return {kind: "promise", text: s.ballots.promise(index, [index], s.policyId, NOW - 2)}
}

const entryOf =
    (kind: string, reason?: string) =>
    (_s: ReturnType<typeof scenario>, ballotId: string): IVitrineCast => ({
        kind,
        text: pure.encoding.b64encode(ballotEntry(ballotId)),
        ...(reason ? {reason} : {}),
    })

const receiptElement = async (container: HTMLElement): Promise<VitrineReceipt> => {
    let element: VitrineReceipt | null = null
    await waitFor(() => {
        element = container.querySelector("vitrine-receipt")
        expect(element).not.toBeNull()
    })
    return element as unknown as VitrineReceipt
}

const shadowText = async (element: VitrineReceipt | VitrineStatus): Promise<string> => {
    await element.updateComplete
    return element.shadowRoot?.textContent ?? ""
}

const missingFailure = async (container: HTMLElement): Promise<HTMLElement> => {
    let failure: HTMLElement | null = null
    await waitFor(() => {
        failure = container.querySelector<HTMLElement>(".vitrine-failure")
        expect(failure?.getAttribute("data-notice")).toBe("missing")
    })
    return failure as unknown as HTMLElement
}

describe("the receipt on the confirmation screen", () => {
    it.each([
        ["receipt", receiptOf, "issued", true],
        ["promise", promiseOf, "promise", false],
    ] as const)("shows the %s, checked by this website", async (_kind, answer, block, qr) => {
        const {cast} = casting(answer)
        const {container} = renderScreen()
        const element = await receiptElement(container)
        await waitFor(() => expect(element.state).toBe("done"))
        expect(element.result?.ok).toBe(true)
        const chain = pure.parseChain(String(element.chain))
        expect(pure.bytes.fromUtf8(chain[block] ?? new Uint8Array())).toBe(cast.text)
        const text = await shadowText(element)
        expect(text).toContain("Checked by this website")
        expect(text).toContain("Check independently")
        expect(container.querySelector("vitrine-status")).toBeNull()
        // A receipt's reference is drawn as a QR code here; a promise has none yet.
        if (qr) {
            await waitFor(() =>
                expect(container.querySelector(".vitrine-receipt-reference svg")).not.toBeNull()
            )
        } else {
            expect(container.querySelector(".vitrine-receipt-reference")).toBeNull()
        }
        await waitFor(() => expect(mockKeepReceipt).toHaveBeenCalled())
        expect(mockKeepReceipt.mock.calls[0][2]).toMatchObject({
            eventId: EVENT_ID,
            electionId: ELECTION_ID,
            chain: element.chain,
        })
    })

    it.each([
        ["queued", entryOf("queued", "busy")],
        ["unavailable", entryOf("unavailable", "log-down")],
    ] as const)("says there is no receipt when the entry was %s", async (_kind, answer) => {
        casting(answer)
        const {container} = renderScreen()
        const failure = await missingFailure(container)
        expect(failure.querySelector(".vitrine-failure-title")?.textContent).toBe(
            "vitrine.failure.titlePublic"
        )
        expect(failure.querySelector(".vitrine-failure-explanation")?.textContent).toBe(
            "vitrine.receipt.missing"
        )
        // The page owes the voter no evidence it doesn't have.
        expect(failure.querySelector(".vitrine-failure-evidence")).toBeNull()
        const element = await receiptElement(container)
        expect(element.state).toBe("empty")
        expect(await shadowText(element)).toContain("No receipt yet.")
        // The entry is kept until the log answers for it.
        await waitFor(() => expect(mockKeepReceipt).toHaveBeenCalled())
        expect(mockKeepReceipt.mock.calls[0][2]).not.toHaveProperty("chain")
        expect(
            calls.every((c) => !c.url.includes("/receipt/") || c.url.startsWith(LOG_URLS.ballots))
        ).toBe(true)
    })

    it("puts the receipt first, and the ballot ID and verification behind a disclosure", async () => {
        casting(receiptOf)
        const {container} = renderScreen()
        await receiptElement(container)
        const receiptBlock = container.querySelector(".vitrine-receipt-container")
        const details = container.querySelector(".vitrine-ballot-details")
        expect(receiptBlock?.compareDocumentPosition(details!)).toBe(
            Node.DOCUMENT_POSITION_FOLLOWING
        )
        expect(details?.querySelector(".ballot-id-container")).not.toBeNull()
        expect(details?.querySelector(".ballot-verification-title")).not.toBeNull()
    })

    it("shows the steps, and says the ballot is cast until the witnesses confirm it", async () => {
        casting(receiptOf)
        const {container} = renderScreen()
        const element = await receiptElement(container)
        expect(container.querySelector(".vitrine-progress-label")?.textContent).toContain(
            "vitrine.progress"
        )
        expect(container.querySelectorAll(".vitrine-progress-bar")).toHaveLength(4)
        await waitFor(() => expect(element.state).toBe("done"))
        const witnessed = element.result?.tiers.find((t) => t.tier === "witnesses")?.status
        const title = container.querySelector(".vitrine-confirmation-title")
        await waitFor(() =>
            expect(title?.textContent).toBe(
                witnessed === "verified"
                    ? "vitrine.confirmation.recordedPublic"
                    : "vitrine.confirmation.cast"
            )
        )
        expect(container.querySelector(".vitrine-receipt-description")?.textContent).toBe(
            "vitrine.confirmation.keep"
        )
    })

    it("offers the QR code on another device behind a toggle", async () => {
        casting(receiptOf)
        const {container} = renderScreen()
        await receiptElement(container)
        const toggle = await waitFor(() => {
            const button = container.querySelector<HTMLButtonElement>(".vitrine-elsewhere-toggle")
            expect(button).not.toBeNull()
            return button!
        })
        const panel = container.querySelector<HTMLElement>(".vitrine-elsewhere-panel")
        expect(toggle.getAttribute("aria-expanded")).toBe("false")
        expect(panel?.hidden).toBe(true)
        fireEvent.click(toggle)
        expect(toggle.getAttribute("aria-expanded")).toBe("true")
        expect(panel?.hidden).toBe(false)
        expect(panel?.querySelector("svg")).not.toBeNull()
    })

    it("saves the receipt chain as a file", async () => {
        casting(receiptOf)
        const {container} = renderScreen()
        const element = await receiptElement(container)
        await waitFor(() => expect(element.state).toBe("done"))
        const button = container.querySelector<HTMLButtonElement>(".save-receipt-button")
        await waitFor(() => expect(button?.disabled).toBe(false))
        fireEvent.click(button!)
        expect(mockSaveTextFile).toHaveBeenCalledTimes(1)
        const [name, text, type] = mockSaveTextFile.mock.calls[0]
        expect(name).toMatch(/^receipt-[A-Za-z0-9]{8}\.vitrine-receipt$/)
        expect(text).toBe(element.chain)
        expect(type).toBe("application/vnd.vitrine.receipt-chain")
    })

    it("offers no receipt to save when none came", async () => {
        casting(entryOf("queued", "busy"))
        const {container} = renderScreen()
        await missingFailure(container)
        expect(container.querySelector(".save-receipt-button")).toBeNull()
        expect(container.querySelector(".finish-button")).not.toBeNull()
    })

    it("keeps nothing on a kiosk", async () => {
        mockIsKiosk = true
        casting(receiptOf)
        const {container} = renderScreen()
        const element = await receiptElement(container)
        await waitFor(() => expect(element.state).toBe("done"))
        expect(container.querySelector(".vitrine-receipt-storage")).toBeNull()
        // The voter can still take the receipt away by scanning it.
        await waitFor(() =>
            expect(container.querySelector(".vitrine-receipt-reference svg")).not.toBeNull()
        )
        expect(mockKeepReceipt).not.toHaveBeenCalled()
    })

    it("treats Harvest's text as data", async () => {
        const markup = '<img src="x" onerror="window.__vitrinePwned = 1"><script>1</script>'
        casting(() => ({kind: "receipt", text: markup, reason: markup}))
        const {container} = renderScreen()
        await missingFailure(container)
        const element = await receiptElement(container)
        const roots = [
            document.body,
            ...Array.from(document.querySelectorAll("vitrine-receipt, vitrine-status")).map(
                (e) => e.shadowRoot
            ),
        ]
        for (const root of roots) {
            expect(root?.innerHTML).not.toContain("onerror")
            expect(root?.querySelector("img, script")).toBeNull()
        }
        expect(element.chain).toBeNull()
        expect((window as unknown as Record<string, unknown>).__vitrinePwned).toBeUndefined()
    })

    it("opens the deployment's verifier with the chain in the fragment", async () => {
        const {s} = casting(receiptOf)
        const open = jest.spyOn(window, "open").mockImplementation(() => null)
        const {container} = renderScreen()
        const element = await receiptElement(container)
        await waitFor(() => expect(element.state).toBe("done"))
        await element.updateComplete
        const button = Array.from(element.shadowRoot?.querySelectorAll("button") ?? []).find((b) =>
            b.textContent?.includes("Check independently")
        )
        button?.click()
        await waitFor(() => expect(open).toHaveBeenCalled())
        const [url, target, features] = open.mock.calls[0]
        expect(target).toBe("_blank")
        expect(features).toBe("noopener,noreferrer")
        const link = new URL(String(url))
        expect(`${link.origin}${link.pathname}`).toBe("https://verifier.example/receipt")
        expect(link.hash.startsWith("#vitrine:chain?")).toBe(true)
        const params = new URLSearchParams(link.hash.slice("#vitrine:chain?".length))
        expect(pure.bytes.fromUtf8(pure.encoding.b64urlDecode(params.get("c") ?? ""))).toBe(
            element.chain
        )
        expect(params.get("u")).toBe(`${LOG_URLS.ballots}policy/${s.policyId}`)
        // Nothing about the receipt reached a server.
        expect(calls.some((c) => c.url.includes("verifier.example"))).toBe(false)
    })

    it.each([
        ["events without transparency", undefined, false],
        [
            "demo ballots",
            {cast: null, transparency: readTransparency(scenario().presentation)},
            true,
        ],
    ] as const)("shows nothing for %s", async (_name, vitrine, isDemo) => {
        mockState = confirm(nextBallotId(), vitrine, isDemo)
        const {container, findAllByTestId} = renderScreen()
        await findAllByTestId("ballot-id")
        expect(container.querySelector(".vitrine-receipt-container")).toBeNull()
        expect(container.querySelector("vitrine-receipt")).toBeNull()
    })
})
