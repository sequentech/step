/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {PrivacySettings} from "./PrivacySettings"

const mockQuery = jest.fn()
const mockRecord = jest.fn()
const mockHash = jest.fn()
const mockChange = jest.fn()
jest.mock("@apollo/client", () => ({
    ...jest.requireActual("@apollo/client"),
    useQuery: (...args: unknown[]) => mockQuery(...args),
}))
jest.mock("react-admin", () => ({
    useRecordContext: () => mockRecord(),
    useInput: () => ({
        field: {
            name: "presentation.network_data_policy",
            value: "do-not-store",
            onChange: mockChange,
        },
    }),
}))
jest.mock("@/services/transmissionPackage", () => ({
    publicKeySha256: (...args: unknown[]) => mockHash(...args),
    shortKeyFingerprint: () => "key-fingerprint",
}))
jest.mock("@sequentech/ui-core", () => ({
    EBallotIdentity: {PSEUDONYM: "pseudonym", VOTER_ID: "voter-id"},
    ENetworkDataPolicy: {FORENSIC_STORE: "forensic-store", DO_NOT_STORE: "do-not-store"},
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        PseudonymChip: ({children}: React.PropsWithChildren) => <span>{children}</span>,
    }),
    {virtual: true}
)
jest.mock("react-router", () => ({
    Link: ({to, children, ...props}: React.PropsWithChildren<{to: string}>) => (
        <a href={to} {...props}>
            {children}
        </a>
    ),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))

const summary = {
    system_public_key: "published-key",
    roles: {forensic_read: ["Election reviewers"], pseudonym_resolve: []},
}
const retry = jest.fn()
beforeEach(() => {
    jest.clearAllMocks()
    mockRecord.mockReturnValue({id: "event", presentation: {ballot_identity: "pseudonym"}})
    mockQuery.mockReturnValue({data: {ballot_privacy_settings: {payload: summary}}, refetch: retry})
    mockHash.mockResolvedValue("key-fingerprint")
})

test("new event shows separate permission holders and allows an explicit network policy choice", async () => {
    render(<PrivacySettings canEdit />)
    expect(screen.getByText("Election reviewers")).toBeVisible()
    expect(screen.getByText("ballotPrivacy.settings.noRoles")).toBeVisible()
    expect(screen.getByRole("link", {name: "ballotPrivacy.settings.roles"})).toHaveAttribute(
        "href",
        "/user-roles"
    )
    fireEvent.click(screen.getByRole("radio", {name: "ballotPrivacy.settings.forensic"}))
    expect(mockChange).toHaveBeenCalledTimes(1)
    await waitFor(() =>
        expect(mockHash).toHaveBeenCalledWith(
            "-----BEGIN PUBLIC KEY-----\npublished-key\n-----END PUBLIC KEY-----"
        )
    )
})

test.each([false, true])(
    "legacy identity or read-only access disables network-policy changes (%s)",
    async (legacy) => {
        mockRecord.mockReturnValue({
            id: "event",
            presentation: {ballot_identity: legacy ? "voter-id" : "pseudonym"},
        })
        render(<PrivacySettings canEdit={legacy} />)
        expect(screen.getByRole("radio", {name: "ballotPrivacy.settings.forensic"})).toBeDisabled()
        expect(screen.getByRole("radio", {name: "ballotPrivacy.settings.none"})).toBeDisabled()
        if (legacy) expect(screen.getByText("ballotPrivacy.settings.legacyNetwork")).toBeVisible()
        await waitFor(() => expect(mockHash).toHaveBeenCalled())
    }
)

test.each(["error", "missing"])(
    "a settings %s offers retry and hides stale permission holders",
    async (failure) => {
        retry.mockRejectedValue(new Error("Still unavailable"))
        mockQuery.mockReturnValue({
            ...(failure === "error"
                ? {
                      data: {ballot_privacy_settings: {payload: summary}},
                      error: new Error("Unavailable"),
                  }
                : {}),
            refetch: retry,
        })
        render(<PrivacySettings canEdit />)
        expect(screen.getByText("ballotPrivacy.settings.loadError")).toBeVisible()
        expect(screen.queryByText("Election reviewers")).not.toBeInTheDocument()
        fireEvent.click(screen.getByRole("button", {name: "ballotPrivacy.retry"}))
        await waitFor(() => expect(retry).toHaveBeenCalledTimes(1))
    }
)

test("loading never shows stale role assignments, and fingerprint failures do not imply verification", async () => {
    mockHash.mockRejectedValue(new Error("Malformed key"))
    mockQuery.mockReturnValue({loading: true, data: {ballot_privacy_settings: {payload: summary}}})
    render(<PrivacySettings canEdit />)
    expect(screen.getByRole("progressbar", {name: "ballotPrivacy.settings.loading"})).toBeVisible()
    expect(screen.queryByText("Election reviewers")).not.toBeInTheDocument()
    await waitFor(() => expect(mockHash).toHaveBeenCalled())
})
