// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Button, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {HistoricalImportConverterApi} from "@/services/HistoricalImportConverter"
import {createHistoricalImportConverter} from "@/services/HistoricalImportWorkerClient"
import {HistoricalSourcePanel} from "./HistoricalSourcePanel"

interface Props {
    baseUrl?: string
    disabled: boolean
    droppedFiles?: File[]
    onPrepare: () => void
    onAccepted: (file: File, hash: string) => Promise<void>
}
export const HistoricalImportPlugin: React.FC<Props> = ({baseUrl, ...panelProps}) => {
    const {t} = useTranslation()
    const [attempt, setAttempt] = useState(0)
    const [loaded, setLoaded] = useState<{
        url: string
        api?: HistoricalImportConverterApi
        error?: string
        loading: boolean
    }>()
    const configured = baseUrl?.trim()
    useEffect(() => {
        if (!configured) return
        let active = true
        setLoaded((previous) => ({
            url: configured,
            api: previous?.url === configured ? previous.api : undefined,
            loading: true,
        }))
        const fail = (message: string) =>
            active &&
            setLoaded((previous) => ({
                url: configured,
                api: previous?.url === configured ? previous.api : undefined,
                error: message,
                loading: false,
            }))
        let client: ReturnType<typeof createHistoricalImportConverter>
        try {
            client = createHistoricalImportConverter(configured)
        } catch {
            fail(
                "Could not start the historical import converter worker. Check browser support and local asset configuration."
            )
            return
        }
        client.ready.then(
            (api) => active && setLoaded({url: configured, api, loading: false}),
            (error: unknown) =>
                fail(error instanceof Error ? error.message : "Could not load converter plugin.")
        )
        return () => {
            active = false
            client.dispose()
        }
    }, [configured, attempt])
    if (!configured) return null
    const current = loaded?.url === configured ? loaded : undefined
    return (
        <>
            {current?.error && (
                <Typography role="alert" color="error">
                    {current.error}
                </Typography>
            )}
            {(!current || current.loading) && (
                <Typography role="status">
                    {String(
                        t("electionEventScreen.import.historical.loading", {
                            defaultValue: "Loading configured historical import converter…",
                        })
                    )}
                </Typography>
            )}
            <Button
                disabled={panelProps.disabled || !current || current.loading}
                onClick={() => setAttempt((value) => value + 1)}
            >
                {String(
                    t("electionEventScreen.import.historical.reload", {
                        defaultValue: "Reload converter",
                    })
                )}
            </Button>
            {current?.api && (
                <HistoricalSourcePanel
                    key={configured}
                    {...panelProps}
                    disabled={panelProps.disabled || current.loading || !!current.error}
                    api={current.api}
                />
            )}
        </>
    )
}
