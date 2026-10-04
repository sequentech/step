/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {cleanup, fireEvent, render, screen, waitFor} from "@testing-library/react"
import {InitializationCountryDialog} from "./InitializationCountryDialog"
import type {ILifecycleSnapshotEntry} from "@/queries/Lifecycle"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation"),
    ...jest.requireActual("../../../../ui-core/src/types/ElectionPresentation"),
}))
jest.mock("@apollo/client", () => ({
    gql: (s: TemplateStringsArray) => s.join(""),
    useQuery: () => ({
        data: {
            sequent_backend_area: [],
            sequent_backend_area_contest: [],
            sequent_backend_ballot_style: [],
        },
        loading: false,
    }),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
afterEach(cleanup)

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
