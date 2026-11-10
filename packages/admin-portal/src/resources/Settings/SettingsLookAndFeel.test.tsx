/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {SettingsLookAndFeel} from "./SettingsLookAndFeel"

const mockSave = jest.fn()
let mockRecord: Record<string, unknown> | undefined
jest.mock("react-admin", () => {
    const {createElement} = require("react")
    return {
        SimpleForm: ({children, toolbar}: {children: React.ReactNode; toolbar: React.ReactNode}) =>
            createElement("form", null, children, toolbar),
        TextInput: ({
            label,
            helperText,
            source,
            defaultValue,
            onBlur,
        }: {
            label: string
            helperText?: string
            source: string
            defaultValue?: string
            onBlur: (event: unknown) => void
        }) =>
            createElement(
                "label",
                null,
                createElement("span", null, label),
                createElement("textarea", {name: source, defaultValue, onBlur}),
                helperText ? createElement("small", null, helperText) : null
            ),
        Toolbar: ({children}: {children: React.ReactNode}) => createElement("div", null, children),
        SaveButton: ({onClick, disabled}: {onClick: () => void; disabled: boolean}) =>
            createElement("button", {type: "button", onClick, disabled}, "Save"),
        useEditController: () => ({record: mockRecord, save: mockSave, isLoading: false}),
        useNotify: () => jest.fn(),
    }
})
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({tenantId: "tenant", isAuthorized: () => true}),
}))
jest.mock("react-i18next", () => {
    const en = jest.requireActual("@/translations/en").default.translations
    const t = (key: string) => {
        const text = key.split(".").reduce((node: any, part: string) => node?.[part], en)
        return typeof text === "string" ? text : key
    }
    return {useTranslation: () => ({t, i18n: {language: "en"}})}
})

// The tab is typed with `void` props, as the settings screen renders it.
const LookAndFeel = SettingsLookAndFeel as unknown as React.FC

const LINKS = [{url: "https://help.example.test/start", title: "Getting started"}]
const tenant = (configuration_signing?: unknown) => ({
    id: "tenant",
    annotations: {logo_url: "https://logo.example.test/old.svg", css: "body {color: red}"},
    settings: {
        voting_countries: ["ES"],
        help_links: LINKS,
        ...(configuration_signing ? {configuration_signing} : {}),
    },
})
const fields = () =>
    screen.getAllByRole("textbox").map((field) => field.previousSibling?.textContent)
const edit = (name: string | RegExp, value: string) =>
    fireEvent.blur(screen.getByRole("textbox", {name}), {target: {value}})

beforeEach(() => {
    mockSave.mockReset()
    // The form logs what it saves.
    jest.spyOn(console, "log").mockImplementation(() => undefined)
})
afterEach(() => jest.restoreAllMocks())

it("offers every field where signed configuration packages are optional", () => {
    mockRecord = tenant({policy: "optional"})
    render(<LookAndFeel />)
    expect(fields()).toEqual(["Logo URL", "Custom CSS", "Display name", "Help Links"])
    edit("Logo URL", "https://logo.example.test/new.svg")
    edit("Custom CSS", "body {color: blue}")
    edit("Help Links", "[]")
    fireEvent.click(screen.getByRole("button", {name: "Save"}))
    expect(mockSave).toHaveBeenCalledWith({
        annotations: {logo_url: "https://logo.example.test/new.svg", css: "body {color: blue}"},
        settings: {
            voting_countries: ["ES"],
            help_links: [],
            configuration_signing: {policy: "optional"},
        },
    })
})

it.each([
    ["requires signed packages", {policy: "required"}],
    ["has no signing setting", undefined],
])("offers only the display name for a tenant that %s, keeping the rest", (_, signing) => {
    mockRecord = tenant(signing)
    render(<LookAndFeel />)
    expect(fields()).toEqual(["Display name"])
    expect(
        screen.getByText("The organization's name in messages that name it.", {exact: false})
    ).toBeInTheDocument()
    // The field's name includes its helper text.
    edit(/^Display name/, "Example Council")
    fireEvent.click(screen.getByRole("button", {name: "Save"}))
    expect(mockSave).toHaveBeenCalledWith({
        annotations: tenant(signing).annotations,
        settings: {...tenant(signing).settings, display_name: "Example Council"},
    })
})
