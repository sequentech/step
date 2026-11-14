/**
 * @jest-environment jsdom
 */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {createElement, ReactNode} from "react"
import {fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import i18next from "i18next"
import {I18nextProvider, initReactI18next} from "react-i18next"
import englishTranslation from "@/translations/en"
import {SettingsContext} from "@/providers/SettingsContextProvider"

const mockQuery = jest.fn()
const mockRestore = jest.fn()
const mockNotify = jest.fn()
const mockClient = {query: mockQuery}

jest.mock("@apollo/client", () => ({
    ...jest.requireActual("@apollo/client"),
    useApolloClient: () => mockClient,
    useMutation: () => [mockRestore, {loading: false}],
}))

jest.mock("react-admin", () => ({useNotify: () => mockNotify}))

jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../../../../ui-core/src/services/translate")
)

jest.mock(
    "@sequentech/ui-essentials",
    () => {
        const {createElement} = jest.requireActual("react")
        return {
            Dialog: ({open, ok, cancel, children, handleClose}: Record<string, any>) =>
                open
                    ? createElement(
                          "div",
                          {role: "dialog"},
                          children,
                          createElement("button", {onClick: () => handleClose(true)}, ok),
                          createElement("button", {onClick: () => handleClose(false)}, cancel)
                      )
                    : null,
        }
    },
    {virtual: true}
)

import {ElectionEventTrash, TRASH_PAGE_SIZE} from "./ElectionEventTrash"

const POLL_INTERVAL = 60_000

const event = (id: string, name: string, overrides: Record<string, unknown> = {}) => ({
    id,
    presentation: {i18n: {en: {name}}},
    trashed_at: "2026-11-03T09:30:00Z",
    purge_after: "2026-11-10T09:30:00Z",
    purge_started_at: null,
    archived_before_trash: false,
    ...overrides,
})

const page = (items: unknown[], total = items.length, retentionDays: number | null = 7) => ({
    data: {list_trashed_election_events: {items, total, retention_days: retentionDays}},
})

const i18n = i18next.createInstance()

beforeAll(async () => {
    await i18n.use(initReactI18next).init({
        resources: {en: englishTranslation},
        lng: "en",
        ns: ["translations"],
        defaultNS: "translations",
        interpolation: {escapeValue: false},
    })
})

beforeEach(() => {
    jest.clearAllMocks()
})

const renderTrash = (onRestored = jest.fn(), pollInterval = POLL_INTERVAL) => {
    const wrapper = ({children}: {children: ReactNode}) =>
        createElement(
            I18nextProvider,
            {i18n},
            createElement(
                SettingsContext.Provider,
                {
                    value: {
                        loaded: true,
                        globalSettings: {QUERY_POLL_INTERVAL_MS: pollInterval},
                    } as any,
                },
                children
            )
        )
    render(createElement(ElectionEventTrash, {onRestored}), {wrapper})
    return onRestored
}

const restoreButton = (name: string) => screen.getByRole("button", {name: `Restore ${name}`})

