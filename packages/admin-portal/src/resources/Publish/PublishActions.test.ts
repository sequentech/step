/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import {PublishActions, PublishActionsProps} from "./PublishActions"
import {PublishStatus} from "./EPublishStatus"
import {EPublishActionsType, EPublishType} from "./EPublishType"

jest.mock("react-admin", () => ({
    Button: ({
        label,
        children,
        ...props
    }: React.ButtonHTMLAttributes<HTMLButtonElement> & {label: string}) =>
        require("react").createElement("button", props, label, children),
    useRecordContext: () => ({}),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => true, isGoldUser: () => true}),
}))
jest.mock("./usePublishPermissions", () => ({
    usePublishPermissions: () => ({canPublishChanges: true}),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        Dialog: ({
            open,
            handleClose,
        }: {
            open: boolean
            handleClose: (confirmed: boolean) => void
        }) =>
            open
                ? require("react").createElement(
                      "button",
                      {onClick: () => handleClose(true)},
                      "Confirm publication"
                  )
                : null,
    }),
    {virtual: true}
)
jest.mock("@sequentech/ui-core", () => ({
    ...require("../../../../ui-core/src/types/CoreTypes"),
    ...require("../../../../ui-core/src/types/ElectionPresentation"),
}))
jest.mock("./PublishExport", () => ({__esModule: true, default: () => null}))

it.each([PublishStatus.Stopped, PublishStatus.Paused, PublishStatus.Started])(
    "allows publishing changes in voting state %s",
    (status) => {
        const onGenerate = jest.fn()
        const channel = {is_channel_enabled: false} as PublishActionsProps["onlineModeEnabled"]
        render(
            React.createElement(PublishActions, {
                status,
                publishType: EPublishType.Election,
                type: EPublishActionsType.List,
                electionStatus: null,
                electionPresentation: null,
                changingStatus: false,
                onlineModeEnabled: channel,
                kioskModeEnabled: channel,
                earlyVotingEnabled: channel,
                telephoneVotingEnabled: channel,
                onGenerate,
            })
        )
        const button = screen.getByRole("button", {
            name: "publish.action.publish",
        }) as HTMLButtonElement
        expect(button.disabled).toBe(false)
        fireEvent.click(button)
        fireEvent.click(screen.getByRole("button", {name: "Confirm publication"}))
        expect(onGenerate).toHaveBeenCalledTimes(1)
    }
)
