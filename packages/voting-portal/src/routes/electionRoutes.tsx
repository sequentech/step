// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {Suspense} from "react"
import type {RouteObject} from "react-router-dom"
import {Loader} from "@sequentech/ui-essentials"
import PublishedBallot from "./PublishedBallot"
import StartScreen from "./StartScreen"
import VotingScreen, {action as votingAction} from "./VotingScreen"
import ReviewScreen, {action as castBallotAction} from "./ReviewScreen"
import ConfirmationScreen from "./ConfirmationScreen"
import AuditScreen from "./AuditScreen"
import BallotLocator from "./BallotLocator"

export const electionRoutes: RouteObject = {
    path: "election/:electionId",
    children: [
        {
            element: <PublishedBallot />,
            children: [
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
            ],
        },
        // A stored vote remains independently readable when no active ballot
        // publication is available. This route keeps the normal authentication.
        {
            path: "ballot-locator/:ballotId?",
            element: (
                <Suspense fallback={<Loader />}>
                    <BallotLocator />
                </Suspense>
            ),
        },
    ],
}
