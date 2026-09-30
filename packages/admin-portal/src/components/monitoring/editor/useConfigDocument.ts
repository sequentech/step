// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useCallback, useEffect, useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import {monitoringErrorMessage, type IMonitoringEditorApi} from "./api"
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
    WARNING = "warning",
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
    /** The revision Harvest reported; `null` when the document was removed meanwhile. */
    currentRevision: number | null
    author?: IMonitoringAuthor | null
    time?: string | null
    /** The saved revision's YAML, once fetched; empty for a removal. */
    theirs?: string
    /** The revision fetched with `theirs`: what keeping the draft replaces. */
    theirsRevision?: number | null
    /** Why `theirs` could not be fetched. */
    theirsError?: string
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
    /** Read at call time: a new `t` (another language) must not load the document again. */
    const tRef = useRef(t)
    tRef.current = t
    const [load, setLoad] = useState(EDocumentLoad.LOADING)
    const [loadError, setLoadError] = useState("")
    const [revision, setRevision] = useState<IDocumentRevision>({revision: null})
    /** The revision a save replaces; moves on when the author has seen a newer one. */
    const [expected, setExpected] = useState<number | null>(null)
    const [busy, setBusy] = useState(EEditorBusy.IDLE)
    const [message, setMessage] = useState<IDocumentMessage | null>(null)
    const [conflict, setConflict] = useState<IDocumentConflict | null>(null)

    const failed = useCallback(
        (error: unknown) => {
            const explained = monitoringErrorMessage(error)
            setMessage({
                tone: EMessageTone.ERROR,
                text: explained
                    ? tRef.current(explained)
                    : tRef.current(messages.requestFailed, {reason: reason(error)}),
            })
        },
        [messages.requestFailed]
    )

    const adopt = useCallback(
        (document: IMonitoringConfigDocument) => {
            controller.reset(document.yaml ?? "")
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
                setLoadError(tRef.current(messages.loadFailed, {reason: reason(error)}))
                setLoad(EDocumentLoad.FAILED)
            }
        )
        return () => {
            current = false
        }
    }, [open, api, kind, key, adopt, messages.loadFailed])

    const validate = async () => {
        setBusy(EEditorBusy.VALIDATING)
        setMessage(null)
        const sent = controller.getState().text
        try {
            const response = await api.validateConfig({kind, key, yaml: sent})
            if (response.preview) controller.acceptPreview(response.preview, sent)
            controller.setServerProblems(response.problems, sent)
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

    /**
     * Who stored a revision, and when, read back for a Harvest whose save
     * does not answer with them, unless a newer revision is shown by then.
     */
    const learnAuthor = (saved: number) => {
        api.getConfig({kind, key, revision: saved}).then(
            (document) =>
                setRevision((shown) =>
                    shown.revision === saved
                        ? {
                              revision: saved,
                              author: document.author,
                              createdAt: document.created_at ?? shown.createdAt,
                          }
                        : shown
                ),
            () => undefined
        )
    }

    const save = async (): Promise<boolean> => {
        setBusy(EEditorBusy.SAVING)
        setMessage(null)
        const sent = controller.getState().text
        try {
            const outcome = await api.saveConfig({
                kind,
                key,
                yaml: sent,
                expected_revision: expected,
                change: EMonitoringSaveChange.UPSERT,
            })
            if (outcome.status === EMonitoringSaveStatus.SAVED) {
                controller.markSaved(sent)
                setRevision({
                    revision: outcome.revision,
                    author: outcome.author ?? undefined,
                    createdAt: outcome.created_at ?? new Date().toISOString(),
                })
                setExpected(outcome.revision)
                if (!outcome.author || !outcome.created_at) learnAuthor(outcome.revision)
                const saved = t(messages.saved, {revision: outcome.revision})
                if (outcome.warnings.length) {
                    controller.setServerProblems(outcome.warnings, sent)
                    setMessage({
                        tone: EMessageTone.WARNING,
                        text: `${saved} · ${t("monitoring.editor.document.savedWithWarnings", {
                            count: outcome.warnings.length,
                        })}`,
                    })
                } else {
                    setMessage({tone: EMessageTone.SUCCESS, text: saved})
                }
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
                            previous
                                ? {
                                      ...previous,
                                      theirs: document.yaml ?? "",
                                      theirsRevision:
                                          document.yaml === null ? null : document.revision,
                                      theirsError: undefined,
                                  }
                                : previous
                        ),
                    (error) =>
                        setConflict((previous) =>
                            previous
                                ? {
                                      ...previous,
                                      theirsError: tRef.current(messages.requestFailed, {
                                          reason: reason(error),
                                      }),
                                  }
                                : previous
                        )
                )
                return false
            }
            controller.setServerProblems(outcome.problems, sent)
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

    /**
     * Keeps the draft: the author has seen the newer revision, so the next
     * save replaces it. The revision fetched is the one they saw; without
     * it, the one Harvest reported (`null`: the document is gone, and the
     * save creates it again).
     */
    const keepEditing = () => {
        if (conflict) {
            setExpected(
                conflict.theirsRevision !== undefined
                    ? conflict.theirsRevision
                    : conflict.currentRevision
            )
        }
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
