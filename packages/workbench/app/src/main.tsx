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
import {clearPersistedSnapshot} from "./persistence"
import {
    WorkbenchElection,
    WorkbenchEvent,
    WorkbenchHome,
    WorkbenchTenant,
} from "./Workbench"

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
                <Link to="/" style={{fontWeight: 600}}>
                    Workbench
                </Link>
                <Link to="/tally">Raw-JSON tally</Link>
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
//
// Three families of routes live under `Shell`:
//   * Workbench-native pages at `/` and `/wb/...` (Workbench.tsx). These
//     are the entry points and drilldown views.
//   * Booth (lifted voting-portal) screens at the production-mirroring
//     paths under `/tenant/:t/event/:e/...`. Mounted via `BoothLayout`
//     so the portal's own provider stack wraps them.
//   * The raw-JSON tally sandbox at `/tally`, kept as a debug page for
//     ad-hoc velvet-wasm experiments without touching Redux.
const router = createBrowserRouter([
    {
        element: <Shell />,
        children: [
            {index: true, element: <WorkbenchHome />},
            {path: "/wb/tenant/:tenantId", element: <WorkbenchTenant />},
            {
                path: "/wb/tenant/:tenantId/event/:eventId",
                element: <WorkbenchEvent />,
            },
            {
                path: "/wb/tenant/:tenantId/event/:eventId/election/:electionId",
                element: <WorkbenchElection />,
            },
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
