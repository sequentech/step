// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import {WasmWrapper} from "@voting-portal/providers/WasmWrapper"
import {LocStudioProvider} from "../LocStudioContext"
import {LocStudioApp} from "../App"
import {initializeLocStudioI18n} from "../i18n"
import {OverridesByLanguage} from "../i18n"
import {UploadedElectionEvent, parseUploadedElectionEvent} from "../uploadedElection"
import "@voting-portal/index.css"
import "../loc-studio.css"

let i18nInitialized = false

const ensureI18n = (): void => {
    if (!i18nInitialized) {
        initializeLocStudioI18n()
        i18nInitialized = true
    }
}

export interface LocStudioEmbedProps {
    embedded?: boolean
    className?: string
    initialElectionEventJson?: unknown
    initialOverrides?: OverridesByLanguage
    onSave?: (result: import("./saveResult").LocStudioSaveResult) => void | Promise<void>
    saving?: boolean
}

export const LocStudioEmbed: React.FC<LocStudioEmbedProps> = ({
    embedded = true,
    initialElectionEventJson,
    initialOverrides,
    onSave,
    saving = false,
}) => {
    useEffect(() => {
        ensureI18n()
    }, [])

    const initialUploadedEvent = React.useMemo((): UploadedElectionEvent | null => {
        if (!initialElectionEventJson) {
            return null
        }
        try {
            return parseUploadedElectionEvent(initialElectionEventJson, "embedded-event.json")
        } catch {
            return null
        }
    }, [initialElectionEventJson])

    return (
        <WasmWrapper>
            <ThemeProvider theme={theme}>
                <LocStudioProvider
                    initialUploadedEvent={initialUploadedEvent}
                    initialOverrides={initialOverrides}
                >
                    <LocStudioApp embedded={embedded} onSave={onSave} saving={saving} />
                </LocStudioProvider>
            </ThemeProvider>
        </WasmWrapper>
    )
}
