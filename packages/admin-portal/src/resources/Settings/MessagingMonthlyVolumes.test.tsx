/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import {AuthContext} from "@/providers/AuthContextProvider"
import {
    MessagingMonthlyVolumes,
    billingCategoryLabel,
    countryLabel,
    currentUtcMonth,
    volumeAccountLabel,
} from "./MessagingMonthlyVolumes"

const t = ((key: string) => key) as any

let mockAnswer: {data?: unknown; loading?: boolean; error?: Error} = {}
const mockUseQuery = jest.fn()
jest.mock("@apollo/client", () => ({
    gql: jest.fn(() => ({})),
    useQuery: (...args: unknown[]) => {
        mockUseQuery(...args)
        const options = args[1] as {skip?: boolean}
        return options.skip ? {data: undefined, loading: false} : mockAnswer
    },
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, unknown>) =>
            values ? `${key} ${JSON.stringify(values)}` : key,
        i18n: {language: "en"},
    }),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({ChannelLabel: ({label}: {label: string}) => <span>{label}</span>}),
    {virtual: true}
)
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-2"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => false}),
}))

const renderVolumes = (roles: string[]) =>
    render(
        <AuthContext.Provider
            value={
                {
                    isAuthorized: (_: boolean, __: string, role: string) => roles.includes(role),
                } as any
            }
        >
            <MessagingMonthlyVolumes initialMonth="2026-10" />
        </AuthContext.Provider>
    )

beforeEach(() => {
    mockAnswer = {}
    mockUseQuery.mockReset()
})

describe("monthly volumes", () => {
    it("lists each month's attempts by channel, account, country and billing, with the total", () => {
        mockAnswer = {
            data: {
                messaging_monthly_volumes: {
                    month: "2026-10",
                    rows: [
                        {
                            channel: "WHATSAPP",
                            account_id: "a1",
                            account_name: "Staff Association WhatsApp",
                            destination_country: "ES",
                            billing_category: "UTILITY",
                            billable: true,
                            messages: 1240,
                        },
                        {
                            channel: "WHATSAPP",
                            account_id: null,
                            account_name: null,
                            destination_country: "34",
                            billing_category: null,
                            billable: false,
                            messages: 2,
                        },
                        {
                            channel: "EMAIL",
                            account_id: null,
                            destination_country: null,
                            billable: null,
                            messages: 5,
                        },
                    ],
                },
            },
        }
        renderVolumes(["messaging-account-read"])
        expect(mockUseQuery.mock.calls[0][1]).toMatchObject({
            variables: {month: "2026-10"},
            skip: false,
            context: {headers: {"x-hasura-role": "messaging-account-read"}},
        })
        const [, first, second, third] = screen.getAllByRole("row")
        expect(within(first).getByText("Staff Association WhatsApp")).toBeInTheDocument()
        expect(within(first).getByText("Spain (ES)")).toBeInTheDocument()
        expect(within(first).getByText("messaging.volumes.category.UTILITY")).toBeInTheDocument()
        expect(within(first).getByText("1,240")).toBeInTheDocument()
        expect(within(first).getByText("messaging.volumes.billableYes")).toBeInTheDocument()
        // An account deleted since (no deployment sender on WhatsApp); an
        // older record's calling code.
        expect(within(second).getByText("messaging.audience.deletedAccount")).toBeInTheDocument()
        expect(within(second).getByText("34")).toBeInTheDocument()
        expect(within(second).getByText("messaging.volumes.billableNo")).toBeInTheDocument()
        // Email without an account: the deployment's sender or a deleted
        // account, never said to be the deployment's alone; nothing reported.
        expect(within(third).getByText("messaging.volumes.deploymentOrDeleted")).toBeInTheDocument()
        expect(within(third).queryByText("messaging.send.environment")).toBeNull()
        expect(within(third).getAllByText("messaging.volumes.notReported")).toHaveLength(2)
        expect(within(screen.getAllByRole("row")[4]).getByText("1,247")).toBeInTheDocument()
    })

    it("reads with the config role when there's no account role, and asks again for another month", () => {
        mockAnswer = {data: {messaging_monthly_volumes: {month: "2026-10", rows: []}}}
        renderVolumes(["messaging-config-read"])
        expect(screen.getByText('messaging.volumes.empty {"month":"2026-10"}')).toBeInTheDocument()
        fireEvent.change(screen.getByLabelText("messaging.volumes.month"), {
            target: {value: "2026-09"},
        })
        expect(mockUseQuery.mock.calls.at(-1)?.[1]).toMatchObject({
            variables: {month: "2026-09"},
            context: {headers: {"x-hasura-role": "messaging-config-read"}},
        })
    })

    it("says why without a read permission, and when the volumes can't be loaded", () => {
        renderVolumes([])
        expect(screen.getByText("messaging.volumes.noPermission")).toBeInTheDocument()
        expect(mockUseQuery.mock.calls[0][1]).toMatchObject({skip: true})
        mockAnswer = {error: new Error("refused")}
        renderVolumes(["messaging-account-read"])
        expect(screen.getByText("messaging.volumes.loadError")).toBeInTheDocument()
    })

    it("names each billing category the backend records, in the viewer's language", () => {
        for (const category of ["AUTHENTICATION", "UTILITY", "SERVICE", "TRANSACTIONAL", "EMAIL"]) {
            expect(billingCategoryLabel(t, category)).toBe(`messaging.volumes.category.${category}`)
        }
        expect(billingCategoryLabel(t, null)).toBe("messaging.volumes.notReported")
        expect(billingCategoryLabel(t, "PREMIUM")).toBe("PREMIUM")
    })

    it("never names the deployment's sender for an attempt a deleted account may have sent", () => {
        const of = (channel: string, account_id: string | null, account_name?: string | null) =>
            volumeAccountLabel(t, {channel, account_id, account_name} as any)
        expect(of("WHATSAPP", "a1", "Staff Association WhatsApp")).toBe(
            "Staff Association WhatsApp"
        )
        for (const channel of ["WHATSAPP", "VIBER", "MESSENGER"]) {
            expect(of(channel, null)).toBe("messaging.audience.deletedAccount")
        }
        for (const channel of ["EMAIL", "SMS"]) {
            expect(of(channel, null)).toBe("messaging.volumes.deploymentOrDeleted")
        }
    })

    it("counts months in UTC and names countries in the viewer's language", () => {
        expect(currentUtcMonth(new Date("2026-10-31T23:30:00-05:00"))).toBe("2026-11")
        expect(countryLabel("PH", "en")).toBe("Philippines (PH)")
        expect(countryLabel("966", "en")).toBe("966")
        expect(countryLabel(null, "en")).toBeNull()
    })
})
