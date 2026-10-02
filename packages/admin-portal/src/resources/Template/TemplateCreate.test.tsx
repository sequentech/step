/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {TemplateCreate} from "./TemplateCreate"

const mockCreate = jest.fn()
const mockNotify = jest.fn()
let mockFormData: Record<string, unknown> = {}

jest.mock("@apollo/client", () => ({
    gql: jest.fn(),
    useMutation: () => [mockCreate],
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))
jest.mock("react-admin", () => ({
    useNotify: () => mockNotify,
    CreateBase: ({children}: React.PropsWithChildren) => children,
    SimpleForm: ({
        children,
        onSubmit,
    }: React.PropsWithChildren<{onSubmit: (data: Record<string, unknown>) => void}>) => (
        <>
            {children}
            <button type="button" onClick={() => onSubmit(mockFormData)}>
                save
            </button>
        </>
    ),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-a"]}))
jest.mock("@/components/styles/PageHeaderStyles", () => ({
    PageHeaderStyles: {Wrapper: "div"},
}))
jest.mock("./TemplateFormContent", () => ({TemplateFormContent: () => null}))

const formData = (selected_methods: Record<string, boolean>) => ({
    type: "CREDENTIALS",
    template: {alias: "reminder", name: "Reminder", selected_methods},
})

const save = async () => {
    render(<TemplateCreate />)
    fireEvent.click(screen.getByRole("button", {name: "save"}))
    await waitFor(() => expect(mockNotify).toHaveBeenCalled())
}

beforeEach(() => {
    jest.clearAllMocks()
    mockCreate.mockResolvedValue({data: {insert_sequent_backend_template: {returning: []}}})
})

it("saves an SMS template as SMS", async () => {
    mockFormData = formData({SMS: true})
    await save()
    expect(mockCreate).toHaveBeenCalledTimes(1)
    expect(mockCreate.mock.calls[0][0].variables.object.communication_method).toBe("SMS")
})

it("saves a WhatsApp-only template as WhatsApp", async () => {
    mockFormData = formData({EMAIL: false, WHATSAPP: true})
    await save()
    expect(mockCreate.mock.calls[0][0].variables.object.communication_method).toBe("WHATSAPP")
})

it("keeps email first when several methods are selected", async () => {
    mockFormData = formData({SMS: true, EMAIL: true})
    await save()
    expect(mockCreate.mock.calls[0][0].variables.object.communication_method).toBe("EMAIL")
})

it("refuses to save a template without a method", async () => {
    mockFormData = formData({})
    await save()
    expect(mockCreate).not.toHaveBeenCalled()
    expect(mockNotify).toHaveBeenCalledWith("messaging.templates.noMethod", {type: "error"})
})
