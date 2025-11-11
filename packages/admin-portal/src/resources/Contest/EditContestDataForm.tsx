// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    SelectInput,
    TextInput,
    useRecordContext,
    SimpleForm,
    useGetOne,
    Toolbar,
    SaveButton,
    FormDataConsumer,
    useGetList,
    useUpdate,
    useNotify,
    useRefresh,
    required,
    RaRecord,
    Identifier,
    RecordContext,
    BooleanInput,
    NumberInput,
} from "react-admin"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Tabs,
    Tab,
    Typography,
    Grid,
    Box,
} from "@mui/material"
import {
    GetUploadUrlMutation,
    Sequent_Backend_Candidate,
    Sequent_Backend_Contest,
    Sequent_Backend_Document,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
} from "../../gql/graphql"
import React, {ReactNode, useCallback, useContext, useEffect, useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import styled from "@emotion/styled"
import {cloneDeep} from "lodash"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "../../components/CustomTabPanel"
import {
    CandidatesOrder,
    EInvalidVotePolicy,
    EUnderVotePolicy,
    EEnableCheckableLists,
    IContestPresentation,
    IElectionEventPresentation,
    isArray,
    ICandidatePresentation,
    IElectionPresentation,
    EBlankVotePolicy,
    EOverVotePolicy,
    ECandidatesIconCheckboxPolicy,
} from "@sequentech/ui-core"
import {DropFile} from "@sequentech/ui-essentials"
import {ICountingAlgorithm, IVotingType} from "./constants"
import {ContestStyles} from "../../components/styles/ContestStyles"
import FileJsonInput from "../../components/FileJsonInput"
import {useMutation} from "@apollo/client"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {CandidateStyles} from "@/components/styles/CandidateStyles"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {CircularProgress} from "@mui/material"
import CustomOrderInput from "@/components/custom-order/CustomOrderInput"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

type FieldValues = Record<string, any>

const CandidateRows = styled.div`
    display: flex;
    flex-direction: column;
    width: 100%;
    cursor: pointer;
    margin-bottom: 0.1rem;
    padding: 1rem;
`

const ListWrapper = styled.div`
    display: flex;
    flex-direction: column;
    border-radius: 4px;
    border: 1px solid #777;
    padding: 8px;
    margin-bottom: 4px;
`

interface EnumChoice<T> {
    id: T
    name: string
}

export type Sequent_Backend_Contest_Extended = Sequent_Backend_Contest &
    RaRecord<Identifier> & {
        candidatesOrder?: Array<Sequent_Backend_Candidate>
    }

const uniqueArray = (arr: string[]): string[] => {
    // Create a Set to store unique elements
    const uniqueSet = new Set<string>()

    // Iterate through the array and add elements to the Set
    arr.forEach((item) => {
        uniqueSet.add(item)
    })

    // Convert the Set back to an array and return it
    return Array.from(uniqueSet)
}

interface IListsPresentationEditorProps {
    formData: FieldValues
    candidates?: Array<Sequent_Backend_Candidate>
    languageConf: Array<string>
}
const ListsPresentationEditor: React.FC<IListsPresentationEditorProps> = ({
    formData,
    candidates,
    languageConf,
}) => {
    const [value, setValue] = useState(0)
    const {t} = useTranslation()

    let types = candidates?.map((candidate) => candidate.type!!).filter((type) => type) ?? []
    types = uniqueArray(types)

    interface ISubtypeData {
        name: string
        candidates: Array<Sequent_Backend_Candidate>
    }
    interface ITypeData {
        name: string
        candidates: Array<Sequent_Backend_Candidate>
        subtypes: Array<ISubtypeData>
    }

    let typesMap: {[type: string]: ITypeData} = {}
    for (let type of types) {
        let filteredCandidates = candidates?.filter((candidate) => type === candidate.type) ?? []
        let subtypes = filteredCandidates
            ?.map(
                (candidate) =>
                    (candidate.presentation as ICandidatePresentation | undefined)?.subtype!!
            )
            .filter((subtype) => subtype)
        subtypes = uniqueArray(subtypes)

        let subtypesData = subtypes.map((subtype) => ({
            name: subtype,
            candidates: filteredCandidates.filter(
                (candidate) =>
                    subtype ===
                    (candidate.presentation as ICandidatePresentation | undefined)?.subtype
            ),
        }))

        typesMap[type] = {
            name: type,
            candidates: filteredCandidates,
            subtypes: subtypesData,
        }
    }

    const handleChange = (_event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue)
    }

    const renderTabs = () => {
        // reset actived tab to first tab if only one
        if (languageConf.length === 1 && value !== 0) {
            setValue(0)
        }

        return languageConf.map((lang) => (
            <Tab key={lang} label={String(t(`common.language.${lang}`))} id={lang}></Tab>
        ))
    }

    const renderTabContent = (type: string) => {
        let tabNodes: Array<ReactNode> = []
        let index = 0
        languageConf.forEach((lang) => {
            tabNodes.push(
                <CustomTabPanel key={lang} value={value} index={index}>
                    <Box style={{marginTop: "16px"}}>
                        <TextInput
                            source={`presentation.types_presentation[${type}].name_i18n[${lang}]`}
                            label="List Name"
                        />
                    </Box>
                </CustomTabPanel>
            )
            index++
        })
        return tabNodes
    }

    const renderSubtypeTabContent = (type: string, subtype: string) => {
        let tabNodes: Array<ReactNode> = []
        let index = 0
        languageConf.forEach((lang) => {
            tabNodes.push(
                <CustomTabPanel key={lang} value={value} index={index}>
                    <Box style={{marginTop: "16px"}}>
                        <TextInput
                            source={`presentation.types_presentation[${type}].subtypes_presentation[${subtype}].name_i18n[${lang}]`}
                            label="List Name"
                        />
                    </Box>
                </CustomTabPanel>
            )
            index++
        })
        return tabNodes
    }

    return (
        <>
            <Typography
                variant="body1"
                component="span"
                sx={{
                    padding: "0.5rem 1rem",
                    fontWeight: "bold",
                    margin: 0,
                    display: {xs: "none", sm: "block"},
                }}
            >
                Edit Lists
            </Typography>
            {types.map((type) => (
                <ListWrapper key={type}>
                    <i>List: {type}</i>
                    <NumberInput
                        source={`presentation.types_presentation[${type}].sort_order`}
                        min={0}
                        label="Sort order"
                    />
                    <Tabs value={value} onChange={handleChange}>
                        {renderTabs()}
                    </Tabs>
                    {renderTabContent(type)}
                    <Typography
                        variant="body1"
                        component="span"
                        sx={{
                            padding: "0.5rem 1rem",
                            fontWeight: "bold",
                            margin: 0,
                            display: {xs: "none", sm: "block"},
                        }}
                    >
                        Edit Subtypes
                    </Typography>
                    <Box>
                        {typesMap[type]?.subtypes.map((subtype) => {
                            return (
                                <ListWrapper key={subtype.name}>
                                    Subtype {subtype.name}
                                    <NumberInput
                                        source={`presentation.types_presentation[${type}].subtypes_presentation[${subtype.name}].sort_order`}
                                        min={0}
                                        label="Sort order"
                                    />
                                    <Tabs value={value} onChange={handleChange}>
                                        {renderTabs()}
                                    </Tabs>
                                    {renderSubtypeTabContent(type, subtype.name)}
                                </ListWrapper>
                            )
                        })}
                    </Box>
                </ListWrapper>
            ))}
        </>
    )
}

