// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {StrictMode} from "react"
import {createRoot} from "react-dom/client"
import {Provider as ReduxProvider} from "react-redux"
import {
    createBrowserRouter,
    Link,
    Navigate,
    Outlet,
    RouterProvider,
} from "react-router-dom"
import {store} from "voting-portal/src/store/store"
import {App} from "./App"
import {BoothLayout, boothChildren} from "./BoothSpike"
import {
    BallotStyleDetailPage,
    ContestDetailPage,
    InspectorLayout,
    SnapshotDetailPage,
    SnapshotOverviewPage,
    VoterDetailPage,
} from "./WorkbenchInspector"

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
                <Link to="/wb" style={{fontWeight: 600}}>
                    Workbench
                </Link>
                <Link to="/tally">Raw-JSON tally</Link>
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
//   * The workbench inspector at `/wb/...` (WorkbenchInspector.tsx).
//     `/` redirects here — there is no separate landing page.
//   * Booth (lifted voting-portal) screens at the production-mirroring
//     paths under `/tenant/:t/event/:e/...`. Mounted via `BoothLayout`
//     so the portal's own provider stack wraps them.
//   * The raw-JSON tally sandbox at `/tally`, kept as a debug page for
//     ad-hoc velvet-wasm experiments without touching Redux.
const router = createBrowserRouter([
    {
        element: <Shell />,
        children: [
            {index: true, element: <Navigate to="/wb" replace />},
            {
                path: "/wb",
                element: <InspectorLayout />,
                children: [
                    {index: true, element: <SnapshotOverviewPage />},
                    {path: "snapshot/:id", element: <SnapshotDetailPage />},
                    {
                        path: "ballot-style/:id",
                        element: <BallotStyleDetailPage />,
                    },
                    {path: "contest/:id", element: <ContestDetailPage />},
                    {path: "voter/:id", element: <VoterDetailPage />},
                ],
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
