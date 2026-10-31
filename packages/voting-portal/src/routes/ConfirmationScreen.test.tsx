// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import {ThemeProvider} from "@mui/material/styles"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import * as pure from "@vitrine/verify/pure"
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
        t: (key: string, values?: {ballotId?: string}) => {
            if (key === "ballotHash") return `Tracking ID: ${values?.ballotId?.slice(0, 8)}`
            return values && key.startsWith("vitrine.status.aria")
                ? `${key} ${JSON.stringify(values)}`
                : key
        },
        i18n: {language: "en"},
    }),
}))
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    ...jest.requireActual("../../../ui-core/src/services/acclamation"),
    ...jest.requireActual("../../../ui-core/src/services/votingPortalDateTime"),
    downloadUrl: jest.fn(),
}))
jest.mock("@sequentech/ui-essentials", () => ({
    ...jest.requireActual("../../../ui-essentials/src/index"),
    Dialog: () => null,
}))
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
    // The screen reads the ballot's Tracking ID, whose digest the entries name.
    const tracking = pure.trackingId(ballotId)
    mockState = confirm(tracking, {cast, transparency: readTransparency(s.presentation)})
    return {s, ballotId: tracking, cast}
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

const statusButton = (container: HTMLElement): HTMLButtonElement | null =>
    container.querySelector<HTMLButtonElement>(".ballot-status-button")

const stateOf = async (container: HTMLElement, expected: string): Promise<void> =>
    waitFor(() => expect(statusButton(container)?.getAttribute("data-status")).toBe(expected))

const GROUPED_PATTERN = /^([0-9A-Z]{4}-){12}[0-9A-Z]{4}$/

/** Whether a rule of the page's styles, in any media query, hides the element.
 * jsdom applies no media query, so this is how a hiding at one width shows. */
const hiddenByAnyRule = (element: Element): boolean => {
    const hides = (rules: CSSRuleList): boolean =>
        Array.from(rules).some((rule) =>
            rule.type === CSSRule.MEDIA_RULE
                ? hides((rule as CSSMediaRule).cssRules)
                : rule.type === CSSRule.STYLE_RULE &&
                  (rule as CSSStyleRule).style.display === "none" &&
                  element.matches((rule as CSSStyleRule).selectorText)
        )
    return Array.from(document.styleSheets).some((sheet) => hides(sheet.cssRules))
}

