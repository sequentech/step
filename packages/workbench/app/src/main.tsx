// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {StrictMode} from "react"
import {createRoot} from "react-dom/client"
import {createBrowserRouter, Link, Outlet, RouterProvider} from "react-router-dom"
import {App} from "./App"
import {BoothLayout, boothChildren} from "./BoothSpike"
import {ELECTION_ID, EVENT_ID, TENANT_ID} from "./fixtures/boothFixtures"

const BOOTH_START = `/tenant/${TENANT_ID}/event/${EVENT_ID}/election/${ELECTION_ID}/start`

function Shell() {
    return (
        <>
            <nav
                style={{
                    padding: "0.5rem 2rem",
                    background: "#eee",
                    fontFamily: "system-ui, sans-serif",
                    display: "flex",
                    gap: "1rem",
                }}
            >
                <Link to="/tally">Tally (raw JSON)</Link>
                <Link to={BOOTH_START}>Booth</Link>
            </nav>
            <Outlet />
        </>
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
