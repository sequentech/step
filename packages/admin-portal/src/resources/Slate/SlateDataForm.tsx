// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useState} from "react"
import {
    SaveButton,
    SelectInput,
    SimpleForm,
    TextInput,
    Toolbar,
    required,
    useNotify,
    useUpdate,
} from "react-admin"
import {FieldValues, useWatch} from "react-hook-form"
import {Accordion, AccordionDetails, AccordionSummary, Alert, Box, Typography} from "@mui/material"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {useTranslation} from "react-i18next"
import {EMobileCandidateLists, ISlate, ISlatesConfig} from "@sequentech/ui-core"
import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
} from "@/gql/graphql"
import {ElectionStyles} from "@/components/styles/ElectionStyles"
import {adminTheme} from "@sequentech/ui-essentials"
import {
    ISlateFormValues,
    SLATES_VERSION,
    checkSlatesConfiguration,
    putSlate,
    slateCandidateChoices,
    slateFormValues,
    slateFromForm,
    slatesProblemsFromError,
    writeSlatesConfig,
} from "@/utils/slates"

export interface SlateCandidateOption {
    id: string
    contest_id?: string | null
    presentation?: unknown
    label: string
}

interface ContestMembersRowProps {
    contest: Sequent_Backend_Contest
    contestName: string
    candidates: Array<SlateCandidateOption>
    config: ISlatesConfig | undefined
    slateId: string
    disabled: boolean
}

const ContestMembersRow: React.FC<ContestMembersRowProps> = ({
    contest,
    contestName,
    candidates,
    config,
    slateId,
    disabled,
}) => {
    const {t} = useTranslation()
    const source = `members.${contest.id}`
    const chosen = (useWatch({name: source}) as Array<string> | undefined) ?? []
    const offered = slateCandidateChoices(candidates, contest.id, config, slateId)

    return (
        <Box
            component="tr"
            sx={{"borderBottom": "1px solid rgba(0, 0, 0, 0.12)", "& > td": {verticalAlign: "top"}}}
        >
            <Box component="td" sx={{padding: "24px 16px", width: "240px"}}>
                <Typography sx={{fontWeight: 700, color: adminTheme.palette.brandColor}}>
                    {contestName}
                </Typography>
            </Box>
            <Box component="td" sx={{padding: "16px 0 8px", width: "600px", maxWidth: "100%"}}>
                {chosen.map((candidateId, index) => {
                    const pickedElsewhere = new Set(
                        chosen.filter((other, position) => position !== index && other)
                    )
                    const current = candidates.find(({id}) => id === candidateId)
                    const choices = offered.filter(({id}) => !pickedElsewhere.has(id))
                    if (current && !choices.some(({id}) => id === current.id)) {
                        choices.push(current)
                    }
                    return (
                        <SelectInput
                            key={`${contest.id}-${index}`}
                            source={`${source}.${index}`}
                            label={String(t("slateScreen.members.candidate"))}
                            choices={choices.map(({id, label}) => ({id, name: label}))}
                            emptyValue=""
                            emptyText={String(t("slateScreen.members.noCandidate"))}
                            disabled={disabled}
                            fullWidth
                            helperText={false}
                        />
                    )
                })}
            </Box>
        </Box>
    )
}

interface SlateDataFormProps {
    election: Sequent_Backend_Election
    slateId: string
    slate: ISlate | undefined
    config: ISlatesConfig | undefined
    contests: Array<Sequent_Backend_Contest>
    contestName: (contest: Sequent_Backend_Contest) => string
    candidates: Array<Sequent_Backend_Candidate>
    candidateOptions: Array<SlateCandidateOption>
    defaultLanguage: string
    canEdit: boolean
    onSaved: (slateId: string) => void
}

enum SlateSection {
    GENERAL = "general",
    MEMBERS = "members",
}

