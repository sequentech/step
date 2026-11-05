// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {SaveButton, SimpleForm, Toolbar} from "react-admin"
import {ETranslationScope, i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {TranslationScopeInput} from "./TranslationScopeInput"

interface Scenario {
    allowedScopes: ETranslationScope[]
    defaultValue: ETranslationScope
    /** The scope the translation already has, if any. */
    saved?: ETranslationScope
    onSubmit: Mock<(values: Record<string, unknown>) => void>
}

let graphql: ReturnType<typeof graphqlBoundary>
// The host form's toolbar; the widget has no actions of its own.
const saveOnly = (
    <Toolbar>
        <SaveButton />
    </Toolbar>
)

const meta = {
    title: "Admin/Components/TranslationScopeInput",
    component: TranslationScopeInput,
    args: {
        allowedScopes: [
            ETranslationScope.GLOBAL,
            ETranslationScope.VOTING_PORTAL,
            ETranslationScope.ADMIN_PORTAL,
        ],
        defaultValue: ETranslationScope.GLOBAL,
        onSubmit: fn(),
    },
    argTypes: {
        allowedScopes: {control: "check", options: Object.values(ETranslationScope)},
        defaultValue: {control: "select", options: Object.values(ETranslationScope)},
        saved: {control: "select", options: Object.values(ETranslationScope)},
    },
    beforeEach: async () => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({allowedScopes, defaultValue, saved, onSubmit}) => (
        <AdminStoryProvider boundary={graphql}>
            <SimpleForm
                record={saved ? {id: 1, scope: saved} : {}}
                onSubmit={onSubmit}
                toolbar={saveOnly}
            >
                <TranslationScopeInput
                    source="scope"
                    allowedScopes={allowedScopes}
                    defaultValue={defaultValue}
                />
            </SimpleForm>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const scope = (value: string) => i18n.t(`electionEventScreen.localization.scopes.${value}`)
const scopeInput = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("combobox", {
        name: new RegExp(i18n.t("electionEventScreen.localization.labels.scope")),
    })

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(scopeInput(canvasElement)).toHaveTextContent(scope("global"))
        await userEvent.click(scopeInput(canvasElement))
        const options = await within(document.body).findAllByRole("option")
        expect(options.map((option) => option.textContent)).toEqual([
            scope("global"),
            scope("votingPortal"),
            scope("adminPortal"),
        ])
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(within(document.body).queryByRole("listbox")).toBeNull())
    },
}

export const SavesTheChosenScope: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(scopeInput(canvasElement))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: scope("votingPortal")})
        )
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
        expect(args.onSubmit.mock.calls[0][0]).toEqual({scope: ETranslationScope.VOTING_PORTAL})
    },
}

export const SavedScope: Story = {
    args: {saved: ETranslationScope.ADMIN_PORTAL},
    play: async ({canvasElement}) => {
        await expect(scopeInput(canvasElement)).toHaveTextContent(scope("adminPortal"))
    },
}
