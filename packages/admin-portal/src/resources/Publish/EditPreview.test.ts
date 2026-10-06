/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import {EditPreview} from "./EditPreview"

const mockPrepare = jest.fn()
const mockQuery = jest.fn()
const mockClient = {query: mockQuery}
const mockNotify = jest.fn()
const mockAddWidget = jest.fn(() => ({identifier: "widget"}))
const mockSetTask = jest.fn()
const mockFailWidget = jest.fn()
const mockT = (key: string) => key
let mockTask: any
jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
    useApolloClient: () => mockClient,
    useMutation: () => [mockPrepare],
    useQuery: (_query: any, options: any) => ({
        data: options.skip
            ? undefined
            : {
                  sequent_backend_tasks_execution: mockTask ? [mockTask] : [],
              },
    }),
}))
jest.mock("react-admin", () => ({
    useNotify: () => mockNotify,
    SimpleForm: ({children, onSubmit}: any) =>
        require("react").createElement(
            "form",
            {
                onSubmit: (event: any) => {
                    event.preventDefault()
                    onSubmit()
                },
            },
            children
        ),
    Toolbar: ({children}: any) => require("react").createElement("div", null, children),
    SaveButton: ({disabled, label}: any) =>
        require("react").createElement("button", {type: "submit", disabled}, label),
    Button: ({disabled, label, onClick}: any) =>
        require("react").createElement("button", {type: "button", disabled, onClick}, label),
    AutocompleteInput: ({choices, onChange}: any) =>
        require("react").createElement(
            "select",
            {"aria-label": "area", "onChange": (event: any) => onChange(event.target.value)},
            require("react").createElement("option", {value: ""}, "Choose area"),
            ...choices.map((area: any) =>
                require("react").createElement("option", {key: area.id, value: area.id}, area.name)
            )
        ),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: mockT})}))
jest.mock("@/providers/TenantContextProvider", () => ({
    TenantContext: require("react").createContext({tenantId: "tenant"}),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {VOTING_PORTAL_URL: "https://voting.example", QUERY_POLL_INTERVAL_MS: 100},
    }),
}))
jest.mock("@/providers/WidgetsContextProvider", () => ({
    useWidgetStore: () => [mockAddWidget, mockSetTask, mockFailWidget],
}))
jest.mock("@sequentech/ui-core", () => ({
    ETaskExecutionStatus: {SUCCESS: "SUCCESS", FAILED: "FAILED"},
}))
const props = {publicationId: "publication", electionEventId: "event"}
beforeEach(() => {
    jest.clearAllMocks()
    mockTask = undefined
    mockQuery.mockImplementation(({query}: {query: string}) =>
        Promise.resolve({
            data: query.includes("PublicationPreviewAreas(")
                ? {
                      sequent_backend_ballot_style: Array.from({length: 60}, (_, i) => ({
                          area_id: `area-${i}`,
                      })),
                  }
                : {
                      sequent_backend_area: Array.from({length: 60}, (_, i) => ({
                          id: `area-${i}`,
                          name: `Area ${i}`,
                      })),
                  },
        })
    )
    mockPrepare.mockResolvedValue({
        data: {
            prepare_ballot_publication_preview: {
                document_id: "document",
                task_execution: {id: "task"},
            },
        },
    })
    jest.spyOn(window, "open").mockImplementation(() => null)
})
afterEach(() => jest.restoreAllMocks())
async function startPreview() {
    await screen.findByText("Area 59")
    fireEvent.change(screen.getByLabelText("area"), {target: {value: "area-59"}})
    await act(async () => fireEvent.click(screen.getByText("publish.preview.action")))
}
it("offers areas beyond the diff limit and opens only after task success", async () => {
    const view = render(require("react").createElement(EditPreview, props))
    await startPreview()
    expect(window.open).not.toHaveBeenCalled()
    mockTask = {id: "task", execution_status: "SUCCESS"}
    view.rerender(require("react").createElement(EditPreview, {...props, close: jest.fn()}))
    await waitFor(() =>
        expect(window.open).toHaveBeenCalledWith(
            "https://voting.example/preview/tenant/document/area-59/publication",
            "_blank"
        )
    )
    expect(mockQuery.mock.calls[0][0].variables.publicationId).toBe("publication")
})
it("recovers from enqueue errors even when the response contains a document id", async () => {
    mockPrepare.mockResolvedValueOnce({
        data: {
            prepare_ballot_publication_preview: {
                document_id: "document",
                error_msg: "Queue unavailable",
                task_execution: {id: "task"},
            },
        },
    })
    render(require("react").createElement(EditPreview, props))
    await startPreview()
    expect(screen.queryByRole("progressbar")).toBeNull()
    expect(mockFailWidget).toHaveBeenCalled()
    expect(window.open).not.toHaveBeenCalled()
    await act(async () => fireEvent.click(screen.getByText("publish.preview.action")))
    expect(mockPrepare).toHaveBeenCalledTimes(2)
})
it("recovers from a failed worker without opening the unready document", async () => {
    const view = render(require("react").createElement(EditPreview, props))
    await startPreview()
    mockTask = {id: "task", execution_status: "FAILED"}
    view.rerender(require("react").createElement(EditPreview, {...props, close: jest.fn()}))
    await screen.findByText("publish.preview.action")
    expect(screen.queryByRole("progressbar")).toBeNull()
    expect(window.open).not.toHaveBeenCalled()
    await act(async () => fireEvent.click(screen.getByText("publish.preview.action")))
    expect(mockPrepare).toHaveBeenCalledTimes(2)
})