export const SlateDataForm: React.FC<SlateDataFormProps> = ({
    election,
    slateId,
    slate,
    config,
    contests,
    contestName,
    candidates,
    candidateOptions,
    defaultLanguage,
    canEdit,
    onSaved,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const [update] = useUpdate()
    const [problems, setProblems] = useState<Array<string>>([])
    const [collapsed, setCollapsed] = useState<Array<SlateSection>>([])

    const record = useMemo(
        () => ({id: slateId, ...slateFormValues(slate, contests, defaultLanguage)}),
        [slateId, slate, contests, defaultLanguage]
    )

    const toggleSection = (section: SlateSection) =>
        setCollapsed((current) =>
            current.includes(section)
                ? current.filter((other) => other !== section)
                : [...current, section]
        )

    const save = async (values: FieldValues) => {
        const edited = slateFromForm(slateId, values as ISlateFormValues, defaultLanguage, slate)
        const next = putSlate(
            config ?? {
                version: SLATES_VERSION,
                mobile_candidate_lists: EMobileCandidateLists.COLLAPSED,
                slates: [],
            },
            edited
        )
        try {
            const found = checkSlatesConfiguration(
                JSON.stringify(next),
                defaultLanguage,
                contests,
                candidates
            )
            if (found.length > 0) {
                setProblems(found.map(({message}) => message))
                notify(t("slateScreen.messages.invalid"), {type: "error"})
                return
            }
            await update(
                "sequent_backend_election",
                {
                    id: election.id,
                    data: {annotations: writeSlatesConfig(election.annotations, next)},
                    previousData: election,
                },
                {mutationMode: "pessimistic", returnPromise: true}
            )
            setProblems([])
            notify(t("slateScreen.messages.saved"), {type: "success"})
            onSaved(edited.id)
        } catch (error) {
            const thrown = slatesProblemsFromError(error)
            setProblems(
                thrown
                    ? thrown.map(({message}) => message)
                    : [error instanceof Error ? error.message : String(error)]
            )
            notify(t("slateScreen.messages.error"), {type: "error"})
        }
    }

    const sectionSummary = (section: SlateSection, title: string) => (
        <AccordionSummary expandIcon={<ExpandMoreIcon id={`slate-data-${section}`} />}>
            <ElectionStyles.Wrapper>
                <ElectionStyles.Title>{title}</ElectionStyles.Title>
            </ElectionStyles.Wrapper>
        </AccordionSummary>
    )

    return (
        <SimpleForm
            key={slateId}
            record={record}
            onSubmit={save}
            toolbar={<Toolbar>{canEdit && <SaveButton />}</Toolbar>}
        >
            <Accordion
                sx={{width: "100%"}}
                expanded={!collapsed.includes(SlateSection.GENERAL)}
                onChange={() => toggleSection(SlateSection.GENERAL)}
            >
                {sectionSummary(SlateSection.GENERAL, t("slateScreen.edit.general"))}
                <AccordionDetails>
                    <TextInput
                        source="name"
                        helperText={false}
                        label={String(t("slateScreen.field.name"))}
                        validate={required()}
                        disabled={!canEdit}
                    />
                    <TextInput
                        source="alias"
                        helperText={false}
                        label={String(t("slateScreen.field.alias"))}
                        disabled={!canEdit}
                    />
                    <TextInput
                        source="description"
                        helperText={false}
                        label={String(t("slateScreen.field.description"))}
                        multiline
                        disabled={!canEdit}
                    />
                    <TextInput
                        source="external_id"
                        helperText={false}
                        label={String(t("slateScreen.field.externalId"))}
                        disabled={!canEdit}
                    />
                </AccordionDetails>
            </Accordion>
            <Accordion
                sx={{width: "100%"}}
                expanded={!collapsed.includes(SlateSection.MEMBERS)}
                onChange={() => toggleSection(SlateSection.MEMBERS)}
            >
                {sectionSummary(SlateSection.MEMBERS, t("slateScreen.edit.members"))}
                <AccordionDetails>
                    <Typography
                        variant="body2"
                        sx={{color: "text.secondary", fontSize: "12px", marginBottom: "16px"}}
                    >
                        {t("slateScreen.members.helper")}
                    </Typography>
                    <Box sx={{overflowX: "auto", width: "100%"}}>
                        <Box
                            component="table"
                            sx={{
                                "borderCollapse": "collapse",
                                "width": "100%",
                                "maxWidth": "900px",
                                "& th": {
                                    textAlign: "left",
                                    padding: "8px 16px 16px",
                                    fontWeight: 700,
                                    fontSize: "14px",
                                },
                            }}
                        >
                            <thead>
                                <Box
                                    component="tr"
                                    sx={{borderBottom: "1px solid rgba(0, 0, 0, 0.12)"}}
                                >
                                    <th>{t("slateScreen.members.contest")}</th>
                                    <Box component="th" sx={{paddingLeft: "0 !important"}}>
                                        {t("slateScreen.members.candidate")}
                                    </Box>
                                </Box>
                            </thead>
                            <tbody>
                                {contests.map((contest) => (
                                    <ContestMembersRow
                                        key={contest.id}
                                        contest={contest}
                                        contestName={contestName(contest)}
                                        candidates={candidateOptions}
                                        config={config}
                                        slateId={slateId}
                                        disabled={!canEdit}
                                    />
                                ))}
                            </tbody>
                        </Box>
                    </Box>
                </AccordionDetails>
            </Accordion>
            {problems.length > 0 && (
                <Alert severity="error" sx={{width: "100%", marginTop: "1rem"}}>
                    <Box component="ul" sx={{margin: 0, paddingLeft: "1rem"}}>
                        {problems.map((message) => (
                            <li key={message}>{message}</li>
                        ))}
                    </Box>
                </Alert>
            )}
        </SimpleForm>
    )
}
