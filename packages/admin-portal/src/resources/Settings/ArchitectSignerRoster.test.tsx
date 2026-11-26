/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {ThemeProvider} from "@mui/material/styles"
import {adminTheme} from "../../../../ui-essentials/src/services/theme"
import {TextDecoder, TextEncoder} from "node:util"
import {createHash, webcrypto} from "node:crypto"
import forge from "node-forge/lib/forge"
import {
    ArchitectSignerRoster,
    computeRosterFingerprint,
    readExactRoster,
    RosterStatus,
} from "./ArchitectSignerRoster"

// LinkBehavior is not exercised by these controls.
jest.mock("react-router-dom", () => ({Link: "a"}))

// Execute the actual shared drop control and its ref helper, not a fake file input.
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        CustomDropFile: jest.requireActual(
            "../../../../ui-essentials/src/components/CustomDropFile/CustomDropFile"
        ).CustomDropFile,
    }),
    {virtual: true}
)
jest.mock("@sequentech/ui-core", () => ({
    useForwardedRef: jest.requireActual("../../../../ui-core/src/utils/ref").useForwardedRef,
}))

const mockQuery = jest.fn()
const mockActivate = jest.fn()
const mockRefresh = jest.fn()
const mockReauthenticate = jest.fn()
const mockReauthWithGold = jest.fn()
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
        reauthWithGold: (...args: unknown[]) => mockReauthWithGold(...args),
    }),
}))

const mode = (status: RosterStatus | null, tenantId = "tenant", loading = false) => (
    <ThemeProvider theme={adminTheme}>
        <ArchitectSignerRoster
            tenantId={tenantId}
            status={status}
            loading={loading}
            refresh={mockRefresh}
        />
    </ThemeProvider>
)
const name = (key: string) => `configurationPackage.roster.${key}`
const button = (key: string) => screen.getByRole("button", {name: name(key)}) as HTMLButtonElement
const rosterFile = (content: string = source, filename = "roster.json") => {
    const bytes = Uint8Array.from(new TextEncoder().encode(content))
    const file = new File([bytes.buffer], filename, {type: "application/json"})
    Object.defineProperty(file, "arrayBuffer", {value: async () => bytes.buffer})
    return file
}
const select = async (content: string = source, entered = sha) => {
    const file = rosterFile(content)
    await act(async () => {
        fireEvent.change(screen.getByLabelText(name("file")), {target: {files: [file]}})
        fireEvent.change(screen.getByLabelText(name("expected")), {target: {value: entered}})
    })
    return file
}
const drop = (files: File[]) => {
    const drawer = screen.getByRole("dialog", {name: name("manage")})
    const area = drawer.querySelector(".drop-file-dropzone")
    if (!area) throw new Error("The shared dropzone is missing")
    const dataTransfer = {files, types: ["Files"]}
    fireEvent.dragEnter(area, {dataTransfer})
    const target = drawer.querySelector(".drag-file-element")
    if (!target) throw new Error("The shared drop target is missing")
    fireEvent.drop(target, {dataTransfer})
}

const response = (generation = "0", candidateSha = sha) => ({
    data: {
        preview_architect_signer_roster: {
            generation,
            candidate_sha256: candidateSha,
            approver_count: 2,
            final_signer_count: 1,
            rp_id: "example.test",
            origins: ["https://example.test"],
        },
    },
})

const originalSubtle = Object.getOwnPropertyDescriptor(globalThis.crypto, "subtle")
const setSubtle = (subtle: unknown) => {
    Object.defineProperty(globalThis.crypto, "subtle", {value: subtle, configurable: true})
}
afterEach(() => {
    if (originalSubtle) Object.defineProperty(globalThis.crypto, "subtle", originalSubtle)
    else Reflect.deleteProperty(globalThis.crypto, "subtle")
})
const digest = (text: string) => createHash("sha256").update(text, "utf8").digest("hex")

beforeEach(() => {
    jest.resetAllMocks()
    mockActor = "operator-a"
    Object.assign(globalThis, {TextDecoder})
    setSubtle(webcrypto.subtle)

    mockQuery.mockResolvedValue(response())
    mockActivate.mockResolvedValue({data: {activate_architect_signer_roster: active}})
    mockRefresh.mockResolvedValue(active)
})

