/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {webcrypto} from "node:crypto"
import {cleanup, fireEvent, render, screen, waitFor} from "@testing-library/react"
import {CeremonyStep, KEYS_CEREMONY_CONFIGURATION, managerKeyFingerprint} from "./CeremonyStep"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {IPermissions} from "@/types/keycloak"

Object.defineProperty(globalThis, "crypto", {value: webcrypto, configurable: true})

/**
 * A stored Configuration for two fixture trustees, signed by the protocol
 * manager whose key is the one of RFC 8032, section 7.1, TEST 1, as Hasura
 * serializes the bytea: the bytes that protocol-board's test of the same
 * fingerprint reads (`src/configuration.rs`).
 */
const STORED_CONFIGURATION =
    "\\x" +
    [
        "000000000000001050726f746f636f6c204d616e61676572d75a980182b10ab7",
        "d54bfed3c964073a0ee172f3daa62325af021a68f707511a50deb02caf09af35",
        "92fbe3aa2871a43d1519856f1a579a706d945ad2ad62300ca575fc03d2baa04d",
        "01178a90bb375728674bfc0940b3463356c61ac23d2a83030000000000000000",
        "08000000006af2f44a0100000000000000d0d9792af071b849528aac94bc0fea",
        "d5f7d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707",
        "511a0000000000000002edb168a860525fc3610c3c6b3a30d741a82777444881",
        "9e92173728decf04a1d45a172ed7b87adf305edb912438d04ceddf0b632c7bcf",
        "2e83bf6277e7bb78d09400000000000000020000000000000001000000000000",
        "0002c4d0e2cd2acc8a42d2b4d7df2aff8deea2695055e40b190b988f79c011d5",
        "fa604071c2f14dd9ab7fb34b8114e91df691e8336cc9ea037d84cd53f86de356",
        "d506",
    ].join("")
/** SHA-256 of that key's 32 bytes, d75a9801…f707511a, as each trustee server logs it. */
const FINGERPRINT =
    "21:FE:31:DF:A1:54:A2:61:62:6B:F8:54:04:6F:D2:27:1B:7B:ED:4B:6A:BE:45:AA:58:87:7E:F4:7F:97:21:B9"
const CEREMONY_ID = "d9792af0-71b8-4952-8aac-94bc0fead5f7"

const mockUseQuery = jest.fn()
const mockPermissions = jest.fn()
const mockNotify = jest.fn()
const mockWrite = jest.fn()
const mockDiv = ({children}: {children?: React.ReactNode}) => <div>{children}</div>

jest.mock("@apollo/client", () => ({
    ...jest.requireActual("@apollo/client"),
    useQuery: (...args: unknown[]) => mockUseQuery(...args),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, string>) =>
            values ? `${key} ${JSON.stringify(values)}` : key,
    }),
}))
jest.mock("react-admin", () => ({
    useGetOne: () => ({data: undefined}),
    useNotify: () => mockNotify,
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/resources/ElectionEvent/useKeysPermissions", () => ({
    useKeysPermissions: () => mockPermissions(),
}))
jest.mock("@sequentech/ui-core", () => ({
    EElectionEventCeremoniesPolicy: {AUTOMATED_CEREMONIES: "automated"},
}))
jest.mock("@sequentech/ui-essentials", () => ({
    theme: {
        palette: {
            warning: {light: "#ffb74d"},
            info: {main: "#0288d1"},
            brandSuccess: "#2e7d32",
            errorColor: "#d32f2f",
            background: {default: "#ffffff"},
            common: {white: "#ffffff", black: "#000000"},
        },
    },
    Dialog: () => null,
}))
jest.mock("@/components/styles/WizardStyles", () => ({
    WizardStyles: new Proxy(
        {},
        {get: () => (props: {children?: React.ReactNode}) => mockDiv(props)}
    ),
}))
jest.mock("../Logs", () => ({Logs: () => null}))
jest.mock("@/resources/Tally/styles", () => ({
    CancelButton: ({children}: {children?: React.ReactNode}) => (
        <button type="button">{children}</button>
    ),
}))

