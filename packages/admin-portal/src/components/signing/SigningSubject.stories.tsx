// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {ISigningApi, ISigningPanelData} from "@/lib/signing/api"
import {openedWindows} from "@/__stories__/storyNetwork"
import {SigningAction} from "@/lib/signing/types"
import {SigningSubject, SigningSubjectVariant} from "./SigningSubject"
import {shortHash} from "./format"
import {CODE, COUNTRY, POST, fakeApi, makePanel, type IPanelOptions} from "./__stories__/fixtures"

interface Scenario {
    panel: IPanelOptions
    variant: SigningSubjectVariant
    /** The bytes "Open the document" downloads. */
    download: "document" | "other"
}

let boundary: ReturnType<typeof graphqlBoundary>
let data: ISigningPanelData
let api: ReturnType<typeof fakeApi>

const meta = {
    title: "Admin/Signing/SigningSubject",
    component: SigningSubject,
    args: {panel: {}, variant: SigningSubjectVariant.Panel, download: "document"},
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary({})
        data = await makePanel(args.panel)
        api = fakeApi(data)
        if (args.download === "other") {
            api.fetchDocument.mockResolvedValue(new TextEncoder().encode("%PDF-1.7 other returns"))
        }
    },
    render: ({variant}) => (
        <AdminStoryProvider boundary={boundary}>
            <SigningSubject data={data} api={api as ISigningApi} variant={variant} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const Document: Story = {
    parameters: {widgets: ["DocumentCard"]},
    play: async ({canvasElement}) => {
        const view = within(canvasElement)
        const hash = data.request.document_sha256!
        await expect(view.getByText(`Election returns, ${POST}, ${COUNTRY}.pdf`)).toBeVisible()
        await expect(view.getByText(`PDF · 3 pages · SHA-256 ${shortHash(hash)}`)).toHaveAttribute(
            "title",
            hash
        )
        await expect(view.getByTestId("signing-code")).toHaveTextContent(CODE)
        await expect(view.queryByText("Everyone who signs sees the same code.")).toBeNull()

        await userEvent.click(view.getByRole("button", {name: "Open the document"}))
        await waitFor(() => expect(openedWindows()).toHaveLength(1))
        await expect(openedWindows()[0]).toMatch(/^blob:/)
    },
}

/** The server hands out another document than the payload names: it isn't shown. */
export const DocumentReplaced: Story = {
    args: {download: "other"},
    parameters: {widgets: ["DocumentCard"]},
    play: async ({canvasElement}) => {
        const view = within(canvasElement)
        await userEvent.click(view.getByRole("button", {name: "Open the document"}))
        await expect(
            await view.findByText("The document does not match the one this request signs.")
        ).toBeVisible()
        await expect(openedWindows()).toEqual([])
    },
}

export const SubjectFields: Story = {
    args: {
        panel: {action: SigningAction.CloseVoting, subject: {channel: "ONLINE", seal: 3}},
        variant: SigningSubjectVariant.Dialog,
    },
    play: async ({canvasElement}) => {
        const view = within(canvasElement)
        const table = view.getByRole("table", {name: "Details"})
        await expect(within(table).getByText("ONLINE")).toBeVisible()
        await expect(within(table).getByText("3")).toBeVisible()
        await expect(view.queryByTestId("signing-document")).toBeNull()
        await expect(view.getByText("Everyone who signs sees the same code.")).toBeVisible()
    },
}