it("shows a locked site-pinned roster even if a caller erroneously claims can_manage", () => {
    render(mode({...active, mode: "site_pinned"}))
    expect(screen.getByText(name("activeSummary"))).toBeTruthy()
    // The active fingerprint stays on the page, behind a keyboard-reachable disclosure.
    const fingerprint = screen.getByTestId("signer-roster-fingerprint")
    expect(fingerprint.tagName).toBe("DETAILS")
    expect(fingerprint.querySelector("summary")?.textContent).toBe(name("fingerprint"))
    expect(fingerprint.textContent).toContain(sha)
    expect(screen.getByText(name("sitePinned"))).toBeTruthy()
    expect(screen.queryByRole("button", {name: name("manage")})).toBeNull()
})

it("never claims absent, unavailable, or pending trust is active", () => {
    const view = render(mode(null))
    expect(screen.getByText(name("unavailable"))).toBeTruthy()
    expect(screen.queryByText(name("activeSummary"))).toBeNull()
    expect(screen.queryByTestId("signer-roster-fingerprint")).toBeNull()
    view.rerender(mode({...pending, can_manage: false}))
    expect(screen.getByText(name("noRoster"))).toBeTruthy()
    expect(screen.getByText(name("locked"))).toBeTruthy()
    expect(screen.queryByText(name("activeSummary"))).toBeNull()
    expect(screen.queryByTestId("signer-roster-fingerprint")).toBeNull()
    expect(screen.queryByRole("button", {name: name("manage")})).toBeNull()
})

it("sends exact bytes and activates only on an explicit fresh confirmation, once", async () => {
    render(mode(pending))
    fireEvent.click(button("manage"))
    const current = () =>
        screen.getByTestId("signer-roster-steps").querySelector('[aria-current="step"]')
            ?.textContent
    expect(current()).toContain("configurationPackage.roster.steps.choose")
    await select()
    fireEvent.click(button("review"))
    await screen.findByText(name("candidate"))
    // Verification done: Activate is the current step and Verify is marked done.
    expect(current()).toContain("configurationPackage.roster.steps.activate")
    const verify = Array.from(
        screen.getByTestId("signer-roster-steps").querySelectorAll("li")
    ).find((step) => step.textContent?.includes("configurationPackage.roster.steps.verify"))
    expect(verify?.textContent?.startsWith("✓")).toBe(true)
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
    await select()
    fireEvent.click(button("review"))
    await screen.findByText(name("candidate"))
    fireEvent.click(button("confirm"))
    await screen.findByText(name("error.freshAuth"))
    expect((screen.getByLabelText(name("expected")) as HTMLInputElement).value).toBe("")
    expect(screen.queryByText(name("candidate"))).toBeNull()
    fireEvent.click(button("signIn"))
    await waitFor(() => expect(mockReauthenticate).toHaveBeenCalledTimes(1))
    expect(mockReauthenticate).toHaveBeenCalledWith()
    expect(mockReauthWithGold).not.toHaveBeenCalled()
    expect(mockActivate).toHaveBeenCalledTimes(1)
    expect(mockQuery).toHaveBeenCalledTimes(1)
})

it("does not reuse a verified candidate when the authority generation changes", async () => {
    const view = render(mode(pending))
    fireEvent.click(button("manage"))
    await select()
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
    await expect(readExactRoster(await select())).resolves.toBe(source)
    await expect(readExactRoster(await select("\uFEFF" + source))).rejects.toThrow()
    expect(mockQuery).not.toHaveBeenCalled()
})

it("does not accept a digest or a quorum different from what must be reviewed", async () => {
    const bad = response()
    bad.data.preview_architect_signer_roster.approver_count = 1
    bad.data.preview_architect_signer_roster.candidate_sha256 = "ef".repeat(32)
    mockQuery.mockResolvedValueOnce(bad)
    render(mode(pending))
    fireEvent.click(button("manage"))
    await select()
    fireEvent.click(button("review"))
    await screen.findByText(name("error.preview"))
    expect(screen.queryByRole("button", {name: name("confirm")})).toBeNull()
    expect(mockActivate).not.toHaveBeenCalled()
})

