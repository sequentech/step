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
    BuildInfoPage,
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
                    Snapshots
                </Link>
                <Link to="/pipeline">Ballot pipeline</Link>
                <Link to="/tally">Tally</Link>
                {/* Right-aligned utility link: build provenance is not
                 *  part of the normal scenario-exploration flow, so we
                 *  push it to the corner. */}
                <Link
                    to="/build"
                    style={{
                        marginLeft: "auto",
                        color: "#666",
                        fontSize: "0.85rem",
                    }}
                >
                    Build info
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
                    // `/build` is a low-prominence utility page that
                    // hosts the build-provenance card previously
                    // embedded on the snapshots overview. Reuses
                    // `InspectorLayout` so the operator can still see
                    // and click into snapshots from the rail.
                    {path: "/build", element: <BuildInfoPage />},
                ],
            },
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
