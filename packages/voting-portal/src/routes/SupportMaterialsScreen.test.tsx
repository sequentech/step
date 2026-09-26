// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, waitFor} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import {ThemeProvider} from "@mui/material/styles"
import {ESupportMaterialsPolicy, EElectionEventDelegatedVotingPolicy} from "@sequentech/ui-core"
import theme from "../../../ui-essentials/src/services/theme"
import {store, type RootState} from "../store/store"
import {ELECTION_WITH_INVALID} from "../fixtures/election"
import SupportMaterialsScreen from "./SupportMaterialsScreen"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        PageLimit: ({children}: {children: React.ReactNode}) => <main>{children}</main>,
        theme: jest.requireActual("../../../ui-essentials/src/services/theme").default,
    }),
    {virtual: true}
)
jest.mock("../store/hooks", () => ({
    useAppSelector: (selector: (state: RootState) => unknown) => selector(mockState),
    useAppDispatch: () => jest.fn(),
}))
jest.mock("../components/Stepper", () => ({__esModule: true, default: () => null}))
jest.mock("../components/SupportMaterial/SupportMaterial", () => ({
    SupportMaterial: ({title, onViewed}: {title: string; onViewed: () => void}) => (
        <button onClick={onViewed}>Read {title}</button>
    ),
}))
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: jest
        .requireActual<typeof React>("react")
        .createContext({globalSettings: {DISABLE_AUTH: false}}),
}))
jest.mock("@apollo/client/react", () => ({
    useQuery: () => ({data: undefined}),
    useMutation: () => [mockAcknowledge],
}))

let mockState: RootState
const mockAcknowledge = jest.fn()
beforeEach(() => {
    mockAcknowledge
        .mockReset()
        .mockResolvedValue({data: {acknowledge_support_materials: {document_ids: ["document-1"]}}})
    mockState = {
        ...store.getState(),
        electionEvent: {
            "event-1": {
                id: "event-1",
                presentation: {
                    delegated_voting_policy: EElectionEventDelegatedVotingPolicy.DISABLED,
                    materials: {policy: ESupportMaterialsPolicy.MANDATORY_FOR_VOTING},
                },
            },
        },
        supportMaterials: {
            "material-1": {
                id: "material-1",
                tenant_id: "tenant-1",
                election_event_id: "event-1",
                document_id: "document-1",
            },
        },
    }
})

function renderMaterials() {
    const router = createMemoryRouter(
        [
            {
                path: "/tenant/:tenantId/event/:eventId/materials",
                element: <SupportMaterialsScreen />,
            },
            {
                path: "/tenant/:tenantId/event/:eventId/election-chooser",
                element: <h1>Choose election</h1>,
            },
        ],
        {initialEntries: ["/tenant/tenant-1/event/event-1/materials?lang=en"]}
    )
    render(
        <ThemeProvider theme={theme}>
            <RouterProvider router={router} />
        </ThemeProvider>
    )
    return router
}

it.each([false, true])(
    "requires viewing and explicit acknowledgement with a loaded ballot style: %s",
    async (hasStyle) => {
        if (hasStyle) {
            // A loaded style remains authoritative even when the event policy differs.
            mockState.electionEvent["event-1"]!.presentation!.materials = {
                policy: ESupportMaterialsPolicy.OPTIONAL,
            }
            mockState.ballotStyles = {
                "election-1": {
                    id: "style-1",
                    tenant_id: "tenant-1",
                    election_event_id: "event-1",
                    election_id: "election-1",
                    created_at: "",
                    last_updated_at: "",
                    ballot_eml: {
                        ...ELECTION_WITH_INVALID,
                        election_event_presentation: {
                            delegated_voting_policy: EElectionEventDelegatedVotingPolicy.DISABLED,
                            materials: {policy: ESupportMaterialsPolicy.MANDATORY_FOR_VOTING},
                        },
                    },
                },
            }
        }
        const user = userEvent.setup()
        const router = renderMaterials()
        const checkbox = await screen.findByRole("checkbox", {
            name: "materials.mandatory.checkboxLabel",
        })
        expect(checkbox).toBeDisabled()
        const proceed = screen.getByRole("button", {name: "materials.mandatory.continueButton"})
        expect(proceed).toBeDisabled()
        await user.click(screen.getByRole("button", {name: "Read"}))
        expect(checkbox).toBeEnabled()
        expect(proceed).toBeDisabled()
        await user.click(checkbox)
        await user.click(proceed)
        await waitFor(() =>
            expect(router.state.location.pathname).toBe(
                "/tenant/tenant-1/event/event-1/election-chooser"
            )
        )
        expect(mockAcknowledge).toHaveBeenCalledWith({
            variables: {electionEventId: "event-1", documentIds: ["document-1"]},
        })
    }
)

it("optional published materials do not require acknowledgement", async () => {
    mockState.electionEvent["event-1"]!.presentation!.materials = {
        policy: ESupportMaterialsPolicy.OPTIONAL,
    }
    renderMaterials()
    expect(await screen.findByRole("button", {name: "Read"})).toBeVisible()
    expect(screen.queryByRole("checkbox")).toBeNull()
    expect(mockAcknowledge).not.toHaveBeenCalled()
})
