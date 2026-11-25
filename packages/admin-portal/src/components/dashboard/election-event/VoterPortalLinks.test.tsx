/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render as renderWith, screen} from "@testing-library/react"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {VoterPortalLinks} from "./VoterPortalLinks"

const PORTAL = "https://vote.example.org"
const KIOSK_PORTAL = "https://kiosk.example.org"
const TENANT = "11111111-1111-4111-8111-111111111111"
const EVENT = "22222222-2222-4222-8222-222222222222"

let mockTenant: string | null = TENANT
let mockSettings: {VOTING_PORTAL_URL: string; KIOSK_VOTING_PORTAL_URL?: string} = {
    VOTING_PORTAL_URL: PORTAL,
}

jest.mock("@/providers/TenantContextProvider", () => ({
    useTenantStore: () => [mockTenant, () => undefined],
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {}}),
}))
// sequent-core's get_auth_url: `{base}/tenant/{tenant}/event/{event}/{action}`.
jest.mock("@/services/UrlGeneration", () => ({
    getAuthUrl: (base: string, tenant: string, event: string, action: string) =>
        `${base}/tenant/${tenant}/event/${event}/${action}`,
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: {link?: string}) =>
            options?.link ? `${key}:${options.link}` : key,
    }),
}))
// The dialog shows its title and content while open; the QR exposes what it encodes.
jest.mock(
    "@sequentech/ui-essentials",
    () => {
        const React = require("react")
        return {
            Dialog: ({
                open,
                title,
                cancel,
                handleClose,
                children,
            }: {
                open: boolean
                title: string
                cancel: string
                handleClose: (confirmed: boolean) => void
                children: React.ReactNode
            }) =>
                open
                    ? React.createElement(
                          "div",
                          {"role": "dialog", "aria-label": title},
                          children,
                          React.createElement("button", {onClick: () => handleClose(false)}, cancel)
                      )
                    : null,
            QRCode: ({value}: {value: string}) =>
                React.createElement("svg", {"data-testid": "qr-code", "data-value": value}),
        }
    },
    {virtual: true}
)

// The deployment's portal addresses, as the settings provider gives them.
const Settings: React.FC<{children: React.ReactNode}> = ({children}) => (
    <SettingsContext.Provider
        value={
            {globalSettings: mockSettings} as unknown as React.ContextType<typeof SettingsContext>
        }
    >
        {children}
    </SettingsContext.Provider>
)
const render = (ui: React.ReactElement) => renderWith(ui, {wrapper: Settings})

const href = (name: string) => screen.getByRole("link", {name}).getAttribute("href")

const event = (kiosk?: boolean) => ({id: EVENT, voting_channels: {online: true, kiosk}})

beforeEach(() => {
    mockTenant = TENANT
    mockSettings = {VOTING_PORTAL_URL: PORTAL}
})

describe("VoterPortalLinks", () => {
    it("links the current event's voter login and enrollment", () => {
        render(<VoterPortalLinks electionEvent={event()} />)
        expect(href("dashboard.voterLoginURL")).toBe(
            `${PORTAL}/tenant/${TENANT}/event/${EVENT}/login`
        )
        expect(href("dashboard.voterEnrollURL")).toBe(
            `${PORTAL}/tenant/${TENANT}/event/${EVENT}/enroll`
        )
        expect(screen.queryByRole("link", {name: "dashboard.voterEnrollKioskURL"})).toBeNull()
    })

    it("opens the links in a new tab without giving the portal the opener", () => {
        render(<VoterPortalLinks electionEvent={event()} />)
        for (const link of screen.getAllByRole("link")) {
            expect(link.getAttribute("target")).toBe("_blank")
            expect(link.getAttribute("rel")).toBe("noopener noreferrer")
        }
    })

    it.each([false, undefined])("offers no kiosk link when the kiosk channel is %s", (kiosk) => {
        render(<VoterPortalLinks electionEvent={event(kiosk)} />)
        expect(screen.getAllByRole("link")).toHaveLength(2)
    })

    it("links the kiosk portal's login when the event votes by kiosk", () => {
        mockSettings = {VOTING_PORTAL_URL: PORTAL, KIOSK_VOTING_PORTAL_URL: ` ${KIOSK_PORTAL} `}
        render(<VoterPortalLinks electionEvent={event(true)} />)
        expect(href("dashboard.voterEnrollKioskURL")).toBe(
            `${KIOSK_PORTAL}/tenant/${TENANT}/event/${EVENT}/login?kiosk`
        )
    })

    it.each(["", "   "])(
        "falls back to the voting portal for the kiosk when its address is %j",
        (kioskUrl) => {
            mockSettings = {VOTING_PORTAL_URL: PORTAL, KIOSK_VOTING_PORTAL_URL: kioskUrl}
            render(<VoterPortalLinks electionEvent={event(true)} />)
            expect(href("dashboard.voterEnrollKioskURL")).toBe(
                `${PORTAL}/tenant/${TENANT}/event/${EVENT}/login?kiosk`
            )
        }
    )

    it.each([
        ["dashboard.voterLoginURL", "/login"],
        ["dashboard.voterEnrollURL", "/enroll"],
    ])("shows a QR code of the %s", (label, action) => {
        render(<VoterPortalLinks electionEvent={event()} />)
        expect(screen.queryByRole("dialog")).toBeNull()
        fireEvent.click(screen.getByRole("button", {name: `dashboard.showQrCode:${label}`}))
        const dialog = screen.getByRole("dialog", {name: label})
        const url = `${PORTAL}/tenant/${TENANT}/event/${EVENT}${action}`
        expect(screen.getByTestId("qr-code").getAttribute("data-value")).toBe(url)
        expect(dialog.textContent).toContain(url)
    })

    it("shows a QR code of the kiosk login for the kiosk to scan", () => {
        mockSettings = {VOTING_PORTAL_URL: PORTAL, KIOSK_VOTING_PORTAL_URL: KIOSK_PORTAL}
        render(<VoterPortalLinks electionEvent={event(true)} />)
        fireEvent.click(
            screen.getByRole("button", {
                name: "dashboard.showQrCode:dashboard.voterEnrollKioskURL",
            })
        )
        expect(screen.getByTestId("qr-code").getAttribute("data-value")).toBe(
            `${KIOSK_PORTAL}/tenant/${TENANT}/event/${EVENT}/login?kiosk`
        )
    })

    it("closes the QR code", () => {
        render(<VoterPortalLinks electionEvent={event()} />)
        fireEvent.click(
            screen.getByRole("button", {name: "dashboard.showQrCode:dashboard.voterLoginURL"})
        )
        fireEvent.click(screen.getByRole("button", {name: "dashboard.closeQrCode"}))
        expect(screen.queryByRole("dialog")).toBeNull()
    })

    it("shows no links until the tenant and event are known", () => {
        mockTenant = null
        const {container, rerender} = render(<VoterPortalLinks electionEvent={event()} />)
        expect(container.querySelector("a")).toBeNull()
        mockTenant = TENANT
        rerender(<VoterPortalLinks electionEvent={{id: undefined}} />)
        expect(container.querySelector("a")).toBeNull()
    })
})
