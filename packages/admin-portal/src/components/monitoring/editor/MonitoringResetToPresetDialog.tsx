// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogContentText,
    DialogTitle,
    FormControl,
    FormControlLabel,
    FormLabel,
    Radio,
    RadioGroup,
} from "@mui/material"
import type {IMonitoringEditorApi} from "./api"
import type {IMonitoringPreset} from "./types"

export interface MonitoringResetToPresetDialogProps {
    open: boolean
    api: IMonitoringEditorApi
    onClose: () => void
    /** After the event's configuration was replaced; the view reloads its dashboards. */
    onReset?: (generation: number, preset: IMonitoringPreset) => void
}

enum EResetStep {
    LOADING = "LOADING",
    CHOOSING = "CHOOSING",
    RESETTING = "RESETTING",
    DONE = "DONE",
}

const reason = (error: unknown) => (error instanceof Error ? error.message : String(error))

/** Reset to preset: choose a preset, read what happens, confirm. */
export const MonitoringResetToPresetDialog: React.FC<MonitoringResetToPresetDialogProps> = ({
    open,
    api,
    onClose,
    onReset,
}) => {
    const {t} = useTranslation()
    const [step, setStep] = useState(EResetStep.LOADING)
    const [presets, setPresets] = useState<IMonitoringPreset[]>([])
    const [chosen, setChosen] = useState("")
    const [error, setError] = useState("")

    useEffect(() => {
        if (!open) return
        let current = true
        setStep(EResetStep.LOADING)
        setError("")
        api.listPresets().then(
            (list) => {
                if (!current) return
                setPresets(list)
                setChosen(list[0]?.id ?? "")
                setStep(EResetStep.CHOOSING)
            },
            (failure) => {
                if (!current) return
                setError(t("monitoring.editor.reset.failed", {reason: reason(failure)}))
                setStep(EResetStep.CHOOSING)
            }
        )
        return () => {
            current = false
        }
    }, [open, api, t])

    const preset = presets.find((candidate) => candidate.id === chosen)
    const reset = async () => {
        if (!preset) return
        setStep(EResetStep.RESETTING)
        setError("")
        try {
            const {generation} = await api.resetToPreset(preset.id)
            setStep(EResetStep.DONE)
            onReset?.(generation, preset)
        } catch (failure) {
            setError(t("monitoring.editor.reset.failed", {reason: reason(failure)}))
            setStep(EResetStep.CHOOSING)
        }
    }

    return (
        <Dialog
            open={open}
            onClose={onClose}
            maxWidth="xs"
            fullWidth
            aria-labelledby="monitoring-reset-title"
        >
            <DialogTitle id="monitoring-reset-title">
                {t("monitoring.editor.reset.title")}
            </DialogTitle>
            <DialogContent sx={{display: "flex", flexDirection: "column", gap: 2}}>
                {step === EResetStep.LOADING ? (
                    <Box sx={{display: "flex", justifyContent: "center", py: 3}}>
                        <CircularProgress aria-label={t("monitoring.editor.reset.loading")} />
                    </Box>
                ) : null}
                {step === EResetStep.DONE && preset ? (
                    <Alert severity="success">
                        {t("monitoring.editor.reset.done", {title: preset.title})}
                    </Alert>
                ) : null}
                {error ? <Alert severity="error">{error}</Alert> : null}
                {step === EResetStep.CHOOSING || step === EResetStep.RESETTING ? (
                    presets.length === 0 ? (
                        error ? null : (
                            <Alert severity="info">{t("monitoring.editor.reset.noPresets")}</Alert>
                        )
                    ) : (
                        <>
                            <DialogContentText>
                                {t("monitoring.editor.reset.body")}
                            </DialogContentText>
                            <FormControl disabled={step === EResetStep.RESETTING}>
                                <FormLabel id="monitoring-reset-preset">
                                    {t("monitoring.editor.reset.preset")}
                                </FormLabel>
                                <RadioGroup
                                    aria-labelledby="monitoring-reset-preset"
                                    value={chosen}
                                    onChange={(event) => setChosen(event.target.value)}
                                >
                                    {presets.map((candidate) => (
                                        <FormControlLabel
                                            key={candidate.id}
                                            value={candidate.id}
                                            control={<Radio />}
                                            label={`${candidate.title} · v${candidate.version}`}
                                        />
                                    ))}
                                </RadioGroup>
                            </FormControl>
                        </>
                    )
                ) : null}
            </DialogContent>
            <DialogActions>
                <Button onClick={onClose} disabled={step === EResetStep.RESETTING}>
                    {step === EResetStep.DONE
                        ? t("monitoring.editor.catalog.close")
                        : t("monitoring.editor.reset.cancel")}
                </Button>
                {step !== EResetStep.DONE ? (
                    <Button
                        color="error"
                        variant="contained"
                        onClick={reset}
                        disabled={!preset || step !== EResetStep.CHOOSING}
                    >
                        {t("monitoring.editor.reset.confirm")}
                    </Button>
                ) : null}
            </DialogActions>
        </Dialog>
    )
}

export default MonitoringResetToPresetDialog