describe("the Tracking ID on the confirmation screen", () => {
    it("keeps the portal's own layout, with the status check mark in the ID box", async () => {
        casting(receiptOf)
        const {container} = renderScreen()
        await waitFor(() => expect(statusButton(container)).not.toBeNull())
        expect(container.querySelector(".screen-title")).not.toBeNull()
        expect(container.querySelector(".ballot-id-container")).not.toBeNull()
        expect(container.querySelector(".ballot-verification-title")).not.toBeNull()
        expect(container.querySelector(".qr-container svg")).not.toBeNull()
        const actions = container.querySelector(".actions-container")
        expect(actions?.querySelector(".print-receipt-button")).not.toBeNull()
        expect(actions?.querySelector(".finish-button")).not.toBeNull()
        expect(container.querySelector(".ballot-id-check-icon")).toBeNull()
        expect(container.querySelector("[class*=vitrine-]")).toBeNull()
    })

    it("shows the short ID once, with no prefix for a phone, and copies the full one in groups of four", async () => {
        const {ballotId} = casting(receiptOf)
        const writeText = jest.fn(async (_text: string) => undefined)
        Object.defineProperty(navigator, "clipboard", {value: {writeText}, configurable: true})
        const {container, findAllByTestId} = renderScreen()
        const links = await findAllByTestId("ballot-id")
        expect(links).toHaveLength(1)
        expect(links[0].textContent).toBe("0000-0000")
        expect(container.querySelector(".ballot-id-value-mobile")).toBeNull()
        expect(container.querySelector(".ballot-id-prefix")).toBeNull()
        await act(async () => {
            fireEvent.click(screen.getByRole("button", {name: "vitrine.ballotId.copy"}))
        })
        expect(writeText).toHaveBeenCalledTimes(1)
        const copied = writeText.mock.calls[0][0]
        expect(copied).toMatch(GROUPED_PATTERN)
        expect(copied).toBe(ballotId)
    })

    it("links the ID to the ballot tracker", async () => {
        casting(receiptOf)
        const {findByTestId} = renderScreen()
        const link = await findByTestId("ballot-id")
        await waitFor(() =>
            expect(link.getAttribute("href")).toContain(`/election/${ELECTION_ID}/ballot-locator/`)
        )
    })

    it("heads the box with Tracking ID on every screen size", async () => {
        casting(receiptOf)
        const {container} = renderScreen()
        await waitFor(() => expect(statusButton(container)).not.toBeNull())
        const label = container.querySelector<HTMLElement>(".ballot-id-label")
        expect(label?.tagName).toBe("H2")
        expect(label?.textContent).toBe("confirmationScreen.ballotId")
        expect(hiddenByAnyRule(label!)).toBe(false)
    })

    it("follows the short ID with the tag Cast, in one row with it", async () => {
        casting(receiptOf)
        const {container, findByTestId} = renderScreen()
        const link = await findByTestId("ballot-id")
        const tags = container.querySelectorAll(".status-tag")
        expect(tags).toHaveLength(1)
        expect(tags[0].textContent).toBe("vitrine.status.tag.cast")
        expect(tags[0].getAttribute("data-tone")).toBe("success")
        const row = container.querySelector(".ballot-id-text")!
        expect(Array.from(row.children)).toEqual([link, tags[0]])
        expect(row.parentElement?.className).toContain("ballot-id-border")
    })

    it("swaps the copy icon for a green check, with no text, when it copies", async () => {
        casting(receiptOf)
        Object.defineProperty(navigator, "clipboard", {
            value: {writeText: jest.fn(async (_text: string) => undefined)},
            configurable: true,
        })
        const {container} = renderScreen()
        const copy = await screen.findByRole("button", {name: "vitrine.ballotId.copy"})
        expect(copy.querySelector("[data-icon=copy]")).not.toBeNull()
        await act(async () => {
            fireEvent.click(copy)
        })
        const done = screen.getByRole("button", {name: "vitrine.ballotId.copied"})
        expect(done.querySelector("[data-icon=copy]")).toBeNull()
        expect(done.querySelector("[data-icon=check]")).not.toBeNull()
        expect(getComputedStyle(done).color).toBe("rgb(4, 120, 87)")
        expect(container.querySelector(".ballot-id-container")?.textContent).not.toContain("Copied")
    })

    it("rounds the box's corners to 8 px", async () => {
        casting(receiptOf)
        const {container} = renderScreen()
        await waitFor(() => expect(statusButton(container)).not.toBeNull())
        expect(getComputedStyle(container.querySelector(".ballot-id-border")!).borderRadius).toBe(
            "8px"
        )
    })
})

/** Opens the dialog, and waits for the receipt to be saveable: its chain is there. */
const openWithReceipt = async (container: HTMLElement): Promise<HTMLElement> => {
    await waitFor(() => expect(statusButton(container)).not.toBeNull())
    fireEvent.click(statusButton(container)!)
    const dialog = await screen.findByRole("dialog", {name: "vitrine.status.title"})
    const save = within(dialog).getByRole("button", {name: "vitrine.status.save"})
    await waitFor(() => expect(save.hasAttribute("disabled")).toBe(false))
    return dialog
}

