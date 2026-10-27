/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useRef, useState} from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import MenuAction from "./MenuActions"
const mockDirect = jest.fn(),
    mockProtected = jest.fn(),
    mockNotify = jest.fn(),
    mockReload = jest.fn()
const mockQuery = jest.fn(async () => ({data: {sequent_backend_ballot_box_seal: [{id: "seal"}]}}))
jest.mock("react-router-dom", () => ({useNavigate: () => jest.fn()}))
jest.mock("react-admin", () => ({
    useDelete: () => [mockDirect],
    useUpdate: () => [jest.fn()],
    useNotify: () => mockNotify,
    useRedirect: () => jest.fn(),
}))
jest.mock("@apollo/client", () => ({
    gql: (p: TemplateStringsArray) => p.join(""),
    useApolloClient: () => ({query: mockQuery}),
    useMutation: () => [mockProtected],
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        adminTheme: {palette: {brandColor: "navy"}},
        Dialog: ({open, children, ok, cancel, handleClose}: any) =>
            open ? (
                <div role="dialog">
                    {children}
                    {ok && <button onClick={() => handleClose(true)}>{ok}</button>}
                    <button onClick={() => handleClose(false)}>{cancel}</button>
                </div>
            ) : null,
    }),
    {virtual: true}
)
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        i18n: {dir: () => "ltr", language: "en"},
        t: (k: string) =>
            ({
                "common.label.actions": "Actions",
                "common.label.delete": "Delete",
                "common.label.cancel": "Cancel",
                "sideMenu.menuActions.remove.electionEvent": "Remove event",
                "sideMenu.menuActions.remove.election": "Remove election",
            })[k] ?? k,
    }),
}))
jest.mock("./TreeMenu", () => ({
    getNavLinkCreate: jest.fn(),
    mapAddResource: {},
    mapImportResource: {},
}))
jest.mock("../use-tree-menu-hook", () => ({
    useActionPermissions: () => ({canDeleteElectionEvent: true, canDeleteElection: true}),
    useTreeMenuData: () => ({
        data: {
            sequent_backend_election_event: [
                {id: "event", presentation: {ballot_box_seal_policy: "seal-at-close"}},
            ],
        },
        refetch: jest.fn(),
    }),
}))
jest.mock("@/components/styles/Menu", () => ({divContainer: ""}))
jest.mock("@/hooks/useSealReadRole", () => ({useSealReadRole: () => "election-event-read"}))
jest.mock("@/providers/ElectionEventTallyProvider", () => ({
    useElectionEventTallyStore: () => ({}),
}))
jest.mock("@/providers/CreateElectionEventContextProvider", () => ({
    useCreateElectionEventStore: () => ({}),
}))
jest.mock("@/providers/WidgetsContextProvider", () => ({
    useWidgetStore: () => [() => ({identifier: "task"}), jest.fn(), jest.fn()],
}))
function Fixture({election = false}: {election?: boolean}) {
    const ref = useRef<HTMLDivElement>(null)
    const [anchor, setAnchor] = useState<HTMLParagraphElement | null>(null)
    return (
        <div ref={ref}>
            <MenuAction
                isArchivedTab={false}
                resourceId={election ? "election" : "event"}
                resourceName="Fixture"
                resourceType={
                    election ? "sequent_backend_election" : "sequent_backend_election_event"
                }
                parentData={{id: "event"} as any}
                menuItemRef={ref}
                setAnchorEl={setAnchor}
                anchorEl={anchor}
                reloadTree={mockReload}
            />
        </div>
    )
}
async function remove(election = false) {
    render(<Fixture election={election} />)
    fireEvent.click(screen.getByRole("button", {name: "Actions: Fixture"}))
    fireEvent.click(
        await screen.findByRole("menuitem", {name: election ? "Remove election" : "Remove event"})
    )
    await screen.findByRole("dialog")
}
beforeEach(() => {
    jest.clearAllMocks()
    mockProtected.mockResolvedValue({
        data: {
            delete_election_event: {
                error_msg:
                    "Export the complete, current election execution data before requesting deletion",
            },
        },
    })
})
test("sealed whole events use the protected action and display its prerequisite refusal", async () => {
    await remove()
    fireEvent.click(await screen.findByRole("button", {name: "Delete"}))
    await waitFor(() =>
        expect(mockProtected).toHaveBeenCalledWith({variables: {electionEventId: "event"}})
    )
    expect(mockDirect).not.toHaveBeenCalled()
    expect(mockNotify).toHaveBeenCalledWith(expect.stringContaining("Export the complete"), {
        type: "error",
    })
    expect(mockReload).not.toHaveBeenCalled()
})
test("canceling whole-event deletion sends no mutation", async () => {
    await remove()
    fireEvent.click(screen.getByRole("button", {name: "Cancel"}))
    expect(mockProtected).not.toHaveBeenCalled()
    expect(mockDirect).not.toHaveBeenCalled()
})
test("individual sealed elections remain undeletable", async () => {
    await remove(true)
    expect(screen.queryByRole("button", {name: "Delete"})).toBeNull()
    expect(mockQuery).toHaveBeenCalled()
    expect(mockProtected).not.toHaveBeenCalled()
    expect(mockDirect).not.toHaveBeenCalled()
})
