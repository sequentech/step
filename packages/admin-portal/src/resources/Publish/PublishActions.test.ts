/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
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
    useGetList: () => ({data: undefined}),
}))
jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
    useQuery: () => ({data: undefined}),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
        i18n: {language: "en"},
    }),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => true, isGoldUser: () => true}),
}))
jest.mock("./usePublishPermissions", () => ({
    usePublishPermissions: () => ({canPublishChanges: true, canPublishStopVoting: true}),
}))
let mockEventPresentation: Record<string, unknown> | undefined
jest.mock("@/hooks/useZonedFormat", () => ({
    useEventPresentation: () => mockEventPresentation,
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        Dialog: ({
            open,
            handleClose,
            children,
        }: {
            open: boolean
            handleClose: (confirmed: boolean) => void
            children?: React.ReactNode
        }) =>
            open
                ? require("react").createElement(
                      "div",
                      {role: "dialog"},
                      children,
                      require("react").createElement(
                          "button",
                          {onClick: () => handleClose(true)},
                          "Confirm publication"
                      )
                  )
                : null,
    }),
    {virtual: true}
)
jest.mock("@sequentech/ui-core", () => ({
    ...require("../../../../ui-core/src/types/CoreTypes"),
    ...require("../../../../ui-core/src/types/ElectionPresentation"),
    ...require("../../../../ui-core/src/types/ElectionEventPresentation"),
}))
jest.mock("./PublishExport", () => ({__esModule: true, default: () => null}))
jest.mock("@/gql/graphql", () => ({
    VotingStatusChannel: {
        Online: "ONLINE",
        Kiosk: "KIOSK",
        EarlyVoting: "EARLY_VOTING",
        Telephone: "TELEPHONE",
    },
}))

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

describe("the Post's Stop menu (R10 B1)", () => {
    const info = (is_channel_enabled: boolean, status: string) =>
        ({is_channel_enabled, status}) as PublishActionsProps["onlineModeEnabled"]
    const renderPost = (
        kiosk: PublishActionsProps["kioskModeEnabled"],
        props: Partial<PublishActionsProps> = {}
    ) =>
        render(
            React.createElement(PublishActions, {
                status: PublishStatus.Started,
                publishType: EPublishType.Election,
                type: EPublishActionsType.List,
                electionStatus: null,
                electionPresentation: null,
                changingStatus: false,
                onlineModeEnabled: info(true, "OPEN"),
                kioskModeEnabled: kiosk,
                earlyVotingEnabled: info(false, "NOT_STARTED"),
                telephoneVotingEnabled: info(false, "NOT_STARTED"),
                onGenerate: jest.fn(),
                ...props,
            })
        )
    const kioskItem = () => {
        fireEvent.click(screen.getByRole("button", {name: "publish.action.stopVotingPeriod"}))
        return within(screen.getByRole("menu"))
            .getByText("publish.action.stopKioskVotingPeriod")
            .closest("li") as HTMLElement
    }
    afterEach(() => {
        mockEventPresentation = undefined
    })

    it.each(["OPEN", "PAUSED", "NOT_STARTED"])(
        "with Seal at close offers stopping a channel the Post doesn't enable that is %s",
        (status) => {
            mockEventPresentation = {ballot_box_seal_policy: "seal-at-close"}
            renderPost(info(false, status))
            expect(kioskItem().getAttribute("aria-disabled")).not.toBe("true")
        }
    )
    it("with Seal at close doesn't offer stopping a CLOSED channel", () => {
        mockEventPresentation = {ballot_box_seal_policy: "seal-at-close"}
        renderPost(info(false, "CLOSED"))
        expect(kioskItem().getAttribute("aria-disabled")).toBe("true")
    })
    it("with the policy off, as before: only an enabled channel is offered", () => {
        renderPost(info(false, "OPEN"))
        expect(kioskItem().getAttribute("aria-disabled")).toBe("true")
    })
    it("explains a Stop after which no channel counts for the seal (N2)", () => {
        mockEventPresentation = {ballot_box_seal_policy: "seal-at-close"}
        renderPost(info(false, "NOT_STARTED"), {onlineModeEnabled: info(false, "NOT_STARTED")})
        fireEvent.click(kioskItem())
        const text = screen.getByRole("dialog").textContent ?? ""
        expect(text).toContain("publish.dialog.sealNoChannel")
        expect(text).not.toContain("publish.dialog.sealHolding")
    })
    it("tells how to release a channel the Post doesn't enable: stop it", () => {
        mockEventPresentation = {ballot_box_seal_policy: "seal-at-close"}
        renderPost(info(false, "OPEN"), {onlineModeEnabled: info(true, "OPEN")})
        fireEvent.click(screen.getByRole("button", {name: "publish.action.stopVotingPeriod"}))
        fireEvent.click(
            within(screen.getByRole("menu")).getByText("publish.action.stopOnlineVoting")
        )
        const text = screen.getByRole("dialog").textContent ?? ""
        expect(text).toContain("publish.dialog.sealNotEnabled")
        expect(text).not.toContain("publish.dialog.sealHolding")
    })
})