describe("the status of the ballot just cast", () => {
    it.each([
        ["receipt", receiptOf, "issued"],
        ["promise", promiseOf, "promise"],
    ] as const)("is checked from the %s, and kept on this device", async (_kind, answer, block) => {
        const {cast} = casting(answer)
        const {container} = renderScreen()
        await waitFor(() => expect(mockKeepReceipt).toHaveBeenCalled())
        const kept = mockKeepReceipt.mock.calls[0][2] as {chain: string}
        const chain = pure.parseChain(kept.chain)
        expect(pure.bytes.fromUtf8(chain[block] ?? new Uint8Array())).toBe(cast.text)
        expect(kept).toMatchObject({eventId: EVENT_ID, electionId: ELECTION_ID})
        // This fake log serves no tiles, so the record is still waiting for its successors.
        expect(statusButton(container)?.getAttribute("data-status")).toBe("checking")
    })

    it("opens the ballot's status in a dialog, with the tracker link", async () => {
        casting(receiptOf)
        const {container} = renderScreen()
        const dialog = await openWithReceipt(container)
        expect(dialog.querySelector(".ballot-status-head")?.textContent).toBe(
            "vitrine.status.state.checking"
        )
        await waitFor(() =>
            expect(dialog.querySelector(".ballot-status-meta")?.textContent).toContain(
                "vitrine.status.cast"
            )
        )
        expect(
            within(dialog).getByRole("link", {name: "vitrine.status.tracker"}).getAttribute("href")
        ).toContain("/ballot-locator/")
    })

    it("saves the receipt chain as a file named after the short ID", async () => {
        casting(receiptOf)
        const {container} = renderScreen()
        const dialog = await openWithReceipt(container)
        fireEvent.click(within(dialog).getByRole("button", {name: "vitrine.status.save"}))
        expect(mockSaveTextFile).toHaveBeenCalledTimes(1)
        const [name, text, type] = mockSaveTextFile.mock.calls[0]
        expect(name).toBe("ballot-0000-0000.vitrine-receipt")
        expect(pure.parseChain(String(text)).issued).toBeDefined()
        expect(type).toBe("application/vnd.vitrine.receipt-chain")
    })

    it.each([
        ["queued", entryOf("queued", "busy")],
        ["unavailable", entryOf("unavailable", "log-down")],
    ] as const)(
        "shows a problem when the entry was %s and there is no receipt",
        async (_kind, answer) => {
            casting(answer)
            const {container} = renderScreen()
            await stateOf(container, "problem")
            fireEvent.click(statusButton(container)!)
            const dialog = await screen.findByRole("dialog", {name: "vitrine.status.title"})
            expect(dialog.querySelector(".ballot-status-head")?.textContent).toBe(
                "vitrine.status.head.noReceipt"
            )
            // The entry is kept until the log answers for it.
            await waitFor(() => expect(mockKeepReceipt).toHaveBeenCalled())
            expect(mockKeepReceipt.mock.calls[0][2]).not.toHaveProperty("chain")
            expect(
                calls.every(
                    (c) => !c.url.includes("/receipt/") || c.url.startsWith(LOG_URLS.ballots)
                )
            ).toBe(true)
            // The voter can still finish.
            expect(container.querySelector(".finish-button")).not.toBeNull()
        }
    )

    it("keeps nothing on a kiosk", async () => {
        mockIsKiosk = true
        casting(receiptOf)
        const {container} = renderScreen()
        const dialog = await openWithReceipt(container)
        expect(
            within(dialog)
                .getByRole("button", {name: "vitrine.status.save"})
                .hasAttribute("disabled")
        ).toBe(false)
        expect(mockKeepReceipt).not.toHaveBeenCalled()
    })

    it("treats Harvest's text as data", async () => {
        const markup = '<img src="x" onerror="window.__vitrinePwned = 1"><script>1</script>'
        casting(() => ({kind: "receipt", text: markup, reason: markup}))
        const {container} = renderScreen()
        await stateOf(container, "problem")
        fireEvent.click(statusButton(container)!)
        await screen.findByRole("dialog", {name: "vitrine.status.title"})
        expect(document.body.innerHTML).not.toContain("onerror")
        expect(document.body.querySelector("img, script")).toBeNull()
        expect((window as unknown as Record<string, unknown>).__vitrinePwned).toBeUndefined()
    })

    it.each([
        ["events without transparency", undefined, false],
        [
            "demo ballots",
            {cast: null, transparency: readTransparency(scenario().presentation)},
            true,
        ],
    ] as const)("shows nothing for %s: the portal's own screen", async (_name, vitrine, isDemo) => {
        mockState = confirm(nextBallotId(), vitrine, isDemo)
        const {container, findAllByTestId} = renderScreen()
        const [desktop] = await findAllByTestId("ballot-id")
        expect(desktop.textContent).toMatch(/^[0-9a-f]{64}$/)
        expect(container.querySelector(".ballot-id-check-icon")).not.toBeNull()
        expect(statusButton(container)).toBeNull()
        expect(container.querySelector(".qr-container svg")).not.toBeNull()
        // Both responsive values stay, with the label hidden on a phone.
        expect(container.querySelector(".ballot-id-value-desktop")).not.toBeNull()
        expect(container.querySelector(".ballot-id-value-mobile")).not.toBeNull()
        expect(hiddenByAnyRule(container.querySelector(".ballot-id-label")!)).toBe(true)
        expect(container.querySelector(".status-tag")).toBeNull()
        expect(container.querySelector(".ballot-id-text")).toBeNull()
        expect(getComputedStyle(container.querySelector(".ballot-id-border")!).borderRadius).toBe(
            "4px"
        )
    })
})
