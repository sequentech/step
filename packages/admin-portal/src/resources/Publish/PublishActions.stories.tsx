// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {EVotingStatus} from "@sequentech/ui-core"
import {PublishActions} from "./PublishActions"
import {PublishStatus} from "./EPublishStatus"
import {EPublishActionsType, EPublishType} from "./EPublishType"
import {AuthContext} from "@/providers/AuthContextProvider"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"

const roles = [
    "publish-write",
    "election-state-write",
    "publish-regenerate",
    "publish-start-voting",
    "publish-pause-voting",
    "publish-stop-voting",
    "publish-changes",
]
const disabledChannel = {is_channel_enabled: false, status: EVotingStatus.NOT_STARTED}
const pendingKeys = [
    "pendingStartVotingPeriod",
    "pendingPauseVotingPeriod",
    "pendingStopVotingPeriod",
    "pendingPublishAction",
]
let boundary: ReturnType<typeof graphqlBoundary>
type Props = React.ComponentProps<typeof PublishActions> & {
    roles: string[]
    gold: boolean
    reauthenticate: (url: string) => Promise<void>
    requireInitializationReport: boolean
}

function PublicationFixture(props: Props) {
    const auth = useContext(AuthContext)
    return (
        <AdminStoryProvider boundary={boundary}>
            <AuthContext.Provider
                value={{
                    ...auth,
                    isAuthenticated: true,
                    tenantId: TENANT_ID,
                    isGoldUser: () => props.gold,
                    reauthWithGold: props.reauthenticate,
                    hasRole: (role) => props.roles.includes(role),
                    isAuthorized: (_superAdmin, tenant, permission) =>
                        tenant === TENANT_ID &&
                        (Array.isArray(permission) ? permission : [permission]).every((role) =>
                            props.roles.includes(role)
                        ),
                }}
            >
                <RecordContextProvider
                    value={{
                        id: EVENT_ID,
                        tenant_id: TENANT_ID,
                        presentation: {
                            initialization_report_policy: props.requireInitializationReport
                                ? "required"
                                : "not-required",
                        },
                        initialization_report_generated: false,
                    }}
                >
                    <PublishActions {...props} />
                </RecordContextProvider>
            </AuthContext.Provider>
        </AdminStoryProvider>
    )
}
const meta = {
    title: "Admin/Publication actions",
    component: PublishActions,
    args: {
        roles,
        gold: true,
        reauthenticate: fn(async () => {}),
        requireInitializationReport: false,
        status: PublishStatus.Published,
        publishType: EPublishType.Event,
        type: EPublishActionsType.List,
        electionStatus: null,
        electionPresentation: null,
        changingStatus: false,
        kioskModeEnabled: disabledChannel,
        earlyVotingEnabled: disabledChannel,
        telephoneVotingEnabled: disabledChannel,
        onlineModeEnabled: {is_channel_enabled: true, status: EVotingStatus.NOT_STARTED},
        onGenerate: fn(),
        onPublish: fn(),
        onChangeStatus: fn(),
    },
    beforeEach: () => {
        boundary = graphqlBoundary({})
        for (const key of pendingKeys) {
            sessionStorage.removeItem(key)
            sessionStorage.removeItem(`${key}_CHANNELS`)
        }
        localStorage.removeItem("electionEventPublishTabIndex")
    },
    render: (args) => <PublicationFixture {...args} />,
} satisfies Meta<Props>
export default meta
type Story = StoryObj<typeof meta>

async function openChannelAction(
    canvasElement: HTMLElement,
    action: string,
    channelAction: string
) {
    await userEvent.click(within(canvasElement).getByRole("button", {name: action}))
    await userEvent.click(await within(document.body).findByRole("menuitem", {name: channelAction}))
    const dialogElement = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialogElement).toBeVisible())
    return within(dialogElement)
}
async function confirm(dialog: ReturnType<typeof within>) {
    await userEvent.click(dialog.getByRole("button", {name: "Confirm"}))
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument())
    expect(boundary.calls).toEqual([])
    expect(boundary.unexpected).toEqual([])
}

