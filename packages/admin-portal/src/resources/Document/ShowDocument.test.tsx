/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import {DocumentProperties} from "./ShowDocument"

const mockQuery = jest.fn()

jest.mock("@apollo/client", () => ({useApolloClient: () => ({query: mockQuery})}))
jest.mock("../../queries/FetchDocument", () => ({FETCH_DOCUMENT: "FetchDocument"}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("@sequentech/ui-core", () => ({downloadUrl: jest.fn(() => Promise.resolve())}))
jest.mock("react-admin", () => ({
    useRecordContext: () => ({id: "document", election_event_id: "event", name: "tally.tar.gz"}),
    TextField: () => null,
    NumberField: () => null,
    ReferenceField: () => null,
    Show: () => null,
}))
jest.mock("react-admin-json-view", () => ({JsonField: () => null}))
jest.mock("./ListDocument", () => ({ListDocument: () => null}))

/** A refusal of `/fetch-document` as Apollo hands it to the page. */
const refused = (message: string) =>
    Object.assign(new Error(message), {
        graphQLErrors: [{message, extensions: {code: "InternalServerError"}}],
    })

beforeEach(() => {
    jest.clearAllMocks()
})

it("explains that a document with decoded ballots waits for post-processing", async () => {
    mockQuery.mockRejectedValueOnce(
        refused(
            "This operation requires electoral phase PostProcessing; current phase is Execution"
        )
    )
    render(<DocumentProperties />)
    fireEvent.click(screen.getByRole("button", {name: "Download Document"}))
    expect(await screen.findByRole("alert")).toHaveProperty(
        "textContent",
        "refusals.documentWaitsForPostProcessing"
    )
})

it("shows another failure as before", async () => {
    mockQuery.mockRejectedValueOnce(refused("Document not found"))
    render(<DocumentProperties />)
    fireEvent.click(screen.getByRole("button", {name: "Download Document"}))
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "Document not found")
})