it("clears an open candidate after a same-tenant operator switch", async () => {
    const view = render(mode(pending))
    fireEvent.click(button("manage"))
    await select()
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
    await select()
    fireEvent.click(button("review"))
    await screen.findByText(name("alreadyActive"))
    expect((button("confirm") as HTMLButtonElement).disabled).toBe(true)
    fireEvent.click(button("confirm"))
    expect(mockActivate).not.toHaveBeenCalled()
})

it("uses a named right drawer, accepts a dropped JSON, and rejects batch and wrong-extension drops", async () => {
    render(mode(pending))
    fireEvent.click(button("manage"))
    const drawer = screen.getByRole("dialog", {name: name("manage")})
    expect(drawer.className).toContain("MuiDrawer-paperAnchorRight")
    fireEvent.change(screen.getByLabelText(name("expected")), {target: {value: sha}})

    drop([rosterFile(source, "wrong.txt")])
    await waitFor(() => expect(button("browse").disabled).toBe(false))
    expect(within(drawer).getByRole("alert").textContent).toContain(name("error.file"))
    expect(button("review").disabled).toBe(true)
    drop([rosterFile(), rosterFile(source, "second.json")])
    await waitFor(() => expect(button("browse").disabled).toBe(false))
    expect(button("review").disabled).toBe(true)
    drop([rosterFile()])
    await waitFor(() => expect(button("browse").disabled).toBe(false))
    expect(within(drawer).getByText("roster.json")).toBeTruthy()
    expect(button("review").disabled).toBe(false)
    expect(mockQuery).not.toHaveBeenCalled()
    fireEvent.click(button("review"))
    const verified = await screen.findByRole("heading", {name: name("candidate")})
    expect(document.activeElement).toBe(verified)
    expect(mockQuery.mock.calls[0][0].variables).toEqual({tenantId: "tenant", roster: source, sha})
    expect(mockActivate).not.toHaveBeenCalled()
})

it("opens Browse by keyboard and keeps the authoritative selected file when the picker is canceled", async () => {
    render(mode(pending))
    fireEvent.click(button("manage"))
    await select()
    const input = screen.getByLabelText(name("file"))
    const click = jest.spyOn(input, "click").mockImplementation(() => {})
    await waitFor(() => expect(button("browse").disabled).toBe(false))
    act(() => button("browse").focus())
    await userEvent.keyboard("{Enter}")
    expect(click).toHaveBeenCalledTimes(1)
    // No input change is emitted after a user cancels the system picker.
    expect(screen.getByText("roster.json")).toBeTruthy()
    expect(button("review").disabled).toBe(false)
    expect(mockQuery).not.toHaveBeenCalled()
    click.mockRestore()
})

it("discards an abandoned Verify response and restores focus on Cancel", async () => {
    let complete!: (value: ReturnType<typeof response>) => void
    mockQuery.mockImplementationOnce(
        () =>
            new Promise((resolve) => {
                complete = resolve
            })
    )
    render(mode(pending))
    const trigger = button("manage")
    act(() => trigger.focus())
    fireEvent.click(trigger)
    await select()
    fireEvent.click(button("review"))
    await waitFor(() => expect(mockQuery).toHaveBeenCalledTimes(1))
    fireEvent.click(button("cancel"))
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull())
    expect(document.activeElement).toBe(trigger)
    fireEvent.click(trigger)
    await act(async () => complete(response()))
    expect(screen.queryByText(name("candidate"))).toBeNull()
    expect(screen.queryByText("roster.json")).toBeNull()
    expect(button("review").disabled).toBe(true)
    expect(mockActivate).not.toHaveBeenCalled()
})

it("does not dismiss during activation, then closes after observing the new authoritative roster", async () => {
    let complete!: (value: {data: {activate_architect_signer_roster: RosterStatus}}) => void
    mockActivate.mockImplementationOnce(
        () =>
            new Promise((resolve) => {
                complete = resolve
            })
    )
    const view = render(mode(pending))
    fireEvent.click(button("manage"))
    await select()
    fireEvent.click(button("review"))
    await screen.findByText(name("candidate"))
    fireEvent.click(button("confirm"))
    expect(button("cancel").disabled).toBe(true)
    await userEvent.keyboard("{Escape}")
    expect(screen.getByRole("dialog")).toBeTruthy()
    await act(async () => complete({data: {activate_architect_signer_roster: active}}))
    view.rerender(mode(active))
    await screen.findByText(name("activeConfirmed"))
    expect(screen.queryByRole("dialog")).toBeNull()
    expect(mockActivate).toHaveBeenCalledTimes(1)
})

