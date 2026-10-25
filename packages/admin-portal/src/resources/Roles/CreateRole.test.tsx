/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {CreateRole} from "./CreateRole"

const mockCreateRole = jest.fn()
const translate = (key: string) => key.replace("usersAndRolesScreen.permissions.", "")
jest.mock("react-i18next", () => ({useTranslation: () => ({t: translate})}))
jest.mock("@apollo/client", () => ({
    gql: (source: TemplateStringsArray) => source.join(""),
    useMutation: () => [mockCreateRole],
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-a"]}))
jest.mock("./EditRole", () => ({getEnumValues: (value: object) => Object.values(value)}))
jest.mock("@/components/styles/PageHeaderStyles", () => ({
    PageHeaderStyles: {Wrapper: "div", Title: "h1", SubTitle: "p"},
}))
// Keep the form and grid boundaries small: the component owns the actual row
// derivation, selection state and submitted role; these render their contracts.
jest.mock("react-admin", () => ({
    useNotify: () => jest.fn(),
    useRefresh: () => jest.fn(),
    SaveButton: () => <button type="submit">Save</button>,
    SimpleForm: ({children, toolbar, onSubmit}: any) => (
        <form
            onSubmit={(event) => {
                event.preventDefault()
                onSubmit()
            }}
        >
            {children}
            {toolbar}
        </form>
    ),
}))
jest.mock("@mui/material", () => ({
    Box: ({children}: any) => <div>{children}</div>,
    TextField: ({label, name, value, onChange}: any) => (
        <label>
            {label}
            <input name={name} value={value} onChange={onChange} />
        </label>
    ),
    Checkbox: ({checked, onClick}: any) => (
        <input type="checkbox" checked={checked} onClick={onClick} readOnly />
    ),
}))
jest.mock("@mui/x-data-grid", () => ({
    GridToolbar: () => null,
    DataGrid: ({rows, columns}: any) => (
        <table>
            <tbody>
                {rows.map((row: any) => (
                    <tr key={row.id}>
                        {columns.map((column: any) => (
                            <td key={column.field}>
                                {column.renderCell
                                    ? column.renderCell({row, value: row[column.field]})
                                    : row[column.field]}
                            </td>
                        ))}
                    </tr>
                ))}
            </tbody>
        </table>
    ),
}))

const permissions = [
    {id: "role-read-id", name: "role-read"},
    {id: "user-read-id", name: "user-read"},
]
const checkbox = (name: string) => {
    const row = screen.getByText(name).closest("tr")!
    return row.querySelector('input[type="checkbox"]') as HTMLInputElement
}
beforeEach(() => {
    mockCreateRole.mockReset().mockResolvedValue({data: {create_role: {id: "new-role"}}})
})

it("lists permissions arriving after the role form mounted and submits the chosen grant", async () => {
    const {rerender} = render(<CreateRole />)
    expect(screen.queryByText("user-read")).not.toBeInTheDocument()
    fireEvent.change(screen.getByRole("textbox"), {target: {value: "Election board"}})
    rerender(<CreateRole permissions={permissions} />)
    expect(screen.getByText("user-read")).toBeVisible()
    fireEvent.click(checkbox("user-read"))
    expect(checkbox("user-read")).toBeChecked()
    expect(mockCreateRole).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole("button", {name: "Save"}))
    await waitFor(() => expect(mockCreateRole).toHaveBeenCalledTimes(1))
    expect(mockCreateRole.mock.calls[0][0].variables.role).toMatchObject({
        name: "Election board",
        permissions: ["user-read"],
    })
})

it("keeps a pending selection while a refreshed permission list adds another choice", async () => {
    const {rerender} = render(<CreateRole permissions={permissions.slice(0, 1)} />)
    fireEvent.click(checkbox("role-read"))
    rerender(<CreateRole permissions={permissions} />)
    expect(checkbox("role-read")).toBeChecked()
    expect(checkbox("user-read")).not.toBeChecked()
    fireEvent.click(checkbox("user-read"))
    fireEvent.click(screen.getByRole("button", {name: "Save"}))
    await waitFor(() => expect(mockCreateRole).toHaveBeenCalledTimes(1))
    expect(mockCreateRole.mock.calls[0][0].variables.role.permissions).toEqual([
        "role-read",
        "user-read",
    ])
})
