// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Booth feasibility spike (Step 1 of the booth lift).
//
// Renders voting-portal's `StartScreen` from its production source,
// unmodified, with a synthetic election fixture seeded into the
// production Redux store via the portal's own action creators. The
// embedding strategy and rules for modifying this file are documented
// in `packages/workbench/LIFTING.md`.

import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import {Provider as ReduxProvider} from "react-redux"
import {Navigate, Route, Routes} from "react-router-dom"

// Side-effectful imports: register translations.
import "voting-portal/src/services/i18n"

import {store} from "voting-portal/src/store/store"
import StartScreen from "voting-portal/src/routes/StartScreen"
import {
    ELECTION_ID,
    EVENT_ID,
    TENANT_ID,
    seedBoothFixtures,
} from "./fixtures/boothFixtures"

const INITIAL_PATH = `/booth/tenant/${TENANT_ID}/event/${EVENT_ID}/election/${ELECTION_ID}/start`

// Seed once at module-eval time. Dispatching at module load — rather than
// from a React effect — means the fixture is in place before StartScreen's
// "redirect on missing election" effect fires.
seedBoothFixtures()

export function BoothSpike() {
    return (
        <ThemeProvider theme={theme}>
            <ReduxProvider store={store}>
                <Routes>
                    <Route
                        path="tenant/:tenantId/event/:eventId/election/:electionId/start"
                        element={<StartScreen />}
                    />
                    <Route
                        path=""
                        element={<Navigate to={INITIAL_PATH} replace />}
                    />
                    <Route
                        path="*"
                        element={
                            <pre style={{padding: "2rem"}}>
                                (redirected away from StartScreen — likely
                                a fixture mismatch; check the console)
                            </pre>
                        }
                    />
                </Routes>
            </ReduxProvider>
        </ThemeProvider>
    )
}
