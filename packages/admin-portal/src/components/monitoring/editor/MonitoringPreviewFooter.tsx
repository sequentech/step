// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useTranslation} from "react-i18next"
import {Box, Button, CircularProgress, Typography} from "@mui/material"
import {EPreviewStatus} from "./yamlDraft"

/** What the footer's buttons are waiting for. */
export enum EEditorBusy {
    IDLE = "IDLE",
    VALIDATING = "VALIDATING",
    SAVING = "SAVING",
}

export interface MonitoringPreviewFooterProps {
    /** Where the preview is drawn: "All authorized Posts", a Post, a country. */
    scopeLabel?: string
    errors: number
    warnings: number
    previewStatus?: EPreviewStatus
    renderMs?: number | null
    revision?: number | null
    savedAt?: string | null
    savedBy?: string | null
    dirty: boolean
    busy?: EEditorBusy
    saveLabel: string
    /** Why Save is off, when it is; a save with errors would be refused anyway. */
    saveBlocked?: boolean
    /** A document without a preview (a dashboard) says nothing about one. */
    hasPreview?: boolean
    onCancel: () => void
    onValidate?: () => void
    onSave: () => void
}

const formatDate = (value: string, locale: string) => {
    const date = new Date(value)
    if (Number.isNaN(date.getTime())) return value
    return new Intl.DateTimeFormat(locale, {dateStyle: "medium", timeStyle: "short"}).format(date)
}

/**
 * Preview · {scope} · Valid · no chart warnings · rendered in {n} ms ·
 * Revision {n} · saved {date} by {user} · Cancel · Validate · Save.
 */
export const MonitoringPreviewFooter: React.FC<MonitoringPreviewFooterProps> = ({
    scopeLabel,
    errors,
    warnings,
    previewStatus = EPreviewStatus.IDLE,
    renderMs,
    revision,
    savedAt,
    savedBy,
    dirty,
    busy = EEditorBusy.IDLE,
    saveLabel,
    saveBlocked = false,
    hasPreview = true,
    onCancel,
    onValidate,
    onSave,
}) => {
    const {t, i18n} = useTranslation()
    const facts: Array<{key: string; text: string; tone?: "error" | "success"}> = []
    if (hasPreview) {
        facts.push({
            key: "preview",
            text: scopeLabel
                ? `${t("monitoring.editor.footer.preview")} · ${scopeLabel}`
                : t("monitoring.editor.footer.preview"),
        })
    }
    facts.push(
        errors > 0
            ? {
                  key: "valid",
                  text: t("monitoring.editor.footer.errors", {count: errors}),
                  tone: "error",
              }
            : {key: "valid", text: t("monitoring.editor.footer.valid"), tone: "success"}
    )
    if (warnings > 0) {
        facts.push({
            key: "warnings",
            text: t("monitoring.editor.footer.warnings", {count: warnings}),
        })
    } else if (hasPreview) {
        facts.push({key: "warnings", text: t("monitoring.editor.footer.noWarnings")})
    }
    if (previewStatus === EPreviewStatus.RENDERING) {
        facts.push({key: "render", text: t("monitoring.editor.footer.rendering")})
    } else if (previewStatus === EPreviewStatus.FAILED) {
        facts.push({
            key: "render",
            text: t("monitoring.editor.footer.previewFailed"),
            tone: "error",
        })
    } else if (typeof renderMs === "number") {
        facts.push({key: "render", text: t("monitoring.editor.footer.renderedIn", {ms: renderMs})})
    }
    if (typeof revision === "number") {
        facts.push({key: "revision", text: t("monitoring.editor.footer.revision", {revision})})
        if (savedAt) {
            facts.push({
                key: "saved",
                text: t("monitoring.editor.footer.savedBy", {
                    date: formatDate(savedAt, i18n.language),
                    user: savedBy || t("monitoring.editor.footer.unknownUser"),
                }),
            })
        }
    } else {
        facts.push({key: "revision", text: t("monitoring.editor.footer.notSaved")})
    }
    if (dirty) facts.push({key: "dirty", text: t("monitoring.editor.footer.unsaved")})

    return (
        <Box
            component="footer"
            sx={{
                display: "flex",
                flexWrap: "wrap",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 1,
                pt: 1,
            }}
        >
            <Box
                role="status"
                aria-live="polite"
                aria-label={
                    hasPreview
                        ? t("monitoring.editor.footer.preview")
                        : t("monitoring.editor.diagnostics.title")
                }
                sx={{display: "flex", flexWrap: "wrap", gap: 0.5, alignItems: "center"}}
            >
                {facts.map((fact, index) => (
                    <Typography
                        key={fact.key}
                        variant="body2"
                        component="span"
                        color={fact.tone ? `${fact.tone}.main` : "text.secondary"}
                    >
                        {index > 0 ? "· " : ""}
                        {fact.text}
                    </Typography>
                ))}
            </Box>
            <Box sx={{display: "flex", gap: 1, flexWrap: "wrap", ml: "auto"}}>
                <Button variant="text" onClick={onCancel} disabled={busy === EEditorBusy.SAVING}>
                    {t("monitoring.editor.footer.cancel")}
                </Button>
                {onValidate ? (
                    <Button
                        variant="outlined"
                        onClick={onValidate}
                        disabled={busy !== EEditorBusy.IDLE}
                        startIcon={
                            busy === EEditorBusy.VALIDATING ? (
                                <CircularProgress size={16} aria-hidden />
                            ) : undefined
                        }
                    >
                        {t("monitoring.editor.footer.validate")}
                    </Button>
                ) : null}
                <Button
                    variant="contained"
                    onClick={onSave}
                    disabled={busy !== EEditorBusy.IDLE || saveBlocked || errors > 0}
                    startIcon={
                        busy === EEditorBusy.SAVING ? (
                            <CircularProgress size={16} aria-hidden />
                        ) : undefined
                    }
                >
                    {saveLabel}
                </Button>
            </Box>
        </Box>
    )
}

export default MonitoringPreviewFooter