const event = {id: "event", tenant_id: "tenant", presentation: {}} as Sequent_Backend_Election_Event
const board = {
    id: "board",
    name: "dkg_d9792af071b849528aac94bc0fead5f7",
    public_manager_message: STORED_CONFIGURATION,
}
const renderStep = () =>
    render(
        <CeremonyStep currentCeremonyId={CEREMONY_ID} electionEvent={event} goBack={jest.fn()} />
    )

beforeEach(() => {
    jest.clearAllMocks()
    mockPermissions.mockReturnValue({canAdminCeremony: true})
    mockUseQuery.mockReturnValue({data: {sequent_backend_protocol_board: [board]}})
    mockWrite.mockResolvedValue(undefined)
    Object.defineProperty(navigator, "clipboard", {
        value: {writeText: mockWrite},
        configurable: true,
    })
})
afterEach(cleanup)

describe("managerKeyFingerprint", () => {
    it("is SHA-256 of the protocol manager key that a stored Configuration names", async () => {
        await expect(managerKeyFingerprint(STORED_CONFIGURATION)).resolves.toBe(FINGERPRINT)
    })
    it.each([
        ["that is not hexadecimal", "\\xzz"],
        ["without the bytea prefix", STORED_CONFIGURATION.slice(2)],
        ["shorter than a name length and a key", "\\x" + "00".repeat(39)],
        ["whose name runs past its end", "\\x00000000000000ff" + "00".repeat(40)],
        ["whose name length needs more than 32 bits", "\\x0000000100000000" + "00".repeat(40)],
    ])("refuses a message %s", async (_case, message) => {
        await expect(managerKeyFingerprint(message)).rejects.toThrow("Invalid public configuration")
    })
})

describe("CeremonyStep protocol manager key", () => {
    it("shows the complete fingerprint with a copy control and the board's name", async () => {
        renderStep()
        expect(await screen.findByText(FINGERPRINT)).toBeTruthy()
        expect(
            screen.getByText(
                `keysGeneration.ceremonyStep.managerKey.help ${JSON.stringify({board: board.name})}`
            )
        ).toBeTruthy()
        expect(mockUseQuery).toHaveBeenCalledWith(
            KEYS_CEREMONY_CONFIGURATION,
            expect.objectContaining({
                variables: {tenantId: "tenant", keysCeremonyId: CEREMONY_ID},
                skip: false,
                context: {headers: {"x-hasura-role": IPermissions.ADMIN_CEREMONY}},
            })
        )
        fireEvent.click(screen.getByRole("button", {name: "copyableFingerprint.copy"}))
        await waitFor(() => expect(mockWrite).toHaveBeenCalledWith(FINGERPRINT))
    })
    it("reads nothing for an account without the ceremony administration permission", () => {
        mockPermissions.mockReturnValue({canAdminCeremony: false})
        renderStep()
        expect(mockUseQuery).not.toHaveBeenCalled()
        expect(screen.queryByText(FINGERPRINT)).toBeNull()
    })
    it.each([
        ["the read fails", {error: new Error("denied")}],
        [
            "the stored message is not a Configuration",
            {data: {sequent_backend_protocol_board: [{...board, public_manager_message: "\\x00"}]}},
        ],
        [
            "the message is withheld",
            {data: {sequent_backend_protocol_board: [{...board, public_manager_message: null}]}},
        ],
        [
            "two boards answer",
            {data: {sequent_backend_protocol_board: [board, {...board, id: "other"}]}},
        ],
    ])("warns instead of showing a value when %s", async (_case, result) => {
        mockUseQuery.mockReturnValue(result)
        renderStep()
        expect(
            await screen.findByText("keysGeneration.ceremonyStep.managerKey.readError")
        ).toBeTruthy()
        expect(screen.queryByText(FINGERPRINT)).toBeNull()
    })
})
