// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useCallback, useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import type {IMonitoringEditorApi} from "./api"
import {
    EMonitoringConfigKind,
    EMonitoringSaveChange,
    EMonitoringSaveStatus,
    EMonitoringValidationResult,
    type IMonitoringAuthor,
    type IMonitoringConfigDocument,
} from "./types"
import {EEditorBusy} from "./MonitoringPreviewFooter"
import type {YamlDraftController} from "./yamlDraft"

export enum EDocumentLoad {
    LOADING = "LOADING",
    READY = "READY",
    FAILED = "FAILED",
}

export enum EMessageTone {
    SUCCESS = "success",
    ERROR = "error",
}

export interface IDocumentMessage {
    tone: EMessageTone
    text: string
}

export interface IDocumentRevision {
    revision: number | null
    author?: IMonitoringAuthor | null
    createdAt?: string | null
}

export interface IDocumentConflict {
    currentRevision: number
    author?: IMonitoringAuthor | null
    time?: string | null
    /** The saved revision's YAML, once fetched. */
    theirs?: string
}

/** Translation keys of what the document's dialog says, e.g. `monitoring.editor.theme.saved`. */
export interface IDocumentMessages {
    loadFailed: string
    saved: string
    refused: string
    validated: string
    invalid: string
    requestFailed: string
}

export interface IConfigDocumentOptions {
    api: IMonitoringEditorApi
    kind: EMonitoringConfigKind
    key: string
    /** Loads when it turns true. */
    open: boolean
    controller: YamlDraftController
    messages: IDocumentMessages
    onSaved?: (revision: number) => void
}

const reason = (error: unknown) => (error instanceof Error ? error.message : String(error))

/**
 * Loading, validating and saving one configuration document edited in a
 * {@link YamlDraftController}: the revision a save replaces, the 409 conflict
 * and the 422 problems, and a message for each outcome.
 */
export const useConfigDocument = ({
    api,
    kind,
    key,
    open,
    controller,
    messages,
    onSaved,
}: IConfigDocumentOptions) => {
    const {t} = useTranslation()
    const [load, setLoad] = useState(EDocumentLoad.LOADING)
    const [loadError, setLoadError] = useState("")
    const [revision, setRevision] = useState<IDocumentRevision>({revision: null})
    /** The revision a save replaces; moves on when the author has seen a newer one. */
    const [expected, setExpected] = useState<number | null>(null)
    const [busy, setBusy] = useState(EEditorBusy.IDLE)
    const [message, setMessage] = useState<IDocumentMessage | null>(null)
    const [conflict, setConflict] = useState<IDocumentConflict | null>(null)

    const failed = useCallback(
        (error: unknown) =>
            setMessage({
                tone: EMessageTone.ERROR,
                text: t(messages.requestFailed, {reason: reason(error)}),
            }),
        [t, messages.requestFailed]
    )

    const adopt = useCallback(
        (document: IMonitoringConfigDocument) => {
            controller.reset(document.yaml)
            setRevision({
                revision: document.revision,
                author: document.author,
                createdAt: document.created_at,
            })
            setExpected(document.revision)
        },
        [controller]
    )

    useEffect(() => {
        if (!open) return
        let current = true
        setLoad(EDocumentLoad.LOADING)
        setMessage(null)
        api.getConfig({kind, key}).then(
            (document) => {
                if (!current) return
                adopt(document)
                setLoad(EDocumentLoad.READY)
            },
            (error) => {
                if (!current) return
                setLoadError(t(messages.loadFailed, {reason: reason(error)}))
                setLoad(EDocumentLoad.FAILED)
            }
        )
        return () => {
            current = false
        }
    }, [open, api, kind, key, adopt, t, messages.loadFailed])

    const validate = async () => {
        setBusy(EEditorBusy.VALIDATING)
        setMessage(null)
        try {
            const response = await api.validateConfig({kind, key, yaml: controller.getState().text})
            if (response.preview) controller.acceptPreview(response.preview)
            controller.setServerProblems(response.problems)
            setMessage(
                response.result === EMonitoringValidationResult.VALID
                    ? {tone: EMessageTone.SUCCESS, text: t(messages.validated)}
                    : {tone: EMessageTone.ERROR, text: t(messages.invalid)}
            )
        } catch (error) {
            failed(error)
        } finally {
            setBusy(EEditorBusy.IDLE)
        }
    }

    const save = async (): Promise<boolean> => {
        setBusy(EEditorBusy.SAVING)
        setMessage(null)
        try {
            const outcome = await api.saveConfig({
                kind,
                key,
                yaml: controller.getState().text,
                expected_revision: expected,
                change: EMonitoringSaveChange.UPSERT,
            })
            if (outcome.status === EMonitoringSaveStatus.SAVED) {
                controller.markSaved()
                setRevision({revision: outcome.revision, createdAt: new Date().toISOString()})
                setExpected(outcome.revision)
                setMessage({
                    tone: EMessageTone.SUCCESS,
                    text: t(messages.saved, {revision: outcome.revision}),
                })
                onSaved?.(outcome.revision)
                return true
            }
            if (outcome.status === EMonitoringSaveStatus.CONFLICT) {
                setConflict({
                    currentRevision: outcome.current_revision,
                    author: outcome.author,
                    time: outcome.time,
                })
                api.getConfig({kind, key}).then(
                    (document) =>
                        setConflict((previous) =>
                            previous ? {...previous, theirs: document.yaml} : previous
                        ),
                    failed
                )
                return false
            }
            controller.setServerProblems(outcome.problems)
            setMessage({tone: EMessageTone.ERROR, text: t(messages.refused)})
            return false
        } catch (error) {
            failed(error)
            return false
        } finally {
            setBusy(EEditorBusy.IDLE)
        }
    }

    /** Replaces the draft with the revision saved meanwhile. */
    const reload = async () => {
        setConflict(null)
        try {
            adopt(await api.getConfig({kind, key}))
        } catch (error) {
            failed(error)
        }
    }

    /** Keeps the draft: the author has seen the newer revision, so the next save replaces it. */
    const keepEditing = () => {
        if (conflict) setExpected(conflict.currentRevision)
        setConflict(null)
    }

    return {
        load,
        loadError,
        revision,
        busy,
        message,
        setMessage,
        conflict,
        validate,
        save,
        reload,
        keepEditing,
    }
}

export const authorName = (author?: IMonitoringAuthor | null) =>
    author ? author.name || author.id : null