it("offers a neutral retry only after trust loading fails, without offering management", async () => {
    const view = render(mode(null, "tenant", true))
    expect(screen.getByText(name("loading"))).toBeTruthy()
    expect(screen.queryByRole("button", {name: name("retry")})).toBeNull()
    view.rerender(mode(null))
    fireEvent.click(button("retry"))
    expect(mockRefresh).toHaveBeenCalledTimes(1)
    expect(screen.queryByRole("button", {name: name("manage")})).toBeNull()
})

it("cannot repopulate a cleared roster form from a review of a newer polled authority", async () => {
    let complete!: (value: ReturnType<typeof response>) => void
    mockQuery.mockImplementationOnce(
        () =>
            new Promise((resolve) => {
                complete = resolve
            })
    )
    const view = render(mode(pending))
    fireEvent.click(button("manage"))
    await select()
    fireEvent.click(button("review"))
    await waitFor(() => expect(mockQuery).toHaveBeenCalledTimes(1))
    view.rerender(mode(active))
    await act(async () => complete(response("1")))
    expect(screen.queryByText(name("candidate"))).toBeNull()
    expect(screen.queryByText("roster.json")).toBeNull()
    expect(screen.queryByRole("button", {name: name("confirm")})).toBeNull()
    expect(button("review").disabled).toBe(true)
    expect(mockActivate).not.toHaveBeenCalled()
})

it("hashes the exact non-ASCII CRLF bytes when left blank, labels no independent match, and does not auto-activate", async () => {
    const body = '{\r\n  "label": "á雪"\r\n}\r\n  '
    const sha256 = digest(body)
    mockQuery.mockResolvedValueOnce(response("0", sha256))
    const installed = {...active, active_sha256: sha256}
    mockActivate.mockResolvedValueOnce({data: {activate_architect_signer_roster: installed}})
    mockRefresh.mockResolvedValueOnce(installed)
    render(mode(pending))
    fireEvent.click(button("manage"))
    await select(body, "  ")
    expect(button("review").disabled).toBe(false)
    fireEvent.click(button("review"))
    await screen.findByText(name("fingerprintComputed"))
    expect(screen.queryByText(name("fingerprintMatched"))).toBeNull()
    expect(mockQuery).toHaveBeenCalledWith(
        expect.objectContaining({
            variables: {tenantId: "tenant", roster: body, sha: sha256},
        })
    )
    expect(mockActivate).not.toHaveBeenCalled()
    expect(mockReauthenticate).not.toHaveBeenCalled()
    expect(mockReauthWithGold).not.toHaveBeenCalled()
    fireEvent.click(button("confirm"))
    await waitFor(() => expect(mockActivate).toHaveBeenCalledTimes(1))
    expect(mockActivate).toHaveBeenCalledWith({
        variables: {tenantId: "tenant", roster: body, sha: sha256, generation: "0"},
    })
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull())
})

it("normalizes an entered fingerprint and compares it on the receiver even without Web Crypto", async () => {
    setSubtle(undefined)
    render(mode(pending))
    fireEvent.click(button("manage"))
    await select(source, "  " + sha.toUpperCase() + " ")
    fireEvent.click(button("review"))
    await screen.findByText(name("fingerprintMatched"))
    expect(screen.queryByText(name("fingerprintComputed"))).toBeNull()
    expect(mockQuery).toHaveBeenCalledWith(
        expect.objectContaining({
            variables: {tenantId: "tenant", roster: source, sha},
        })
    )
    expect(mockActivate).not.toHaveBeenCalled()
})

it("never silently switches malformed or mismatched entered fingerprints to computed", async () => {
    const spy = jest.fn(webcrypto.subtle.digest.bind(webcrypto.subtle))
    setSubtle({digest: spy})
    render(mode(pending))
    fireEvent.click(button("manage"))
    await select(source, sha + "x")
    expect(button("review").disabled).toBe(true)
    fireEvent.click(button("review"))
    expect(mockQuery).not.toHaveBeenCalled()
    fireEvent.change(screen.getByLabelText(name("expected")), {target: {value: "cb".repeat(32)}})
    fireEvent.click(button("review"))
    await screen.findByText(name("error.preview"))
    expect(spy).not.toHaveBeenCalled()
    expect(screen.queryByText(name("fingerprintMatched"))).toBeNull()
    expect(screen.queryByRole("button", {name: name("confirm")})).toBeNull()
})

