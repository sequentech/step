/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useQuery} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {cleanup, fireEvent, render, screen, waitFor} from "@testing-library/react"
import {InitializationCountryDialog} from "./InitializationCountryDialog"
import type {ILifecycleSnapshotEntry} from "@/queries/Lifecycle"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation"),
    ...jest.requireActual("../../../../ui-core/src/types/ElectionPresentation"),
}))
jest.mock("@apollo/client", () => ({
    gql: (s: TemplateStringsArray) => s.join(""),
    useQuery: jest.fn(() => ({
        data: {
            sequent_backend_area: [],
            sequent_backend_area_contest: [],
            sequent_backend_ballot_style: [],
            sequent_backend_contest: [],
        },
        loading: false,
    })),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
const mockPermissions = [
    IPermissions.AREA_READ,
    IPermissions.PUBLISH_READ,
    IPermissions.CONTEST_READ,
]
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: jest.requireActual("react").createContext({
        hasRole: (role: IPermissions) => mockPermissions.includes(role),
    }),
}))
afterEach(() => {
    cleanup()
    jest.clearAllMocks()
})

const snapshot = (countries?: Record<string, string[]>): ILifecycleSnapshotEntry => ({
    election_id: "post",
    publication_id: "publication",
    published_at: "2026-10-04T00:00:00Z",
    approval_request_id: null,
    approval_code: null,
    signed: false,
    snapshot: {schedule: [], initialization_countries: countries},
})

it("generates an ordinary whole-Post report for explicitly proven empty membership", async () => {
    const generate = jest.fn().mockResolvedValue(true)
    const close = jest.fn()
    render(
        <InitializationCountryDialog
            electionEventId="event"
            electionId="post"
            busy={false}
            snapshots={[snapshot({post: []})]}
            onClose={close}
            onGenerate={generate}
        />
    )
    const button = screen.getByRole("button", {
        name: "publish.action.generateInitializationReport",
    }) as HTMLButtonElement
    expect(button.disabled).toBe(false)
    expect(screen.getByRole("combobox").textContent).toContain("publish.initialization.entirePost")
    fireEvent.click(button)
    await waitFor(() => expect(generate).toHaveBeenCalledWith(undefined))
    expect(close).toHaveBeenCalledTimes(1)
})

it("keeps unknown legacy membership blocked rather than treating it as empty", () => {
    const generate = jest.fn()
    render(
        <InitializationCountryDialog
            electionEventId="event"
            electionId="post"
            busy={false}
            snapshots={[snapshot()]}
            onClose={jest.fn()}
            onGenerate={generate}
        />
    )
    const button = screen.getByRole("button", {
        name: "publish.action.generateInitializationReport",
    }) as HTMLButtonElement
    expect(button.disabled).toBe(true)
    fireEvent.click(button)
    expect(generate).not.toHaveBeenCalled()
})

it("reads country labels and published material with their own held permissions", () => {
    render(
        <InitializationCountryDialog
            electionEventId="event"
            electionId="post"
            busy={false}
            onClose={jest.fn()}
            onGenerate={jest.fn()}
        />
    )
    expect(useQuery).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
            context: {headers: {"x-hasura-role": IPermissions.AREA_READ}},
        })
    )
    expect(useQuery).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({
            context: {headers: {"x-hasura-role": IPermissions.PUBLISH_READ}},
        })
    )
})

it("refuses generation when country material cannot be read with held permissions", () => {
    const saved = mockPermissions.splice(0)
    try {
        render(
            <InitializationCountryDialog
                electionEventId="event"
                electionId="post"
                busy={false}
                snapshots={[snapshot({post: []})]}
                onClose={jest.fn()}
                onGenerate={jest.fn()}
            />
        )
        expect(screen.getByText("publish.initialization.countriesError")).toBeTruthy()
        expect(
            (
                screen.getByRole("button", {
                    name: "publish.action.generateInitializationReport",
                }) as HTMLButtonElement
            ).disabled
        ).toBe(true)
        expect(useQuery).toHaveBeenCalledWith(
            expect.anything(),
            expect.objectContaining({skip: true})
        )
    } finally {
        mockPermissions.push(...saved)
    }
})