export const StartOnlineVoting: Story = {
    play: async ({canvasElement, args}) => {
        const dialog = await openChannelAction(canvasElement, "Start Voting", "Start Online Voting")
        expect(args.onChangeStatus).not.toHaveBeenCalled()
        await confirm(dialog)
        expect(args.onChangeStatus).toHaveBeenCalledWith("OPEN", ["ONLINE"])
        expect(args.reauthenticate).not.toHaveBeenCalled()
    },
}
export const PauseOnlineVoting: Story = {
    args: {onlineModeEnabled: {is_channel_enabled: true, status: EVotingStatus.OPEN}},
    play: async ({canvasElement, args}) => {
        const dialog = await openChannelAction(canvasElement, "Pause Voting", "Pause Online Voting")
        await confirm(dialog)
        expect(args.onChangeStatus).toHaveBeenCalledWith("PAUSED", ["ONLINE"])
    },
}
export const StopOnlineVoting: Story = {
    args: {onlineModeEnabled: {is_channel_enabled: true, status: EVotingStatus.OPEN}},
    play: async ({canvasElement, args}) => {
        const dialog = await openChannelAction(canvasElement, "Stop Voting", "Stop Online Voting")
        await confirm(dialog)
        expect(args.onChangeStatus).toHaveBeenCalledWith("CLOSED", ["ONLINE"])
    },
}
export const CancelSensitiveAction: Story = {
    play: async ({canvasElement, args}) => {
        const dialog = await openChannelAction(canvasElement, "Start Voting", "Start Online Voting")
        await userEvent.click(dialog.getByRole("button", {name: "Cancel"}))
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        )
        expect(args.onChangeStatus).not.toHaveBeenCalled()
        expect(args.reauthenticate).not.toHaveBeenCalled()
    },
}
export const ActionsHiddenWithoutPermissions: Story = {
    args: {roles: []},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        for (const name of ["Start Voting", "Pause Voting", "Stop Voting", "Publish Changes"]) {
            expect(canvas.queryByRole("button", {name})).not.toBeInTheDocument()
        }
        expect(args.onChangeStatus).not.toHaveBeenCalled()
        expect(boundary.calls).toEqual([])
    },
}
export const StartPermissionDoesNotGrantOtherActions: Story = {
    args: {roles: ["election-state-write", "publish-start-voting"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("button", {name: "Start Voting"})).toBeEnabled()
        for (const name of ["Pause Voting", "Stop Voting", "Publish Changes"]) {
            expect(canvas.queryByRole("button", {name})).not.toBeInTheDocument()
        }
    },
}
export const ClosedChannelsCannotRestart: Story = {
    args: {onlineModeEnabled: {is_channel_enabled: true, status: EVotingStatus.CLOSED}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        for (const name of ["Start Voting", "Pause Voting", "Stop Voting"]) {
            await expect(canvas.getByRole("button", {name})).toBeDisabled()
        }
    },
}
export const GoldReauthenticationPreservesChannel: Story = {
    args: {gold: false},
    play: async ({canvasElement, args}) => {
        const dialog = await openChannelAction(canvasElement, "Start Voting", "Start Online Voting")
        await confirm(dialog)
        expect(args.onChangeStatus).not.toHaveBeenCalled()
        expect(args.reauthenticate).toHaveBeenCalledTimes(1)
        expect(args.reauthenticate).toHaveBeenCalledWith(expect.stringContaining("tabIndex=8"))
        expect(sessionStorage.getItem("pendingStartVotingPeriod")).toBe("true")
        expect(sessionStorage.getItem("pendingStartVotingPeriod_CHANNELS")).toBe('["ONLINE"]')
    },
}
export const PublishChangesRequiresConfirmation: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Publish Changes"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        expect(args.onGenerate).not.toHaveBeenCalled()
        await confirm(dialog)
        expect(args.onGenerate).toHaveBeenCalledTimes(1)
    },
}
export const RegenerationRequiresPermission: Story = {
    args: {type: EPublishActionsType.Generate, roles: ["publish-write"]},
    play: async ({canvasElement, args}) => {
        expect(
            within(canvasElement).queryByRole("button", {name: "Regenerate"})
        ).not.toBeInTheDocument()
        expect(args.onGenerate).not.toHaveBeenCalled()
    },
}
export const RegeneratePublication: Story = {
    args: {type: EPublishActionsType.Generate},
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Regenerate"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await confirm(dialog)
        expect(args.onGenerate).toHaveBeenCalledTimes(1)
    },
}
export const RequiredInitializationReportBlocksOnlineStart: Story = {
    args: {
        requireInitializationReport: true,
        kioskModeEnabled: {is_channel_enabled: true, status: EVotingStatus.NOT_STARTED},
    },
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Start Voting"}))
        const menu = within(await within(document.body).findByRole("menu"))
        await expect(menu.getByRole("menuitem", {name: "Start Online Voting"})).toHaveAttribute(
            "aria-disabled",
            "true"
        )
        expect(menu.getByRole("menuitem", {name: "Start Kiosk Voting"})).not.toHaveAttribute(
            "aria-disabled",
            "true"
        )
        await userEvent.keyboard("{Escape}")
    },
}
