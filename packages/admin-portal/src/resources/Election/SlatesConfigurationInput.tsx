// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useEffect} from "react"
import {TextInput, useGetList} from "react-admin"
import {useFormContext, useWatch} from "react-hook-form"
import {
    Box,
    FormControl,
    FormHelperText,
    InputLabel,
    MenuItem,
    Select,
    SelectChangeEvent,
    Typography,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {EMobileCandidateLists} from "@sequentech/ui-core"
import {Sequent_Backend_Candidate, Sequent_Backend_Contest} from "@/gql/graphql"
import {
    SLATES_FORM_FIELD,
    checkSlatesConfiguration,
    checkSlatesStructure,
    formatSlatesProblems,
    getMobileCandidateLists,
    setMobileCandidateLists,
} from "@/utils/slates"

const MOBILE_LISTS_LABEL_ID = "slates-mobile-candidate-lists-label"

interface SlatesConfigurationInputProps {
    defaultLanguage: string
    contests: Array<Sequent_Backend_Contest> | undefined
    tenantId: string | undefined
    electionEventId: string | undefined
}

export const SlatesConfigurationInput: React.FC<SlatesConfigurationInputProps> = ({
    defaultLanguage,
    contests,
    tenantId,
    electionEventId,
}) => {
    const {t} = useTranslation()
    const {setValue, trigger, getFieldState} = useFormContext()
    const text = useWatch({name: SLATES_FORM_FIELD}) as string | null | undefined
    const mobileCandidateLists = getMobileCandidateLists(text)
    const hasConfiguration = !!text?.trim()

    const {data: candidates, total: candidatesTotal} = useGetList<Sequent_Backend_Candidate>(
        "sequent_backend_candidate",
        {
            filter: {tenant_id: tenantId, election_event_id: electionEventId},
            pagination: {page: 1, perPage: 9999},
        },
        {enabled: hasConfiguration && !!tenantId && !!electionEventId}
    )
    const isCandidateListPartial = !!candidates && (candidatesTotal ?? 0) > candidates.length

    useEffect(() => {
        if (candidates && getFieldState(SLATES_FORM_FIELD).isDirty) {
            trigger(SLATES_FORM_FIELD)
        }
    }, [candidates, getFieldState, trigger])

    const validate = useCallback(
        (value: string | null | undefined) => {
            if (!value?.trim()) {
                return undefined
            }
            if (!contests || !candidates) {
                return t("electionScreen.slates.loading")
            }
            try {
                const problems = isCandidateListPartial
                    ? checkSlatesStructure(value)
                    : checkSlatesConfiguration(value, defaultLanguage, contests, candidates)
                return problems.length > 0 ? formatSlatesProblems(problems) : undefined
            } catch (error) {
                return error instanceof Error ? error.message : String(error)
            }
        },
        [contests, candidates, isCandidateListPartial, defaultLanguage, t]
    )

    const onMobileCandidateListsChange = (event: SelectChangeEvent<EMobileCandidateLists>) => {
        const selected = Object.values(EMobileCandidateLists).find(
            (option) => option === event.target.value
        )
        if (!selected || !text) {
            return
        }
        setValue(SLATES_FORM_FIELD, setMobileCandidateLists(text, selected), {
            shouldDirty: true,
            shouldValidate: true,
        })
    }

    return (
        <Box sx={{width: "100%", marginBottom: "1rem"}}>
            <Typography variant="body1" component="span" sx={{fontWeight: "bold"}}>
                {t("electionScreen.slates.title")}
            </Typography>
            <TextInput
                source={SLATES_FORM_FIELD}
                label={String(t("electionScreen.slates.configuration"))}
                helperText={String(t("electionScreen.slates.helper"))}
                validate={validate}
                multiline
                minRows={6}
                maxRows={24}
                fullWidth
                slotProps={{
                    htmlInput: {spellCheck: false, style: {fontFamily: "monospace"}},
                    formHelperText: {sx: {whiteSpace: "pre-wrap"}},
                }}
            />
            <FormControl fullWidth disabled={!mobileCandidateLists}>
                <InputLabel id={MOBILE_LISTS_LABEL_ID}>
                    {t("electionScreen.slates.mobileCandidateLists.label")}
                </InputLabel>
                <Select
                    labelId={MOBILE_LISTS_LABEL_ID}
                    label={t("electionScreen.slates.mobileCandidateLists.label")}
                    value={mobileCandidateLists ?? ""}
                    onChange={onMobileCandidateListsChange}
                >
                    {Object.values(EMobileCandidateLists).map((option) => (
                        <MenuItem key={option} value={option}>
                            {t(`electionScreen.slates.mobileCandidateLists.options.${option}`)}
                        </MenuItem>
                    ))}
                </Select>
                <FormHelperText disabled={false}>
                    {t("electionScreen.slates.mobileCandidateLists.helper")}
                </FormHelperText>
            </FormControl>
        </Box>
    )
}
