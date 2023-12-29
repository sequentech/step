// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box} from "@mui/system"
import {isRouteErrorResponse, useRouteError} from "react-router-dom"

export function ErrorPage() {
    const error = useRouteError()

    let content = (
        <>
            <h1>Oops! Unexpected Error</h1>
            <p>Something went wrong.</p>
        </>
    )

    if (isRouteErrorResponse(error)) {
        content = (
            <>
                <h1>Oops! {error.status}</h1>
                <p>{error.statusText}</p>
                {error.data?.message && (
                    <p>
                        <i>{error.data.message}</i>
                    </p>
                )}
            </>
        )
    } else if (error instanceof Error) {
        content = (
            <>
                <h1>Oops! Unexpected Error</h1>
                <p>Something went wrong.</p>
                <p>
                    <i>{error.message}</i>
                </p>
            </>
        )
    }

    return (
        <>
            <Box
                id="error-page"
                sx={{
                    width: "100%",
                    height: "50vh",
                    display: "flex",
                    flexDirection: "column",
                    justifyContent: "center",
                    alignItems: "center",
                }}
            >
                {content}
            </Box>
        </>
    )
}
