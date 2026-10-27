/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {MockedProvider} from "@apollo/client/testing"
import {
    SettingsAttackDetection,
    GET_ATTACK_SETTINGS,
    SAVE_ATTACK_SETTINGS,
} from "./SettingsAttackDetection"

let mockAllowed = true
jest.mock("@/providers/AuthContextProvider", () => {
    const {createContext} = jest.requireActual("react")
    return {AuthContext: createContext({isAuthorized: () => mockAllowed})}
})
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-a"]}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (_key: string, text: string) => text}),
}))
const settings = {failed_logins_per_minute: 20, requests_per_minute: 2000}
const read = (tenant = "tenant-a") => ({
    request: {query: GET_ATTACK_SETTINGS},
    result: {data: {get_attack_detection_settings: {tenant_id: tenant, settings}}},
})
beforeEach(() => {
    mockAllowed = true
})

it("does not expose configuration controls to a non-administrator", () => {
    mockAllowed = false
    render(
        <MockedProvider mocks={[]}>
            <SettingsAttackDetection />
        </MockedProvider>
    )
    expect(screen.queryByRole("spinbutton")).not.toBeInTheDocument()
    expect(screen.getByText(/administrator permission/i)).toBeInTheDocument()
})

it("refuses another tenant's response and never replaces missing settings with defaults", async () => {
    render(
        <MockedProvider mocks={[read("tenant-b")]}>
            <SettingsAttackDetection />
        </MockedProvider>
    )
    await screen.findByText(/could not be verified/i)
    expect(screen.queryByRole("spinbutton")).not.toBeInTheDocument()
})

it("does not submit zero, fractional or excessive thresholds", async () => {
    render(
        <MockedProvider mocks={[read()]}>
            <SettingsAttackDetection />
        </MockedProvider>
    )
    const field = await screen.findByRole("spinbutton", {name: /Failed logins/})
    for (const value of ["0", "1.5", "100001", ""]) {
        fireEvent.change(field, {target: {value}})
        expect(screen.getByRole("button", {name: /Save attack/})).toBeDisabled()
    }
})

it("reports a refused update without claiming it was saved", async () => {
    const mocks = [
        read(),
        {
            request: {query: SAVE_ATTACK_SETTINGS, variables: {settings}},
            error: new Error("Execution phase refuses configuration changes"),
        },
    ]
    render(
        <MockedProvider mocks={mocks}>
            <SettingsAttackDetection />
        </MockedProvider>
    )
    fireEvent.click(await screen.findByRole("button", {name: /Save attack/}))
    await screen.findByText(/were not saved/i)
    expect(
        screen.queryByText(/^Attack detection settings saved and verified/i)
    ).not.toBeInTheDocument()
})

it("confirms a save only after the current tenant's network read-back matches", async () => {
    const changed = {...settings, failed_logins_per_minute: 25}
    const mocks = [
        read(),
        {
            request: {query: SAVE_ATTACK_SETTINGS, variables: {settings: changed}},
            result: {
                data: {
                    update_attack_detection_settings: {tenant_id: "tenant-a", settings: changed},
                },
            },
        },
        {
            ...read(),
            result: {
                data: {get_attack_detection_settings: {tenant_id: "tenant-a", settings: changed}},
            },
        },
    ]
    render(
        <MockedProvider mocks={mocks}>
            <SettingsAttackDetection />
        </MockedProvider>
    )
    fireEvent.change(await screen.findByRole("spinbutton", {name: /Failed logins/}), {
        target: {value: "25"},
    })
    fireEvent.click(await screen.findByRole("button", {name: /Save attack/}))
    await waitFor(() => expect(screen.getByText(/saved and verified/i)).toBeInTheDocument())
})

it("does not confirm a save when read-back differs from the approved input", async () => {
    const mocks = [
        read(),
        {
            request: {query: SAVE_ATTACK_SETTINGS, variables: {settings}},
            result: {data: {update_attack_detection_settings: {tenant_id: "tenant-a", settings}}},
        },
        {
            ...read(),
            result: {
                data: {
                    get_attack_detection_settings: {
                        tenant_id: "tenant-a",
                        settings: {...settings, requests_per_minute: 3000},
                    },
                },
            },
        },
    ]
    render(
        <MockedProvider mocks={mocks}>
            <SettingsAttackDetection />
        </MockedProvider>
    )
    fireEvent.click(await screen.findByRole("button", {name: /Save attack/}))
    await screen.findByText(/were not saved/i)
    expect(
        screen.queryByText(/^Attack detection settings saved and verified/i)
    ).not.toBeInTheDocument()
})
