// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {storyFetch} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {TrusteeDashboard} from "./TrusteeDashboard"
import {B4_URL, BOARD_NAME, trusteeKeys} from "./__stories__/TrusteeDashboardFixture"

type Scenario = Record<string, never>

let graphql: ReturnType<typeof graphqlBoundary>
let board: ReturnType<typeof storyFetch>

const meta = {
    title: "Admin/Screens/TrusteeDashboard",
    component: TrusteeDashboard,
    parameters: {
        expectedFailure: {
            reason: "The protocol progress bar has no name and the console's clear action is an unnamed icon button.",
            a11y: ["aria-progressbar-name", "button-name"],
        },
    },
    beforeEach: () => {
        graphql = graphqlBoundary({})
        board = storyFetch({[B4_URL]: () => ({status: 503, body: "Synthetic board unavailable"})})
    },
    render: () => (
        <AdminStoryProvider boundary={graphql}>
            <TrusteeDashboard />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const WASM_LOADED = "braid-wasm loaded and thread pool initialized"

const button = (canvasElement: HTMLElement, name: string | RegExp) =>
    within(canvasElement).getByRole("button", {name})

/** Waits for the console to show a line containing the text. */
const consoleLine = (canvasElement: HTMLElement, text: string) =>
    within(canvasElement).findByText(new RegExp(text), {}, {timeout: 10_000})

async function initialize(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await consoleLine(canvasElement, WASM_LOADED)
    await userEvent.type(
        canvas.getByRole("textbox", {name: "Signing Key SK (Base64 DER)"}),
        trusteeKeys.signingKeySk
    )
    await userEvent.type(
        canvas.getByRole("textbox", {name: "Signing Key PK (Base64 DER)"}),
        trusteeKeys.signingKeyPk
    )
    await userEvent.type(
        canvas.getByRole("textbox", {name: "Encryption Key (Base64)"}),
        trusteeKeys.encryptionKey
    )
    const url = canvas.getByRole("textbox", {name: "B4 Server URL"})
    await userEvent.clear(url)
    await userEvent.type(url, B4_URL)
    await userEvent.click(button(canvasElement, "Initialize Trustee"))
    await consoleLine(canvasElement, "Trustee initialized: browser-trustee-1")
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(await consoleLine(canvasElement, WASM_LOADED)).toBeVisible()
        await expect(button(canvasElement, "Initialize Trustee")).toBeEnabled()
        for (const name of ["Execute Step", "Auto Run", "Fetch Boards", "Connect", "Refresh"]) {
            await expect(button(canvasElement, name)).toBeDisabled()
        }
        await expect(
            within(canvasElement).getByText("Connect to a board to see storage information")
        ).toBeVisible()
        expect(board.calls).toEqual([])
    },
}

export const InitializationNeedsEveryField: Story = {
    play: async ({canvasElement}) => {
        await consoleLine(canvasElement, WASM_LOADED)
        await userEvent.click(button(canvasElement, "Initialize Trustee"))
        await expect(await consoleLine(canvasElement, "All fields required")).toBeVisible()
        await expect(button(canvasElement, "Fetch Boards")).toBeDisabled()
    },
}

export const InitializeATrustee: Story = {
    play: async ({canvasElement}) => {
        await initialize(canvasElement)
        await expect(button(canvasElement, "Initialized")).toBeDisabled()
        await expect(button(canvasElement, "Fetch Boards")).toBeEnabled()
        await expect(button(canvasElement, "Execute Step")).toBeDisabled()
        expect(board.calls).toEqual([])
    },
}

export const BoardServiceUnavailable: Story = {
    play: async ({canvasElement}) => {
        await initialize(canvasElement)
        await userEvent.click(button(canvasElement, "Fetch Boards"))
        await expect(await consoleLine(canvasElement, "Fetch failed")).toBeVisible()
        await waitFor(() => expect(button(canvasElement, "Fetch Boards")).toBeEnabled())
        expect(board.calls.map(({url}) => new URL(url).origin)).toContain(B4_URL)
    },
}

export const ConnectWithInvalidKeys: Story = {
    play: async ({canvasElement}) => {
        await initialize(canvasElement)
        await userEvent.type(
            within(canvasElement).getByRole("textbox", {name: "Board name"}),
            BOARD_NAME
        )
        await userEvent.click(button(canvasElement, "Connect"))
        // The session rejects with a string, so the console loses the reason.
        await expect(await consoleLine(canvasElement, "Connect failed: undefined")).toBeVisible()
        await expect(button(canvasElement, "Execute Step")).toBeDisabled()
        expect(board.calls).toEqual([])
    },
}

export const ClearTheConsole: Story = {
    parameters: {
        expectedFailure: {
            reason: "The protocol progress bar has no name, the console's clear action is an unnamed icon button, and the empty console's grey text lacks contrast on its dark background.",
            a11y: ["aria-progressbar-name", "button-name", "color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        const line = await consoleLine(canvasElement, WASM_LOADED)
        const header = line.closest(".MuiCard-root")
        if (!(header instanceof HTMLElement)) throw new Error("The console card is missing")
        await userEvent.click(within(header).getAllByRole("button")[0])
        await expect(within(header).getByText("No logs yet")).toBeVisible()
    },
}
