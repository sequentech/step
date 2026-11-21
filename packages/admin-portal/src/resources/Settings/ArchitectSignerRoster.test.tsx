/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import {TextDecoder, TextEncoder} from "node:util"
import {ArchitectSignerRoster, readExactRoster, RosterStatus} from "./ArchitectSignerRoster"

const mockQuery = jest.fn()
const mockActivate = jest.fn()
const mockRefresh = jest.fn()
const mockReauthenticate = jest.fn()
let mockActor = "operator-a"
const sha = "ab".repeat(32)
const source = '{\r\n  "public": true\r\n}\r\n'
const pending: RosterStatus = {
    mode: "portal_managed",
    generation: "0",
    active_sha256: null,
    required_approvals: 2,
    can_manage: true,
}
const active: RosterStatus = {...pending, generation: "1", active_sha256: sha}

jest.mock("@apollo/client", () => ({
    gql: (chunks: TemplateStringsArray) => chunks[0],
    useApolloClient: () => ({query: mockQuery}),
    useMutation: () => [mockActivate],
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        get userId() {
            return mockActor
        },
        reauthenticate: () => mockReauthenticate(),
    }),
}))

const mode = (status: RosterStatus | null, tenantId = "tenant") => (
    <ArchitectSignerRoster
        tenantId={tenantId}
        status={status}
        loading={false}
        refresh={mockRefresh}
    />
)
const name = (key: string) => `configurationPackage.roster.${key}`
const button = (key: string) => screen.getByRole("button", {name: name(key)})
const select = (content: string = source) => {
    const bytes = Uint8Array.from(new TextEncoder().encode(content))
    const file = new File([bytes.buffer], "roster.json", {type: "application/json"})
    Object.defineProperty(file, "arrayBuffer", {value: async () => bytes.buffer})
    fireEvent.change(screen.getByLabelText(name("file")), {target: {files: [file]}})
    fireEvent.change(screen.getByLabelText(name("expected")), {target: {value: sha}})
    return file
}

const response = (generation = "0") => ({
    data: {
        preview_architect_signer_roster: {
            generation,
            candidate_sha256: sha,
            approver_count: 2,
            final_signer_count: 1,
            rp_id: "example.test",
            origins: ["https://example.test"],
        },
    },
})

beforeEach(() => {
    jest.resetAllMocks()
    mockActor = "operator-a"
    Object.assign(globalThis, {TextDecoder})
    mockQuery.mockResolvedValue(response())
    mockActivate.mockResolvedValue({data: {activate_architect_signer_roster: active}})
    mockRefresh.mockResolvedValue(active)
})

it("shows a locked site-pinned roster even if a caller erroneously claims can_manage", () => {
    render(mode({...active, mode: "site_pinned"}))
    expect(screen.getByText(name("active"))).toBeTruthy()
    expect(screen.getByText(name("sitePinned"))).toBeTruthy()
    expect(screen.queryByRole("button", {name: name("manage")})).toBeNull()
})

it("never claims absent, unavailable, or pending trust is active", () => {
    const view = render(mode(null))
    expect(screen.getByText(name("unavailable"))).toBeTruthy()
    view.rerender(mode({...pending, can_manage: false}))
    expect(screen.getByText(name("noRoster"))).toBeTruthy()
    expect(screen.getByText(name("locked"))).toBeTruthy()
    expect(screen.queryByText(name("active"))).toBeNull()
    expect(screen.queryByRole("button", {name: name("manage")})).toBeNull()
})

it("sends exact bytes and activates only on an explicit fresh confirmation, once", async () => {
    render(mode(pending))
    fireEvent.click(button("manage"))
    select()
    fireEvent.click(button("review"))
    await screen.findByText(name("candidate"))
    expect(mockQuery).toHaveBeenCalledWith(
        expect.objectContaining({
            variables: {tenantId: "tenant", roster: source, sha},
            fetchPolicy: "no-cache",
        })
    )
    expect(mockActivate).not.toHaveBeenCalled()
    fireEvent.click(button("confirm"))
    fireEvent.click(button("confirm"))
    await waitFor(() => expect(mockRefresh).toHaveBeenCalledTimes(1))
    expect(mockActivate).toHaveBeenCalledTimes(1)
    expect(mockActivate).toHaveBeenCalledWith({
        variables: {tenantId: "tenant", roster: source, sha, generation: "0"},
    })
})