describe("ElectionEventTrash", () => {
    it("explains the retention when Trash is empty", async () => {
        mockQuery.mockResolvedValue(page([]))
        renderTrash()
        expect(await screen.findByText("Trash is empty")).toBeTruthy()
        expect(
            screen.getByText("Deleted election events stay here for 7 days and can be restored.")
        ).toBeTruthy()
    })

    it("shows an error with retry when Trash can't be loaded", async () => {
        mockQuery.mockRejectedValueOnce(new Error("offline")).mockResolvedValue(page([]))
        renderTrash()
        expect(await screen.findByText("Trash could not be loaded.")).toBeTruthy()
        fireEvent.click(screen.getByRole("button", {name: "Retry"}))
        expect(await screen.findByText("Trash is empty")).toBeTruthy()
    })

    it("lists each event with its deletion and permanent deletion dates", async () => {
        mockQuery.mockResolvedValue(
            page([
                event("1", "Board 2026", {archived_before_trash: true}),
                event("2", "Referendum", {purge_started_at: "2026-11-10T09:30:05Z"}),
            ])
        )
        renderTrash()
        const items = await screen.findAllByRole("listitem")
        expect(items).toHaveLength(2)
        expect(within(items[0]).getByText("Board 2026")).toBeTruthy()
        expect(within(items[0]).getByText("Was archived")).toBeTruthy()
        expect(within(items[0]).getByText(/^Deleted Nov 3, 2026/)).toBeTruthy()
        expect(within(items[0]).getByText(/^Permanently deleted on Nov 10, 2026/)).toBeTruthy()
        expect((restoreButton("Board 2026") as HTMLButtonElement).disabled).toBe(false)
        expect(within(items[1]).getAllByText("Permanent deletion in progress")).toBeTruthy()
        expect((restoreButton("Referendum") as HTMLButtonElement).disabled).toBe(true)
    })

    it("restores after confirmation and refreshes Trash", async () => {
        mockQuery
            .mockResolvedValueOnce(page([event("1", "Board 2026", {archived_before_trash: true})]))
            .mockResolvedValue(page([]))
        mockRestore.mockResolvedValue({
            data: {restore_election_event: {id: "1", is_archived: true}},
        })
        const onRestored = renderTrash()
        fireEvent.click(await screen.findByRole("button", {name: "Restore Board 2026"}))
        const dialog = screen.getByRole("dialog")
        expect(within(dialog).getByText('Restore "Board 2026"?')).toBeTruthy()
        expect(within(dialog).getByText(/It returns to Archived/)).toBeTruthy()
        expect(within(dialog).getByText(/Its voting status does not change/)).toBeTruthy()
        fireEvent.click(within(dialog).getByRole("button", {name: "Restore"}))
        expect(await screen.findByText("Trash is empty")).toBeTruthy()
        expect(mockRestore).toHaveBeenCalledWith({variables: {electionEventId: "1"}})
        expect(onRestored).toHaveBeenCalledTimes(1)
        expect(mockNotify).toHaveBeenCalledWith('"Board 2026" restored', {type: "success"})
    })

    it("does not report a restore as failed when only the refresh after it fails", async () => {
        mockQuery
            .mockResolvedValueOnce(page([event("1", "Board 2026")]))
            .mockRejectedValue(new Error("offline"))
        mockRestore.mockResolvedValue({
            data: {restore_election_event: {id: "1", is_archived: false}},
        })
        const onRestored = renderTrash()
        fireEvent.click(await screen.findByRole("button", {name: "Restore Board 2026"}))
        fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", {name: "Restore"}))
        expect(
            await screen.findByText("Trash could not be refreshed; it may be out of date.")
        ).toBeTruthy()
        expect(onRestored).toHaveBeenCalledTimes(1)
        expect(mockNotify).toHaveBeenCalledTimes(1)
        expect(mockNotify).toHaveBeenCalledWith('"Board 2026" restored', {type: "success"})
    })

    it("reports a refused restore and keeps the event listed", async () => {
        mockQuery.mockResolvedValue(page([event("1", "Board 2026")]))
        mockRestore.mockRejectedValue(new Error("permanent deletion has started"))
        const onRestored = renderTrash()
        fireEvent.click(await screen.findByRole("button", {name: "Restore Board 2026"}))
        fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", {name: "Restore"}))
        await waitFor(() =>
            expect(mockNotify).toHaveBeenCalledWith(
                "The election event could not be restored: permanent deletion has started",
                {type: "error"}
            )
        )
        expect(onRestored).not.toHaveBeenCalled()
        expect(screen.getByText("Board 2026")).toBeTruthy()
    })

    it("cancelling the confirmation restores nothing", async () => {
        mockQuery.mockResolvedValue(page([event("1", "Board 2026")]))
        renderTrash()
        fireEvent.click(await screen.findByRole("button", {name: "Restore Board 2026"}))
        fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", {name: "Cancel"}))
        expect(screen.queryByRole("dialog")).toBeNull()
        expect(mockRestore).not.toHaveBeenCalled()
    })

    it("loads further pages so every event in Trash can be reached", async () => {
        const first = Array.from({length: TRASH_PAGE_SIZE}, (_, i) => event(`a${i}`, `Event ${i}`))
        mockQuery.mockImplementation(({variables}) =>
            Promise.resolve(
                variables.offset === 0 ? page(first, 51) : page([event("last", "Oldest")], 51)
            )
        )
        renderTrash()
        expect(await screen.findByText("Showing 50 of 51")).toBeTruthy()
        fireEvent.click(screen.getByRole("button", {name: "Load more"}))
        expect(await screen.findByText("Oldest")).toBeTruthy()
        expect(screen.getByText("Showing 51 of 51")).toBeTruthy()
        expect(screen.queryByRole("button", {name: "Load more"})).toBeNull()
        expect(mockQuery).toHaveBeenCalledWith(
            expect.objectContaining({variables: {limit: TRASH_PAGE_SIZE, offset: TRASH_PAGE_SIZE}})
        )
    })

    it("keeps the loaded pages when Trash shrinks below them", async () => {
        const events = Array.from({length: 101}, (_, i) => event(`e${i}`, `Event ${i}`))
        let total = 101
        mockQuery.mockImplementation(({variables}) => {
            const rows = events.slice(0, total).slice(variables.offset, variables.offset + 50)
            // A page past the end doesn't know the total.
            return Promise.resolve(page(rows, rows.length ? total : 0))
        })
        renderTrash()
        fireEvent.click(await screen.findByRole("button", {name: "Load more"}))
        expect(await screen.findByText("Showing 100 of 101")).toBeTruthy()
        fireEvent.click(screen.getByRole("button", {name: "Load more"}))
        expect(await screen.findByText("Event 100")).toBeTruthy()
        total = 100
        fireEvent.click(screen.getByRole("button", {name: "Refresh"}))
        await waitFor(() => expect(screen.queryByText("Event 100")).toBeNull())
        expect(await screen.findByText("Showing 100 of 100")).toBeTruthy()
        expect(screen.getByText("Event 99")).toBeTruthy()
        expect(screen.queryByRole("button", {name: "Load more"})).toBeNull()
    })

    it("keeps polling, so a claim for permanent deletion disables restore without leaving Trash", async () => {
        mockQuery
            .mockResolvedValueOnce(page([event("1", "Board 2026")]))
            .mockResolvedValue(
                page([event("1", "Board 2026", {purge_started_at: "2026-11-10T09:30:05Z"})])
            )
        renderTrash(jest.fn(), 100)
        await waitFor(() =>
            expect((restoreButton("Board 2026") as HTMLButtonElement).disabled).toBe(false)
        )
        await waitFor(() =>
            expect((restoreButton("Board 2026") as HTMLButtonElement).disabled).toBe(true)
        )
        expect(screen.getAllByText("Permanent deletion in progress").length).toBeGreaterThan(0)
    })
})
