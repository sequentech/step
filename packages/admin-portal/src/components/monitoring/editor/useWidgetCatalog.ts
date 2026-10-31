// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import type {IMonitoringEditorApi} from "./api"
import {loadWidgetCatalog, type IWidgetCatalogEntry} from "./catalog"
import {EMonitoringConfigKind} from "./types"

const reason = (error: unknown) => (error instanceof Error ? error.message : String(error))

/**
 * The event's widgets and themes, read once each time `open` turns true.
 * `catalog` is `undefined` while they are read.
 */
export const useWidgetCatalog = (api: IMonitoringEditorApi, open: boolean) => {
    const {t} = useTranslation()
    /** Read at call time: a new `t` (another language) must not read every widget again. */
    const tRef = useRef(t)
    tRef.current = t
    const [catalog, setCatalog] = useState<IWidgetCatalogEntry[] | undefined>()
    const [catalogError, setCatalogError] = useState("")
    const [themes, setThemes] = useState<string[]>([])

    useEffect(() => {
        if (!open) return
        let current = true
        setCatalog(undefined)
        setCatalogError("")
        loadWidgetCatalog(api).then(
            (entries) => current && setCatalog(entries),
            (error) =>
                current &&
                setCatalogError(
                    tRef.current("monitoring.editor.dashboard.requestFailed", {
                        reason: reason(error),
                    })
                )
        )
        api.listConfig().then(
            (documents) =>
                current &&
                setThemes(
                    documents
                        .filter((entry) => entry.kind === EMonitoringConfigKind.THEME)
                        .map((entry) => entry.key)
                ),
            () => undefined
        )
        return () => {
            current = false
        }
    }, [open, api])

    return {catalog, setCatalog, catalogError, themes}
}
