// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useEffect, useMemo, useRef, useState} from "react"
import {createMemoryRouter, Outlet, RouterProvider, type RouteObject} from "react-router-dom"
import {Alert, AlertTitle, Box, Typography} from "@mui/material"
import {tenantEventRoutes} from "../appRoutes"
import {PortalChrome} from "../components/PortalChrome"
import {ErrorPage} from "../routes/ErrorPage"
import TenantEvent from "../routes/TenantEvent"
import {
    embeddedSource,
    embedMessage,
    EmbedMessageError,
    EmbedMessageType,
    readShowMessage,
    type EmbedReply,
    type ShowRequest,
} from "./embed"
import {EVENT_ROUTE, previewScreenAt, previewSessionPath} from "./screens"
import type {PreviewSession} from "./session"
import {VoterPreview} from "./VoterPreview"

/** The event's screens inside the portal's header, footer and stylesheet, as in production. */
const PreviewChrome: React.FC = () => (
    <PortalChrome>
        <Outlet />
    </PortalChrome>
)

export const embeddedRoutes: RouteObject[] = [
    {
        element: <PreviewChrome />,
        errorElement: <ErrorPage />,
        children: [
            {
                path: EVENT_ROUTE,
                element: <TenantEvent />,
                errorElement: <ErrorPage />,
                children: tenantEventRoutes,
            },
        ],
    },
]

type Reply = (message: EmbedReply) => void

/** The production screens of a session, reporting every location the voter reaches. */
const EmbeddedScreens: React.FC<{session: PreviewSession; reply: Reply}> = ({session, reply}) => {
    const router = useMemo(
        () =>
            createMemoryRouter(embeddedRoutes, {
                initialEntries: [previewSessionPath(session.snapshot, session.screen)],
            }),
        [session]
    )
    const reported = useRef<string | undefined>(undefined)

    useEffect(() => {
        const report = (path: string) => {
            if (path === reported.current) return
            reported.current = path
            reply({type: EmbedMessageType.SHOWN, screen: previewScreenAt(path), path})
        }
        report(router.state.location.pathname)
        const unsubscribe = router.subscribe(({location}) => report(location.pathname))
        return () => {
            unsubscribe()
            reported.current = undefined
        }
    }, [router, reply])

    return <RouterProvider router={router} />
}

interface Shown {
    /** Counts requests, so each one mounts its own router. */
    sequence: number
    request: ShowRequest
    session: PreviewSession
    /** Replies go to the origin of the window that sent the request, and nowhere else. */
    origin: string
}

const reasonOf = (error: unknown) =>
    error instanceof EmbedMessageError
        ? error.issues
        : [error instanceof Error ? error.message : String(error)]

export interface EmbeddedPreviewProps {
    /** The window allowed to send documents; the parent of the frame by default. */
    host?: Window
}

/**
 * The voter preview for another tool's document. The framing window sends a publication
 * preview document with `show` (see `embed.ts`), and the embed renders the portal's
 * production screens for it, as the publication preview does, inside the portal chrome.
 */
export const EmbeddedPreview: React.FC<EmbeddedPreviewProps> = ({host = window.parent}) => {
    const [shown, setShown] = useState<Shown>()
    const [issues, setIssues] = useState<string[]>()

    useEffect(() => {
        try {
            // The publication preview's demo mode: a closed election can still be voted.
            window.sessionStorage.setItem("isDemo", "true")
        } catch {
            // Without storage the portal treats the election's own status as final.
        }
        const onMessage = (event: MessageEvent) => {
            if (event.source !== host) return
            try {
                const request = readShowMessage(event.data)
                if (!request) return
                setIssues(undefined)
                setShown((previous) => ({
                    sequence: (previous?.sequence ?? 0) + 1,
                    request,
                    origin: event.origin,
                    session: {snapshot: embeddedSource(request), screen: request.screen},
                }))
            } catch (error) {
                const reasons = reasonOf(error)
                setIssues(reasons)
                host.postMessage(
                    embedMessage({type: EmbedMessageType.FAILED, issues: reasons}),
                    event.origin || "*"
                )
            }
        }
        window.addEventListener("message", onMessage)
        host.postMessage(embedMessage({type: EmbedMessageType.READY}), "*")
        return () => window.removeEventListener("message", onMessage)
    }, [host])

    const origin = shown?.origin
    const reply = useCallback<Reply>(
        (message) => {
            if (origin !== undefined) host.postMessage(embedMessage(message), origin || "*")
        },
        [host, origin]
    )
    const onLoadError = useCallback(
        (error: unknown) => reply({type: EmbedMessageType.FAILED, issues: reasonOf(error)}),
        [reply]
    )

    if (issues)
        return (
            <Alert severity="error" sx={{m: 2}}>
                <AlertTitle>The voter preview could not open this request</AlertTitle>
                <Box component="ul" sx={{m: 0, pl: 2}}>
                    {issues.map((issue) => (
                        <li key={issue}>{issue}</li>
                    ))}
                </Box>
            </Alert>
        )
    if (!shown)
        return (
            <Typography role="status" sx={{m: 2}} color="text.secondary">
                Waiting for a ballot to preview
            </Typography>
        )
    return (
        <VoterPreview
            session={shown.session}
            language={shown.request.language}
            onLoadError={onLoadError}
        >
            <EmbeddedScreens key={shown.sequence} session={shown.session} reply={reply} />
        </VoterPreview>
    )
}