it("discards the candidate when fresh authentication is required and never resubmits", async () => {
    mockActivate.mockRejectedValueOnce({
        graphQLErrors: [{extensions: {code: "architect-roster-fresh-auth-required"}}],
    })
    render(mode(pending))
    fireEvent.click(button("manage"))
    select()
    fireEvent.click(button("review"))
    await screen.findByText(name("candidate"))
    fireEvent.click(button("confirm"))
    await screen.findByText(name("error.freshAuth"))
    expect((screen.getByLabelText(name("expected")) as HTMLInputElement).value).toBe("")
    expect(screen.queryByText(name("candidate"))).toBeNull()
    fireEvent.click(button("signIn"))
    await waitFor(() => expect(mockReauthenticate).toHaveBeenCalledTimes(1))
    expect(mockActivate).toHaveBeenCalledTimes(1)
    expect(mockQuery).toHaveBeenCalledTimes(1)
})

it("does not reuse a verified candidate when the authority generation changes", async () => {
    const view = render(mode(pending))
    fireEvent.click(button("manage"))
    select()
    fireEvent.click(button("review"))
    await screen.findByText(name("candidate"))
    view.rerender(mode(active))
    expect(screen.queryByText(name("candidate"))).toBeNull()
    expect((button("review") as HTMLButtonElement).disabled).toBe(true)
    expect(mockActivate).not.toHaveBeenCalled()
})

it("rejects a BOM and does not normalize the selected file", async () => {
    render(mode(pending))
    fireEvent.click(button("manage"))
    await expect(readExactRoster(select())).resolves.toBe(source)
    await expect(readExactRoster(select("\uFEFF" + source))).rejects.toThrow()
    expect(mockQuery).not.toHaveBeenCalled()
})

it("does not accept a digest or a quorum different from what must be reviewed", async () => {
    const bad = response()
    bad.data.preview_architect_signer_roster.approver_count = 1
    bad.data.preview_architect_signer_roster.candidate_sha256 = "ef".repeat(32)
    mockQuery.mockResolvedValueOnce(bad)
    render(mode(pending))
    fireEvent.click(button("manage"))
    select()
    fireEvent.click(button("review"))
    await screen.findByText(name("error.preview"))
    expect(screen.queryByRole("button", {name: name("confirm")})).toBeNull()
    expect(mockActivate).not.toHaveBeenCalled()
})

it("clears an open candidate after a same-tenant operator switch", async () => {
    const view = render(mode(pending))
    fireEvent.click(button("manage"))
    select()
    fireEvent.click(button("review"))
    await screen.findByText(name("candidate"))
    mockActor = "operator-b"
    view.rerender(mode(pending))
    expect(screen.queryByRole("dialog")).toBeNull()
    fireEvent.click(button("manage"))
    expect((screen.getByLabelText(name("expected")) as HTMLInputElement).value).toBe("")
    expect(screen.queryByText(name("candidate"))).toBeNull()
    mockActor = ""
    view.rerender(mode(pending))
    expect(screen.queryByRole("button", {name: name("manage")})).toBeNull()
    expect(mockActivate).not.toHaveBeenCalled()
})

it("shows an already-active fingerprint without prompting or attempting activation", async () => {
    mockQuery.mockResolvedValueOnce(response("1"))
    render(mode(active))
    fireEvent.click(button("manage"))
    select()
    fireEvent.click(button("review"))
    await screen.findByText(name("alreadyActive"))
    expect((button("confirm") as HTMLButtonElement).disabled).toBe(true)
    fireEvent.click(button("confirm"))
    expect(mockActivate).not.toHaveBeenCalled()
})
