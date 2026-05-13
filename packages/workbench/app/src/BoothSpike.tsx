// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Booth lift: mounts voting-portal route components from production
// source, unmodified, at the same paths the portal itself uses, with
// route-level `action`s wired exactly as in `voting-portal/src/index.tsx`.
// Data is supplied by `fixtures/boothFixtures.ts` via the portal's own
// action creators. See `packages/workbench/LIFTING.md` for the procedure.

import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import {Provider as ReduxProvider} from "react-redux"
import type {RouteObject} from "react-router-dom"
import {Outlet} from "react-router-dom"

// Side-effectful imports: register translations.
import "voting-portal/src/services/i18n"

import {store} from "voting-portal/src/store/store"
import {WasmWrapper} from "voting-portal/src/providers/WasmWrapper"
import StartScreen from "voting-portal/src/routes/StartScreen"
import VotingScreen, {
    action as votingAction,
} from "voting-portal/src/routes/VotingScreen"
import ReviewScreen, {
    action as castBallotAction,
} from "voting-portal/src/routes/ReviewScreen"
import ConfirmationScreen from "voting-portal/src/routes/ConfirmationScreen"
import {seedBoothFixtures} from "./fixtures/boothFixtures"

// Seed once at module-eval time so the fixture is in place before any
// selector fires (StartScreen redirects on a missing election).
seedBoothFixtures()

// Workbench-only debug: expose the production store on `window.__store`
// so we can inspect Redux state from the browser console / Playwright
// while lifting screens. This is a workbench affordance only and lives
// outside `voting-portal/src/`.
if (typeof window !== "undefined") {
    ;(window as unknown as {__store: typeof store}).__store = store
    const w = window as unknown as {__dispatchLog: unknown[]}
    w.__dispatchLog = []
    // Patch dispatch to log every action; helps spot whether a click
    // actually reached the reducer or not.
    const origDispatch = store.dispatch.bind(store)
    ;(store as unknown as {dispatch: typeof origDispatch}).dispatch = ((
        action: Parameters<typeof origDispatch>[0]
    ) => {
        w.__dispatchLog.push({type: (action as {type?: string}).type, action})
        return origDispatch(action)
    }) as typeof origDispatch
}

/**
 * Layout for every booth screen. Provides the MUI theme, the production
 * Redux store (already populated by `seedBoothFixtures()`), and the
 * portal's `WasmWrapper` (which initializes the sequent-core wasm module
 * before rendering children). Designed to be mounted as a layout route
 * under a data router so the portal's `useSubmit` / `useActionData` calls
 * work.
 */
export function BoothLayout() {
    return (
        <ThemeProvider theme={theme}>
            <ReduxProvider store={store}>
                <WasmWrapper>
                    <Outlet />
                </WasmWrapper>
            </ReduxProvider>
        </ThemeProvider>
    )
}

/**
 * Route children mirroring the portal's own route tree (see
 * `voting-portal/src/index.tsx`) at the same paths so its absolute
 * `<Link to="/tenant/.../vote">` navigations and `useSubmit` calls
 * resolve without any rewrite.
 *
 * As we lift more screens, add them here paired with their `action`
 * (when the portal wires one). Keep this list aligned with the portal's
 * `election/:electionId` subtree.
 */
export const boothChildren: RouteObject[] = [
    {
        path: "tenant/:tenantId/event/:eventId/election/:electionId",
        children: [
            {path: "start", element: <StartScreen />},
            {path: "vote", element: <VotingScreen />, action: votingAction},
            {
                path: "review",
                element: <ReviewScreen />,
                action: castBallotAction,
            },
            {path: "confirmation", element: <ConfirmationScreen />},
        ],
    },
]
