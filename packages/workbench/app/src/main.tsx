// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {StrictMode} from "react"
import {createRoot} from "react-dom/client"
import {BrowserRouter, Link, Route, Routes} from "react-router-dom"
import {App} from "./App"
import {BoothSpike} from "./BoothSpike"

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
                <Link to="/">Tally (raw JSON)</Link>
                <Link to="/booth">Booth spike</Link>
            </nav>
            <Routes>
                <Route path="/" element={<App />} />
                <Route path="/booth/*" element={<BoothSpike />} />
            </Routes>
        </>
    )
}

createRoot(document.getElementById("root")!).render(
    <StrictMode>
        <BrowserRouter>
            <Shell />
        </BrowserRouter>
    </StrictMode>
)
