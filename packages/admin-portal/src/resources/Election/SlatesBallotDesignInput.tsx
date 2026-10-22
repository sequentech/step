// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {SelectInput, required} from "react-admin"
import {useController} from "react-hook-form"
import {FormControlLabel, FormGroup, FormHelperText, Switch} from "@mui/material"
import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {EMobileCandidateLists} from "@sequentech/ui-core"
import {SLATES_ENABLED_FIELD, SLATES_MOBILE_CANDIDATE_LISTS_FIELD} from "@/utils/slates"

interface SlatesBallotDesignInputProps {
    /** How many slates the election has stored. */
    slatesCount: number
    disabled?: boolean
}

export const SlatesBallotDesignInput: React.FC<SlatesBallotDesignInputProps> = ({
    slatesCount,
    disabled,
}) => {
    const {t} = useTranslation()
    const {field} = useController({name: SLATES_ENABLED_FIELD})
    const [isConfirmOpen, setIsConfirmOpen] = useState(false)
    const isEnabled = field.value === true

    const onToggle = (event: React.ChangeEvent<HTMLInputElement>) => {
        if (!event.target.checked && slatesCount > 0) {
            setIsConfirmOpen(true)
            return
        }
        field.onChange(event.target.checked)
    }

    return (
        <>
            <FormGroup sx={{marginBottom: "1rem"}}>
                <FormControlLabel
                    control={
                        <Switch
                            name={field.name}
                            checked={isEnabled}
                            onChange={onToggle}
                            onBlur={field.onBlur}
                            disabled={disabled}
                            color="primary"
                        />
                    }
                    label={t("electionScreen.slates.enable.label")}
                />
                <FormHelperText sx={{marginTop: 0}}>
                    {t("electionScreen.slates.enable.helper")}
                </FormHelperText>
            </FormGroup>
            {isEnabled && (
                <SelectInput
                    source={SLATES_MOBILE_CANDIDATE_LISTS_FIELD}
                    label={String(t("electionScreen.slates.mobileCandidateLists.label"))}
                    helperText={String(t("electionScreen.slates.mobileCandidateLists.helper"))}
                    choices={Object.values(EMobileCandidateLists).map((option) => ({
                        id: option,
                        name: t(`electionScreen.slates.mobileCandidateLists.options.${option}`),
                    }))}
                    defaultValue={EMobileCandidateLists.COLLAPSED}
                    validate={required()}
                    disabled={disabled}
                />
            )}
            <Dialog
                variant="warning"
                open={isConfirmOpen}
                ok={String(t("electionScreen.slates.disable.confirm"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.warning"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        field.onChange(false)
                    }
                    setIsConfirmOpen(false)
                }}
            >
                {t("electionScreen.slates.disable.message", {count: slatesCount})}
            </Dialog>
        </>
    )
}
