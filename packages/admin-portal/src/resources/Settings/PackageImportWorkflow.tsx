// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useId} from "react"
import {Box, Paper, Stack, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {ApplyMode, Outcome} from "./configurationPackage"

export type PackageImportPhase =
    | "waitingForTrust"
    | "trustBlocked"
    | "choose"
    | "starting"
    | "previewing"
    | "previewBlocked"
    | "previewAgain"
    | "review"
    | "applying"
    | "complete"
    | "attention"

export interface PackageImportSignals {
    permitsImport: boolean
    rosterLoading: boolean
    submitting: boolean
    running: boolean
    submittedMode?: ApplyMode
    outcome?: Outcome
    canApply: boolean
    hasBlockers: boolean
    previewRefused: boolean
    unreadable: boolean
    noRecord: boolean
    hasError: boolean
    hasScopeMismatch: boolean
}

// Presentation only: the phase picks copy and emphasis. It admits nothing; the parent keeps
// its own gates (canApply, trust, confirmation) and must not feed a phase back into them.
export const packageImportPhase = (signals: PackageImportSignals): PackageImportPhase => {
    const {submittedMode, outcome} = signals

    // Missing trust outranks everything else on the page.
    if (!signals.permitsImport) {
        return signals.rosterLoading ? "waitingForTrust" : "trustBlocked"
    }
    // A request in flight has no task yet, so an earlier run's mode says nothing about it.
    if (signals.submitting) {
        return "starting"
    }
    if (signals.unreadable || signals.noRecord || signals.hasError) {
        return "attention"
    }
    if (signals.running) {
        if (!submittedMode) {
            return "starting"
        }
        return submittedMode === "apply" ? "applying" : "previewing"
    }
    if (signals.previewRefused && submittedMode === "preview") {
        return "previewBlocked"
    }
    if (outcome === "failed" || outcome === "refused" || outcome === "unknown") {
        return "attention"
    }
    if (outcome === "applied" && submittedMode === "apply") {
        return "complete"
    }
    if (signals.canApply) {
        return "review"
    }
    if (signals.hasBlockers) {
        return "previewBlocked"
    }
    // A preview that finished but may not be applied is no longer the selected one:
    // the options, the checksum or the receiver's trust changed since.
    if (signals.hasScopeMismatch || outcome === "previewed") {
        return "previewAgain"
    }
    return "choose"
}

const STEPS = ["upload", "preview", "review", "apply"] as const

type Tone = "neutral" | "info" | "warning" | "success" | "error"

interface PhaseStage {
    // Index into STEPS of the step in progress, when one is known.
    current: number | null
    // How many leading steps are known to be behind us.
    done: number
    tone: Tone
}

// No success tone before an apply has actually finished, and no step is claimed as current
// or done when the phase cannot tell (trust missing, mode unknown, outcome unsettled).
const PHASE_STAGE: Record<PackageImportPhase, PhaseStage> = {
    waitingForTrust: {current: null, done: 0, tone: "neutral"},
    trustBlocked: {current: null, done: 0, tone: "warning"},
    choose: {current: 0, done: 0, tone: "neutral"},
    starting: {current: null, done: 0, tone: "info"},
    previewing: {current: 1, done: 1, tone: "info"},
    previewBlocked: {current: 1, done: 1, tone: "warning"},
    previewAgain: {current: 1, done: 1, tone: "warning"},
    review: {current: 2, done: 2, tone: "info"},
    applying: {current: 3, done: 3, tone: "info"},
    complete: {current: null, done: STEPS.length, tone: "success"},
    attention: {current: null, done: 0, tone: "error"},
}

const SAFETY_NOTE: Partial<Record<PackageImportPhase, string>> = {
    choose: "configurationPackage.verificationNote",
    previewing: "configurationPackage.verificationNote",
    review: "configurationPackage.applyRechecks",
}

export interface PackageImportWorkflowProps {
    phase: PackageImportPhase
    uploadAction: React.ReactNode
    applyAction: React.ReactNode
    children?: React.ReactNode
}

export const PackageImportWorkflow: React.FC<PackageImportWorkflowProps> = ({
    phase,
    uploadAction,
    applyAction,
    children,
}) => {
    const {t} = useTranslation()
    const titleId = useId()
    const {current, done, tone} = PHASE_STAGE[phase]
    const accent = tone === "neutral" ? "primary" : tone
    const safetyNote = SAFETY_NOTE[phase]

    return (
        <Paper
            component="section"
            variant="outlined"
            aria-labelledby={titleId}
            data-testid="package-workflow"
            sx={{
                p: 2,
                borderInlineStart: 4,
                borderInlineStartColor: tone === "neutral" ? "divider" : `${tone}.main`,
            }}
        >
            <Stack spacing={1.5}>
                <Box
                    component="ol"
                    role="list"
                    aria-label={String(t("configurationPackage.title"))}
                    sx={{
                        display: "flex",
                        flexWrap: "wrap",
                        alignItems: "center",
                        columnGap: {xs: 2, sm: 1},
                        rowGap: 0.5,
                        listStyle: "none",
                        m: 0,
                        p: 0,
                    }}
                >
                    {STEPS.map((step, index) => {
                        const isCurrent = index === current
                        const isDone = index < done
                        const isLast = index === STEPS.length - 1
                        return (
                            <Box
                                component="li"
                                key={step}
                                aria-current={isCurrent ? "step" : undefined}
                                data-testid={isCurrent ? "package-workflow-current" : undefined}
                                sx={{
                                    "display": "flex",
                                    "alignItems": "center",
                                    "gap": 1,
                                    "flex": {sm: isLast ? "none" : 1},
                                    // Connector to the next step, on wide layouts only.
                                    "&::after": isLast
                                        ? undefined
                                        : {
                                              content: '""',
                                              display: {xs: "none", sm: "block"},
                                              flex: 1,
                                              borderTop: 1,
                                              borderColor: "divider",
                                          },
                                }}
                            >
                                <Box
                                    component="span"
                                    aria-hidden="true"
                                    sx={{
                                        display: "inline-flex",
                                        alignItems: "center",
                                        justifyContent: "center",
                                        flexShrink: 0,
                                        width: (theme) => theme.spacing(3),
                                        height: (theme) => theme.spacing(3),
                                        borderRadius: "50%",
                                        border: 1,
                                        typography: "caption",
                                        borderColor: isCurrent
                                            ? `${accent}.main`
                                            : isDone
                                              ? "text.secondary"
                                              : "divider",
                                        bgcolor: isCurrent ? `${accent}.main` : "transparent",
                                        color: isCurrent
                                            ? `${accent}.contrastText`
                                            : "text.secondary",
                                    }}
                                >
                                    {isDone ? "✓" : index + 1}
                                </Box>
                                <Typography
                                    component="span"
                                    variant="body2"
                                    sx={{
                                        color: isCurrent ? "text.primary" : "text.secondary",
                                        fontWeight: isCurrent
                                            ? "fontWeightBold"
                                            : "fontWeightRegular",
                                    }}
                                >
                                    {t(`configurationPackage.workflow.steps.${step}`)}
                                </Typography>
                            </Box>
                        )
                    })}
                </Box>

                {/* Not a live region: the task outcome in the run panel is the one that announces. */}
                <Box>
                    <Typography id={titleId} variant="h6" component="h3">
                        {t(`configurationPackage.workflow.phases.${phase}.title`)}
                    </Typography>
                    <Typography
                        variant="body2"
                        color="text.secondary"
                        sx={{mt: 0.25, maxWidth: "75ch"}}
                    >
                        {t(`configurationPackage.workflow.phases.${phase}.help`)}
                    </Typography>
                </Box>

                <Stack
                    direction={{xs: "column", sm: "row"}}
                    spacing={1}
                    sx={{alignItems: {xs: "stretch", sm: "center"}}}
                >
                    {uploadAction}
                    {applyAction}
                </Stack>

                {children}

                {safetyNote && (
                    <Typography
                        variant="caption"
                        color="text.secondary"
                        data-testid="package-safety-note"
                    >
                        {t(safetyNote)}
                    </Typography>
                )}
            </Stack>
        </Paper>
    )
}
