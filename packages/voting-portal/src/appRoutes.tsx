// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {Suspense} from "react"
import type {RouteObject} from "react-router-dom"
import {Loader} from "@sequentech/ui-essentials"
import App from "./App"
import {ErrorPage} from "./routes/ErrorPage"
import {VotingPortalError, VotingPortalErrorType} from "./services/VotingPortalError"
import {action as votingAction} from "./routes/VotingScreen"
import {action as castBallotAction} from "./routes/ReviewScreen"
import TenantEvent from "./routes/TenantEvent"
import PublishedBallot from "./routes/PublishedBallot"
import PreviewPublicationEvent from "./routes/PreviewPublicationEvent"
import ElectionSelectionScreen from "./routes/ElectionSelectionScreen"
import LoginScreen from "./routes/LoginScreen"
import RegisterScreen from "./routes/RegisterScreen"
import StartScreen from "./routes/StartScreen"
import VotingScreen from "./routes/VotingScreen"
import ReviewScreen from "./routes/ReviewScreen"
import ConfirmationScreen from "./routes/ConfirmationScreen"
import AuditScreen from "./routes/AuditScreen"
import BallotLocator from "./routes/BallotLocator"
import SupportMaterialsScreen from "./routes/SupportMaterialsScreen"

export const ThrowCertAuthError = (): React.ReactElement => {
    throw new VotingPortalError(VotingPortalErrorType.CERT_AUTH_FAILED)
}

/** Children of `/tenant/:tenantId/event/:eventId/election/:electionId`. */
export const electionRoutes: RouteObject[] = [
    {
        path: "start",
        element: (
            <Suspense fallback={<Loader />}>
                <StartScreen />
            </Suspense>
        ),
    },
    {
        path: "vote",
        element: (
            <Suspense fallback={<Loader />}>
                <VotingScreen />
            </Suspense>
        ),
        action: votingAction,
    },
    {
        path: "review",
        element: (
            <Suspense fallback={<Loader />}>
                <ReviewScreen />
            </Suspense>
        ),
        action: castBallotAction,
    },
    {
        path: "confirmation",
        element: (
            <Suspense fallback={<Loader />}>
                <ConfirmationScreen />
            </Suspense>
        ),
    },
    {
        path: "audit",
        element: (
            <Suspense fallback={<Loader />}>
                <AuditScreen />
            </Suspense>
        ),
    },
    {
        path: "ballot-locator/:ballotId?",
        element: (
            <Suspense fallback={<Loader />}>
                <BallotLocator />
            </Suspense>
        ),
    },
]

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
    {
        path: "election/:electionId",
        element: <PublishedBallot />,
        children: electionRoutes,
    },
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