export const ContestDataForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Contest>()

    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const [languageConf, setLanguageConf] = useState<Array<string>>(["en"])
    const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
    const notify = useNotify()
    const refresh = useRefresh()
    const authContext = useContext(AuthContext)

    const [value, setValue] = useState(0)
    const [expanded, setExpanded] = useState("contest-data-general")

    const canEdit = authContext.isAuthorized(true, authContext.tenantId, IPermissions.CONTEST_WRITE)

    const {data: electionEvent} = useGetOne<Sequent_Backend_Election_Event>(
        "sequent_backend_election_event",
        {
            id: record?.election_event_id,
            meta: {tenant_id: record?.tenant_id},
        }
    )

    const {data: election} = useGetOne<Sequent_Backend_Election>("sequent_backend_election", {
        id: record?.election_id,
    })

    const {data: imageData, refetch: refetchImage} = useGetOne<Sequent_Backend_Document>(
        "sequent_backend_document",
        {
            id: record?.image_document_id || record?.tenant_id,
            meta: {tenant_id: record?.tenant_id},
        }
    )

    const [updateImage] = useUpdate()

    const {data: candidates} = useGetList<Sequent_Backend_Candidate>("sequent_backend_candidate", {
        filter: {
            contest_id: record?.id,
            tenant_id: record?.tenant_id,
            election_event_id: record?.election_event_id,
        },
        pagination: {page: 1, perPage: 500},
    })

    useEffect(() => {
        if (election) {
            let langConf = (election.presentation as IElectionPresentation | undefined)
                ?.language_conf
            if (langConf?.enabled_language_codes) {
                setLanguageConf(langConf?.enabled_language_codes)
                return
            }
        }
        if (electionEvent) {
            let langConf = (electionEvent.presentation as IElectionEventPresentation | undefined)
                ?.language_conf
            if (langConf?.enabled_language_codes) {
                setLanguageConf(langConf?.enabled_language_codes)
                return
            }
        }
    }, [electionEvent?.presentation?.language_conf, election?.presentation?.language_conf])

    const votingTypesChoices = (): Array<EnumChoice<IVotingType>> => {
        return Object.values(IVotingType).map((value) => ({
            id: value,
            name: t(`contestScreen.options.${value.toLowerCase()}`),
        }))
    }

    const countingAlgorithmChoices = (): Array<EnumChoice<ICountingAlgorithm>> => {
        return Object.values(ICountingAlgorithm).map((value) => ({
            id: value,
            name: t(`contestScreen.options.${value.toLowerCase()}`),
        }))
    }

    const orderAnswerChoices = (): Array<EnumChoice<CandidatesOrder>> => {
        return Object.values(CandidatesOrder).map((value) => ({
            id: value,
            name: t(`contestScreen.options.${value.toLowerCase()}`),
        }))
    }

    const invalidVotePolicyChoices = (): Array<EnumChoice<EInvalidVotePolicy>> => {
        return Object.values(EInvalidVotePolicy).map((value) => ({
            id: value,
            name: t(`contestScreen.invalidVotePolicy.${value.toLowerCase()}`),
        }))
    }

    const candidatesIconCheckboxPolicy = (): Array<EnumChoice<ECandidatesIconCheckboxPolicy>> => {
        return Object.values(ECandidatesIconCheckboxPolicy).map((value) => ({
            id: value,
            name: t(`contestScreen.candidatesIconCheckboxPolicy.${value.toLowerCase()}`),
        }))
    }

    const underVotePolicyChoices = (): Array<EnumChoice<EUnderVotePolicy>> => {
        return Object.values(EUnderVotePolicy).map((value) => ({
            id: value,
            name: t(`contestScreen.underVotePolicy.${value.toLowerCase()}`),
        }))
    }

    const checkableListChoices = (): Array<EnumChoice<EEnableCheckableLists>> => {
        return Object.values(EEnableCheckableLists).map((value) => ({
            id: value,
            name: t(`contestScreen.checkableListPolicy.${value.toLowerCase()}`),
        }))
    }

    const blankVotePolicyChoices = () => {
        return Object.values(EBlankVotePolicy).map((value) => ({
            id: value,
            name: t(`contestScreen.blankVotePolicy.${value}`),
        }))
    }

    const overVotePolicyChoices = () => {
        return Object.values(EOverVotePolicy).map((value) => ({
            id: value,
            name: t(`contestScreen.overVotePolicy.${value}`),
        }))
    }

    const parseValues = useCallback(
        (incoming: Sequent_Backend_Contest_Extended): Sequent_Backend_Contest_Extended => {
            if (!electionEvent) {
                return incoming
            }
            const newContest: Sequent_Backend_Contest_Extended = cloneDeep(incoming)
            const newPresentation = (newContest.presentation ?? {}) as IContestPresentation

            newContest.presentation = newPresentation
            // name, alias and description fields
            if (!newContest.presentation) {
                newContest.presentation = {}
            }
            if (!newContest.presentation.i18n) {
                newContest.presentation.i18n = {}
            }
            if (!newContest.presentation.i18n.en) {
                newContest.presentation.i18n.en = {}
            }
            if (!newContest.presentation.i18n.en.name && newContest.name) {
                newContest.presentation.i18n.en.name = newContest.name
            }
            if (!newContest.presentation.i18n.en.name && newContest.name) {
                newContest.presentation.i18n.en.name = newContest.name
            }
            if (!newContest.presentation.i18n.en.alias && newContest.alias) {
                newContest.presentation.i18n.en.alias = newContest.alias
            }
            if (!newContest.presentation.i18n.en.description && newContest.description) {
                newContest.presentation.i18n.en.description = newContest.description
            }
            newContest.name = newContest.presentation.i18n.en.name
            newContest.alias = newContest.presentation.i18n.en.alias
            newContest.description = newContest.presentation.i18n.en.description

            // defaults
            newContest.voting_type = newContest.voting_type || IVotingType.NON_PREFERENTIAL
            newContest.counting_algorithm =
                newContest.counting_algorithm || ICountingAlgorithm.PLURALITY_AT_LARGE
            newContest.min_votes = newContest.min_votes || 0

            newContest.presentation.candidates_order =
                newContest.presentation.candidates_order || CandidatesOrder.ALPHABETICAL

            newContest.presentation.invalid_vote_policy =
                newContest.presentation.invalid_vote_policy || EInvalidVotePolicy.ALLOWED

            newContest.presentation.enable_checkable_lists =
                newContest.presentation.enable_checkable_lists ||
                EEnableCheckableLists.CANDIDATES_AND_LISTS

            newContest.presentation.candidates_icon_checkbox_policy =
                newContest.presentation.candidates_icon_checkbox_policy ||
                ECandidatesIconCheckboxPolicy.SQUARE_CHECKBOX

            newContest.presentation.under_vote_policy =
                newContest.presentation.under_vote_policy || EUnderVotePolicy.ALLOWED

            newContest.presentation.blank_vote_policy =
                newContest.presentation.blank_vote_policy || EBlankVotePolicy.ALLOWED

            newContest.presentation.over_vote_policy =
                newContest.presentation.over_vote_policy || EOverVotePolicy.ALLOWED

            newContest.presentation.pagination_policy =
                newContest.presentation.pagination_policy || ""

            return newContest
        },
        [languageConf, electionEvent, candidates]
    )

    const handleChange = (_event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue)
    }

    const formValidator = (values: any): any => {
        const errors: any = {dates: {}}

        if (values?.dates?.end_date <= values?.dates?.start_date) {
            errors.dates.end_date = t("electionEventScreen.error.endDate")
        }

        return errors
    }

    const renderTabs = () => {
        let tabNodes: Array<ReactNode> = []

        languageConf.forEach((lang) => {
            tabNodes.push(<Tab key={lang} label={String(t(`common.language.${lang}`))} id={lang}></Tab>)
        })

        // reset actived tab to first tab if only one
        if (tabNodes.length === 1 && value !== 0) {
            setValue(0)
        }

        return tabNodes
    }

    const renderTabContent = () => {
        let tabNodes: Array<ReactNode> = []
        let index = 0
        languageConf.forEach((lang) => {
            tabNodes.push(
                <CustomTabPanel key={lang} value={value} index={index}>
                    <div style={{marginTop: "16px"}}>
                        <TextInput
                            source={`presentation.i18n[${lang}].name`}
                            label={String(t("electionEventScreen.field.name"))}
                        />
                        <TextInput
                            source={`presentation.i18n[${lang}].alias`}
                            label={String(t("electionEventScreen.field.alias"))}
                        />
                        <TextInput
                            source={`presentation.i18n[${lang}].description`}
                            label={String(t("electionEventScreen.field.description"))}
                        />
                    </div>
                </CustomTabPanel>
            )
            index++
        })
        return tabNodes
    }

    const handleFiles = async (files: FileList | null) => {
        // https://fullstackdojo.medium.com/s3-upload-with-presigned-url-react-and-nodejs-b77f348d54cc

        const theFile = files?.[0]

        if (theFile) {
            let {data, errors} = await getUploadUrl({
                variables: {
                    name: theFile.name,
                    media_type: theFile.type,
                    size: theFile.size,
                },
            })
            if (data?.get_upload_url?.document_id) {
                try {
                    await fetch(data.get_upload_url.url, {
                        method: "PUT",
                        headers: {
                            "Content-Type": "image/*",
                        },
                        body: theFile,
                    })
                    notify(t("electionScreen.error.fileLoaded"), {type: "success"})

                    updateImage("sequent_backend_contest", {
                        id: record?.id,
                        data: {
                            image_document_id: data.get_upload_url.document_id,
                        },
                    })

                    refetchImage()
                    refresh()
                } catch (e) {
                    notify(t("electionScreen.error.fileError"), {type: "error"})
                }
            } else {
                notify(t("electionScreen.error.fileError"), {type: "error"})
            }
        }
    }

    const sortedCandidates = (candidates ?? []).sort((a, b) => {
        let presentationA = a.presentation as ICandidatePresentation | undefined
        let presentationB = b.presentation as ICandidatePresentation | undefined
        let sortOrderA = presentationA?.sort_order ?? -1
        let sortOrderB = presentationB?.sort_order ?? -1
        return sortOrderA - sortOrderB
    })

    return electionEvent && isArray(candidates) ? (
        <RecordContext.Consumer>
            {(incoming) => {
                const parsedValue = parseValues(incoming as Sequent_Backend_Contest_Extended)

                return (
                    <SimpleForm
                        defaultValues={{candidatesOrder: sortedCandidates}}
                        validate={formValidator}
                        record={parsedValue}
                        toolbar={<Toolbar>{canEdit && <SaveButton />}</Toolbar>}
                    >
                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "contest-data-general"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "contest-data-general" ? "" : "contest-data-general"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="contest-data-general" />}
                            >
                                <ContestStyles.Wrapper>
                                    <ContestStyles.Title>
                                        {t("contestScreen.edit.general")}
                                    </ContestStyles.Title>
                                </ContestStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <Tabs value={value} onChange={handleChange}>
                                    {renderTabs()}
                                </Tabs>
                                {renderTabContent()}
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "contest-data-system"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "contest-data-system" ? "" : "contest-data-system"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="contest-data-system" />}
                            >
                                <ContestStyles.Wrapper>
                                    <ContestStyles.Title>
                                        {t("contestScreen.edit.system")}
                                    </ContestStyles.Title>
                                </ContestStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <SelectInput
                                    source="voting_type"
                                    choices={votingTypesChoices()}
                                    validate={required()}
                                />
                                <SelectInput
                                    source="counting_algorithm"
                                    choices={countingAlgorithmChoices()}
                                    validate={required()}
                                />
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "contest-data-design"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "contest-data-design" ? "" : "contest-data-design"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="contest-data-design" />}
                            >
                                <ContestStyles.Wrapper>
                                    <ContestStyles.Title>
                                        {t("contestScreen.edit.design")}
                                    </ContestStyles.Title>
                                </ContestStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <BooleanInput source="is_acclaimed" />
                                <NumberInput source="min_votes" min={0} />
                                <NumberInput source="max_votes" min={0} />
                                <NumberInput source="presentation.columns" min={1} />
                                <NumberInput source="winning_candidates_num" min={0} />
                                <SelectInput
                                    source="presentation.candidates_order"
                                    choices={orderAnswerChoices()}
                                    validate={required()}
                                />
                                <FormDataConsumer>
                                    {({formData, ...rest}) => {
                                        return (
                                            formData?.presentation as
                                                | IContestPresentation
                                                | undefined
                                        )?.candidates_order === CandidatesOrder.CUSTOM ? (
                                            <CandidateRows>
                                                <Typography
                                                    variant="body1"
                                                    component="span"
                                                    sx={{
                                                        padding: "0.5rem 1rem",
                                                        fontWeight: "bold",
                                                        margin: 0,
                                                        display: {xs: "none", sm: "block"},
                                                    }}
                                                >
                                                    {t("contestScreen.edit.reorder")}
                                                </Typography>
                                                <CustomOrderInput source="candidatesOrder" />
                                                <Box sx={{width: "100%", height: "180px"}}></Box>
                                            </CandidateRows>
                                        ) : null
                                    }}
                                </FormDataConsumer>
                                <FormDataConsumer>
                                    {({formData, ...rest}) => (
                                        <ListsPresentationEditor
                                            formData={formData}
                                            candidates={candidates}
                                            languageConf={languageConf}
                                        />
                                    )}
                                </FormDataConsumer>

                                <SelectInput
                                    source="presentation.enable_checkable_lists"
                                    choices={checkableListChoices()}
                                    validate={required()}
                                />

                                <NumberInput
                                    source="presentation.max_selections_per_type"
                                    min={0}
                                    isRequired={false}
                                />

                                <Typography
                                    variant="body1"
                                    component="span"
                                    sx={{
                                        padding: "0.5rem 1rem",
                                        fontWeight: "bold",
                                        margin: 0,
                                        display: {xs: "none", sm: "block"},
                                    }}
                                >
                                    {t("contestScreen.edit.policies")}
                                </Typography>

                                <SelectInput
                                    source="presentation.under_vote_policy"
                                    choices={underVotePolicyChoices()}
                                    label={String(t(`contestScreen.underVotePolicy.label`))}
                                    validate={required()}
                                />

                                <SelectInput
                                    source="presentation.invalid_vote_policy"
                                    choices={invalidVotePolicyChoices()}
                                    label={String(t(`contestScreen.invalidVotePolicy.label`))}
                                    validate={required()}
                                />

                                <SelectInput
                                    source={`presentation.blank_vote_policy`}
                                    choices={blankVotePolicyChoices()}
                                    label={String(t(`contestScreen.blankVotePolicy.label`))}
                                    defaultValue={EBlankVotePolicy.ALLOWED}
                                    validate={required()}
                                />

                                <SelectInput
                                    source={`presentation.over_vote_policy`}
                                    choices={overVotePolicyChoices()}
                                    label={String(t(`contestScreen.overVotePolicy.label`))}
                                    defaultValue={EOverVotePolicy.ALLOWED}
                                    validate={required()}
                                />

                                <SelectInput
                                    source={`presentation.candidates_icon_checkbox_policy`}
                                    choices={candidatesIconCheckboxPolicy()}
                                    label={String(t(`contestScreen.candidatesIconCheckboxPolicy.label`))}
                                    defaultValue={ECandidatesIconCheckboxPolicy.SQUARE_CHECKBOX}
                                    validate={required()}
                                />

                                <TextInput
                                    source={`presentation.pagination_policy`}
                                    label={String(t(`contestScreen.paginationPolicy.label`))}
                                />
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "contest-data-image"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "contest-data-image" ? "" : "contest-data-image"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="contest-data-image" />}
                            >
                                <CandidateStyles.Wrapper>
                                    <CandidateStyles.Title>
                                        {t("electionScreen.edit.image")}
                                    </CandidateStyles.Title>
                                </CandidateStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <Grid container spacing={1}>
                                    <Grid size={2}>
                                        {parsedValue?.image_document_id &&
                                        parsedValue?.image_document_id !== "" ? (
                                            <img
                                                width={200}
                                                height={200}
                                                src={`${globalSettings.PUBLIC_BUCKET_URL}tenant-${parsedValue?.tenant_id}/document-${parsedValue?.image_document_id}/${imageData?.name}`}
                                                alt={`tenant-${parsedValue?.tenant_id}/document-${parsedValue?.image_document_id}/${imageData?.name}`}
                                            />
                                        ) : null}
                                    </Grid>
                                    <Grid size={10}>
                                        <DropFile
                                            handleFiles={async (files) => handleFiles(files)}
                                        />
                                    </Grid>
                                </Grid>
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-data-advanced"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "election-data-advanced"
                                        ? ""
                                        : "election-data-advanced"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-advanced" />}
                            >
                                <ContestStyles.Wrapper>
                                    <ContestStyles.Title>
                                        {t("electionScreen.edit.advanced")}
                                    </ContestStyles.Title>
                                </ContestStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <FileJsonInput
                                    parsedValue={parsedValue}
                                    fileSource="configuration"
                                    jsonSource="presentation"
                                />
                            </AccordionDetails>
                        </Accordion>
                    </SimpleForm>
                )
            }}
        </RecordContext.Consumer>
    ) : (
        <CircularProgress />
    )
}