it.each([undefined, {digest: () => Promise.reject(new Error("no digest"))}])(
    "uses the pinned raw-byte fallback for a plain HTTP/offline browser (%p)",
    async (subtle) => {
        setSubtle(subtle)
        const body = '{"name":"ü雪"}\r\n '
        const hash = digest(body)
        mockQuery.mockResolvedValueOnce(response("0", hash))
        render(mode(pending))
        fireEvent.click(button("manage"))
        await select(body, "")
        fireEvent.click(button("review"))
        await screen.findByText(name("fingerprintComputed"))
        expect(mockQuery.mock.calls[0][0].variables.sha).toBe(hash)
        expect(mockActivate).not.toHaveBeenCalled()
    }
)

it("validates pinned SHA-256 known vectors and raw binary at maximum upload size", async () => {
    setSubtle(undefined)
    const abc = new TextEncoder().encode("abc")
    const raw = new Uint8Array(262144)
    raw.set([0x00, 0xff, 0x80, 0xc3, 0xa9], 16382)
    raw[raw.length - 1] = 0xff
    await expect(computeRosterFingerprint(new ArrayBuffer(0))).resolves.toBe(
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    )
    await expect(computeRosterFingerprint(abc.buffer)).resolves.toBe(
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    )
    await expect(computeRosterFingerprint(raw.buffer)).resolves.toBe(
        createHash("sha256").update(raw).digest("hex")
    )
})

it("reports failure of both hash paths without submitting roster bytes", async () => {
    setSubtle({digest: () => Promise.reject(new Error("unavailable"))})
    const create = jest.spyOn(forge.md.sha256, "create").mockImplementationOnce(() => {
        throw new Error("unavailable")
    })
    try {
        render(mode(pending))
        fireEvent.click(button("manage"))
        await select(source, "")
        fireEvent.click(button("review"))
        await screen.findByText(name("error.computeHash"))
        expect(mockQuery).not.toHaveBeenCalled()
        expect(mockActivate).not.toHaveBeenCalled()
        expect(screen.queryByRole("button", {name: name("confirm")})).toBeNull()
    } finally {
        create.mockRestore()
    }
})

it("does not accept a receiver fingerprint different from the locally hashed file", async () => {
    render(mode(pending))
    fireEvent.click(button("manage"))
    await select(source, "")
    fireEvent.click(button("review"))
    await screen.findByText(name("error.preview"))
    expect(mockQuery.mock.calls[0][0].variables.sha).toEqual(digest(source))
    expect(screen.queryByRole("button", {name: name("confirm")})).toBeNull()
})

it("ignores a canceled in-flight digest when reopening and selecting a different same-named file", async () => {
    let complete!: (value: ArrayBuffer) => void
    const slow = jest.fn(
        () =>
            new Promise<ArrayBuffer>((resolve) => {
                complete = resolve
            })
    )
    setSubtle({digest: slow})
    const replacement = '{"public":"different"}\r\n'
    mockQuery.mockResolvedValueOnce(response("0", digest(replacement)))
    render(mode(pending))
    fireEvent.click(button("manage"))
    await select(source, "")
    fireEvent.click(button("review"))
    await waitFor(() => expect(slow).toHaveBeenCalledTimes(1))
    fireEvent.click(button("cancel"))
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull())
    fireEvent.click(button("manage"))
    await select(replacement, "")
    await act(async () => complete(new ArrayBuffer(32)))
    expect(mockQuery).not.toHaveBeenCalled()
    setSubtle(webcrypto.subtle)
    fireEvent.click(button("review"))
    await screen.findByText(name("fingerprintComputed"))
    expect(mockQuery.mock.calls[0][0].variables).toEqual({
        tenantId: "tenant",
        roster: replacement,
        sha: digest(replacement),
    })
    expect(mockActivate).not.toHaveBeenCalled()
})
