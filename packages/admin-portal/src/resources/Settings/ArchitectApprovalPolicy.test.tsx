/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import {ArchitectApprovalPolicy} from "./ArchitectApprovalPolicy"

const mockRead = jest.fn()
const mockWrite = jest.fn()

jest.mock("@apollo/client", () => ({
    gql: jest.fn(),
    useQuery: (_: unknown, options: unknown) => mockRead(options),
    useMutation: () => [mockWrite, {loading: false}],
}))

jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))

describe("Architect revision approval policy", () => {
    beforeEach(() => {
        jest.clearAllMocks()
        mockRead.mockReturnValue({
            data: {get_architect_approval_policy: {required_approvals: null, generation: "0"}},
        })
    })

    it("rejects bad values, supports an override, and clears back to the default", async () => {
        mockWrite
            .mockResolvedValueOnce({
                data: {set_architect_approval_policy: {required_approvals: 3, generation: "1"}},
            })
            .mockResolvedValueOnce({
                data: {set_architect_approval_policy: {required_approvals: null, generation: "2"}},
            })
        render(<ArchitectApprovalPolicy tenantId="tenant-a" />)
        const input = screen.getByRole("textbox") as HTMLInputElement
        const save = screen.getByRole("button", {name: "architectApprovalPolicy.save"})
        expect(mockRead).toHaveBeenCalledWith(
            expect.objectContaining({
                variables: {tenantId: "tenant-a"},
                context: {headers: {"x-hasura-role": "tenant-write"}},
            })
        )
        expect(input.value).toBe("")
        expect(input.placeholder).toBe("2")
        for (const value of ["0", "-1", "1.5", "17", " "]) {
            fireEvent.change(input, {target: {value}})
            expect((save as HTMLButtonElement).disabled).toBe(true)
        }
        fireEvent.change(input, {target: {value: "3"}})
        fireEvent.click(save)
        await waitFor(() => expect(screen.getByText("architectApprovalPolicy.saved")).toBeTruthy())
        expect(mockWrite).toHaveBeenLastCalledWith({
            context: {headers: {"x-hasura-role": "tenant-write"}},
            variables: {tenantId: "tenant-a", count: 3, generation: "0"},
        })
        fireEvent.change(input, {target: {value: ""}})
        fireEvent.click(save)
        await waitFor(() => expect(screen.getByText("architectApprovalPolicy.saved")).toBeTruthy())
        expect(mockWrite).toHaveBeenLastCalledWith({
            context: {headers: {"x-hasura-role": "tenant-write"}},
            variables: {tenantId: "tenant-a", count: null, generation: "1"},
        })
    })
})
