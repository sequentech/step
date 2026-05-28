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
import {BallotPipeline} from "./BallotPipeline"
import {TallyPage} from "./TallyPage"
import {BoothLayout, boothChildren} from "./BoothSpike"
import {
    BallotStyleDetailPage,
    DiagnosticsPage,
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
                    background: "#1e1e1e",
                    fontFamily: "system-ui, sans-serif",
                    display: "flex",
                    gap: "1rem",
                    alignItems: "center",
                    borderBottom: "1px solid #3a3a3a",
                }}
            >
                <Link to="/wb">
                    Snapshots
                </Link>
                <Link to="/pipeline">Ballot pipeline</Link>
                <Link to="/tally">Tally</Link>
                {/* Right-aligned utility link: build provenance +
                 *  live-state dump are not part of the normal
                 *  scenario-exploration flow, so we push them to the
                 *  corner. */}
                <Link
                    to="/diagnostics"
                    style={{
                        marginLeft: "auto",
                        color: "#999",
                        fontSize: "0.85rem",
                    }}
                >
                    Diagnostics
                </Link>
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
//   * The ballot pipeline sandbox at `/pipeline` (BallotPipeline.tsx),
//     a per-stage encode/encrypt/decrypt/decode/tally playground that
//     also doubles as a round-trip oracle when seeded from
//     `ContestDetailPage`'s "Open in ballot pipeline" button.
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
            // `/pipeline` reuses `InspectorLayout` so the workbench rail
            // is always available — landing on the pipeline from a
            // contest's "Open in ballot pipeline" button used to strand
            // the operator with no nav until they manually clicked back
            // to `/wb`.
            {
                element: <InspectorLayout />,
                children: [
                    {path: "/pipeline", element: <BallotPipeline />},
                    // `/tally` is the standalone tally sandbox — sibling
                    // of `/pipeline`. Mounted under the same layout for
                    // the same nav-availability reason. Seeded via
                    // react-router location state by "Open in tally"
                    // buttons on the contest page and the pipeline's
                    // tally section; lands on velvet-wasm fixtures on a
                    // bare reload.
                    {path: "/tally", element: <TallyPage />},
                    // `/diagnostics` is a low-prominence utility page
                    // that hosts the build-provenance card (previously
                    // embedded on the snapshots overview) alongside a
                    // live-state JSON dump. Reuses `InspectorLayout`
                    // so the operator can still see and click into
                    // snapshots from the rail.
                    {path: "/diagnostics", element: <DiagnosticsPage />},
                ],
            },
            {
                element: <BoothLayout />,
                children: boothChildren,
            },
        ],
    },
])

// Dark theme: set global defaults on the document body. The booth
// routes reset these via BoothLayout's wrapper div (see BoothSpike.tsx).
document.body.style.backgroundColor = "#1e1e1e"
document.body.style.color = "#e0e0e0"

createRoot(document.getElementById("root")!).render(
    <StrictMode>
        <RouterProvider router={router} />
    </StrictMode>
)
