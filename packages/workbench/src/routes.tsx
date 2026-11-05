// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {Navigate, useParams, type RouteObject} from "react-router"
import {Alert, AlertTitle, Box, Link} from "@mui/material"
import {isScenarioId, SCENARIOS} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {tenantEventRoutes} from "voting-portal/src/appRoutes"
import {ErrorPage} from "voting-portal/src/routes/ErrorPage"
import TenantEvent from "voting-portal/src/routes/TenantEvent"
import {
    isPreviewScreen,
    PREVIEW_SCREENS,
    PreviewScreen,
    previewDeepLink,
} from "voting-portal/src/preview/screens"
import {WorkbenchLayout} from "./components/WorkbenchLayout"
import {DEFAULT_SCENARIO, screenPath, useWorkbench, useWorkbenchActions} from "./state"

const OpenSession: React.FC = () => {
    const {state} = useWorkbench()
    return <Navigate replace to={screenPath(state.session.snapshot, state.session.screen)} />
}

/** `#/scenario/<scenarioId>/<screen>`: loads the bundled scenario and opens the screen. */
const ScenarioEntry: React.FC = () => {
    const {scenarioId, screen = PreviewScreen.START} = useParams()
    const {openScenario} = useWorkbenchActions()
    const valid = isScenarioId(scenarioId) && isPreviewScreen(screen)

    useEffect(() => {
        if (isScenarioId(scenarioId) && isPreviewScreen(screen))
            openScenario(scenarioId, screen, true)
        // Only the link opens a session; later state changes must not reopen it.
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [scenarioId, screen])

    return valid ? null : <UnknownLink />
}

const UnknownLink: React.FC = () => (
    <Box sx={{p: 2}}>
        <Alert severity="warning">
            <AlertTitle>This workbench link names no scenario screen</AlertTitle>
            Scenarios: {SCENARIOS.map(({id}) => id).join(", ")}. Screens:{" "}
            {PREVIEW_SCREENS.join(", ")}. For example,{" "}
            <Link href={`#${previewDeepLink(DEFAULT_SCENARIO, PreviewScreen.VOTE)}`}>
                #{previewDeepLink(DEFAULT_SCENARIO, PreviewScreen.VOTE)}
            </Link>
            .
        </Alert>
    </Box>
)

/**
 * Links resolve before the workbench layout mounts: the layout reloads the voter session,
 * which would remount a link route below it and open its scenario again. The production
 * routes of an election event keep their paths and actions below the workbench controls.
 */
export const routes: RouteObject[] = [
    {index: true, element: <OpenSession />},
    {path: "scenario/:scenarioId/:screen?", element: <ScenarioEntry />},
    {
        element: <WorkbenchLayout />,
        children: [
            {
                path: "tenant/:tenantId/event/:eventId",
                element: <TenantEvent />,
                errorElement: <ErrorPage />,
                children: tenantEventRoutes,
            },
            {path: "*", element: <UnknownLink />},
        ],
    },
]
