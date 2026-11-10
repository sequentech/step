/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import {GenerateReport} from "./GenerateReport"
import {ETemplateType} from "@/types/templates"

const mockNotify = jest.fn()
const mockGenerate = jest.fn()
const mockAddWidget = jest.fn(() => ({identifier: "widget"}))
const mockSetWidgetTaskId = jest.fn()
const mockUpdateWidgetFail = jest.fn()

jest.mock("@apollo/client", () => ({useMutation: () => [mockGenerate]}))
jest.mock("@/queries/GenerateTemplate", () => ({GENERATE_TEMPLATE: "GenerateTemplate"}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("react-admin", () => ({useNotify: () => mockNotify}))
jest.mock("@/providers/WidgetsContextProvider", () => ({
    useWidgetStore: () => [mockAddWidget, mockSetWidgetTaskId, mockUpdateWidgetFail],
}))
jest.mock("@/resources/User/DownloadDocument", () => ({DownloadDocument: () => null}))

/** A refusal of `/generate-template` as Apollo hands it to the component. */
const refused = (message: string) =>
    Object.assign(new Error(message), {
        graphQLErrors: [{message, extensions: {code: "InternalServerError"}}],
    })

const generate = () => {
    render(
        <GenerateReport
            electionEventId="event"
            electionId="election"
            tallySessionId="tally"
            reportType={ETemplateType.BALLOT_IMAGES}
            handleClose={jest.fn()}
        />
    )
    fireEvent.click(screen.getByRole("menuitem"))
}

beforeEach(() => {
    jest.clearAllMocks()
})

it("explains that ballot images wait for post-processing", async () => {
    mockGenerate.mockRejectedValueOnce(
        refused(
            "This operation requires electoral phase PostProcessing; current phase is Evaluation"
        )
    )
    generate()
    await waitFor(() => expect(mockUpdateWidgetFail).toHaveBeenCalledWith("widget"))
    expect(mockNotify).toHaveBeenCalledWith("refusals.ballotImagesWaitForPostProcessing", {
        type: "error",
    })
})

it("marks the task as failed and says nothing more for an unreadable refusal", async () => {
    mockGenerate.mockRejectedValueOnce(refused("not a valid json response from webhook"))
    generate()
    await waitFor(() => expect(mockUpdateWidgetFail).toHaveBeenCalledWith("widget"))
    expect(mockNotify).not.toHaveBeenCalled()
})

it("follows the task of generated ballot images", async () => {
    mockGenerate.mockResolvedValueOnce({
        data: {generate_template: {document_id: "document", task_execution: {id: "task"}}},
    })
    generate()
    await waitFor(() => expect(mockSetWidgetTaskId).toHaveBeenCalledWith("widget", "task"))
    expect(mockGenerate).toHaveBeenCalledWith({
        variables: {
            tallySessionId: "tally",
            electionId: "election",
            electionEventId: "event",
            type: "BallotImages",
        },
    })
    expect(mockNotify).not.toHaveBeenCalled()
    expect(mockUpdateWidgetFail).not.toHaveBeenCalled()
})
