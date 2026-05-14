// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {StrictMode} from "react"
import {createRoot} from "react-dom/client"
import {Provider as ReduxProvider} from "react-redux"
import {createBrowserRouter, Link, Outlet, RouterProvider} from "react-router-dom"
import {store} from "voting-portal/src/store/store"
import {App} from "./App"
import {BoothLayout, boothChildren} from "./BoothSpike"
import {ELECTION_ID, EVENT_ID, TENANT_ID} from "./fixtures/boothFixtures"
import {clearPersistedSnapshot} from "./persistence"

const BOOTH_START = `/tenant/${TENANT_ID}/event/${EVENT_ID}/election/${ELECTION_ID}/start`

function Shell() {
    return (
        <ReduxProvider store={store}>
            <nav
                style={{
                    padding: "0.5rem 2rem",
                    background: "#eee",
                    fontFamily: "system-ui, sans-serif",
                    display: "flex",
                    gap: "1rem",
                    alignItems: "center",
                }}
            >
                <Link to="/tally">Tally (raw JSON)</Link>
                <Link to={BOOTH_START}>Booth</Link>
                <span style={{flex: 1}} />
                <button
                    type="button"
                    onClick={() => {
                        if (
                            !confirm(
                                "Wipe the persisted workbench state and reload?"
                            )
                        ) {
                            return
                        }
                        clearPersistedSnapshot()
                        location.reload()
                    }}
                    style={{
                        padding: "0.25rem 0.6rem",
                        fontSize: "0.8rem",
                        cursor: "pointer",
                    }}
                    title="Wipe localStorage workbench:state:v1 and reload"
                >
                    Reset workbench state
                </button>
            </nav>
            <Outlet />
        </ReduxProvider>
    )
}

// Use `createBrowserRouter` (a v6 "data router") rather than the legacy
// `<BrowserRouter>` because voting-portal screens call `useSubmit` /
// `useActionData`, which only function under a data router. See
// `voting-portal/src/index.tsx` for the production setup we mirror.
const router = createBrowserRouter([
    {
        element: <Shell />,
        children: [
            {path: "/tally", element: <App />},
            {
                element: <BoothLayout />,
                children: boothChildren,
            },
        ],
    },
])

createRoot(document.getElementById("root")!).render(
    <StrictMode>
        <RouterProvider router={router} />
    </StrictMode>
)
