// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {Suspense} from "react"
import type {RouteObject} from "react-router-dom"
import {Loader} from "@sequentech/ui-essentials"
import App from "./App"
import {ErrorPage} from "./routes/ErrorPage"
import {VotingPortalError, VotingPortalErrorType} from "./services/VotingPortalError"
import TenantEvent from "./routes/TenantEvent"
import {electionRoutes} from "./routes/electionRoutes"
import PreviewPublicationEvent from "./routes/PreviewPublicationEvent"
import PreviewFromFile from "./routes/PreviewFromFile"
import ElectionSelectionScreen from "./routes/ElectionSelectionScreen"
import LoginScreen from "./routes/LoginScreen"
import RegisterScreen from "./routes/RegisterScreen"
import SupportMaterialsScreen from "./routes/SupportMaterialsScreen"

export const ThrowCertAuthError = (): React.ReactElement => {
    throw new VotingPortalError(VotingPortalErrorType.CERT_AUTH_FAILED)
}

/** Children of `/tenant/:tenantId/event/:eventId`. */
export const tenantEventRoutes: RouteObject[] = [
    {
        path: "election-chooser",
        element: (
            <Suspense fallback={<Loader />}>
                <ElectionSelectionScreen />
            </Suspense>
        ),
    },
    {
        path: "login",
        element: (
            <Suspense fallback={<Loader />}>
                <LoginScreen />
            </Suspense>
        ),
    },
    {
        path: "enroll",
        element: (
            <Suspense fallback={<Loader />}>
                <RegisterScreen />
            </Suspense>
        ),
    },
    electionRoutes,
    {
        path: "materials",
        element: (
            <Suspense fallback={<Loader />}>
                <SupportMaterialsScreen />
            </Suspense>
        ),
    },
]

export const appRoutes: RouteObject[] = [
    {
        path: "/cert-auth-error",
        element: <ThrowCertAuthError />,
        errorElement: <ErrorPage />,
    },
    {
        path: "/",
        element: <App />,
        errorElement: <ErrorPage />,
        children: [
            {
                // Before the path-parameter route, so `file` is not read as
                // a tenant id.
                path: "/preview/file",
                element: <PreviewFromFile />,
            },
            {
                path: "/preview/:tenantId/:documentId/:areaId/:publicationId",
                element: <PreviewPublicationEvent />,
            },
            {
                path: "/tenant/:tenantId/event/:eventId",
                element: (
                    <Suspense fallback={<Loader />}>
                        <TenantEvent />
                    </Suspense>
                ),
                children: tenantEventRoutes,
            },
        ],
    },
]
