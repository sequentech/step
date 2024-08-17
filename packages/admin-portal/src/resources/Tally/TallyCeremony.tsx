// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useContext, useEffect, useMemo, useRef, useState} from "react"
import {
    BreadCrumbSteps,
    BreadCrumbStepsVariant,
    Dialog,
    DropFile,
    theme,
} from "@sequentech/ui-essentials"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import {useTranslation} from "react-i18next"
import ElectionHeader from "@/components/ElectionHeader"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {
    Accordion,
    AccordionSummary,
    SelectChangeEvent,
    MenuItem,
    Select,
    FormControl,
    Button,
    Box,
} from "@mui/material"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {ListActions} from "@/components/ListActions"
import {TallyElectionsList} from "./TallyElectionsList"
import {TallyTrusteesList} from "./TallyTrusteesList"
import {TallyStyles} from "@/components/styles/TallyStyles"
import {TallyStartDate} from "./TallyStartDate"
import {TallyElectionsProgress} from "./TallyElectionsProgress"
import {TallyElectionsResults} from "./TallyElectionsResults"
import {TallyResults} from "./TallyResults"
import {TallyLogs} from "./TallyLogs"
import {useGetList, useGetOne, useNotify, useRecordContext} from "react-admin"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {UPDATE_TALLY_CEREMONY} from "@/queries/UpdateTallyCeremony"
import {CREATE_TALLY_CEREMONY} from "@/queries/CreateTallyCeremony"
import {useMutation} from "@apollo/client"
import {ITallyExecutionStatus} from "@/types/ceremonies"
import {
    CreateTallyCeremonyMutation,
    CreateTransmissionPackageMutation,
    SendTransmissionPackageMutation,
    Sequent_Backend_Communication_Template,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
    Sequent_Backend_Results_Event,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
    UpdateTallyCeremonyMutation,
    UploadSignatureMutation,
} from "@/gql/graphql"
import {CancelButton, NextButton} from "./styles"
import {statusColor} from "./constants"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ExportElectionMenu} from "@/components/tally/ExportElectionMenu"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {IResultDocuments} from "@/types/results"
import {ResultsDataLoader} from "./ResultsDataLoader"
import {ICommunicationType} from "@/types/communications"
import {
    IMiruTallySessionData,
    IMiruTransmissionPackageData,
    MIRU_TALLY_SESSION_ANNOTATION_KEY,
} from "@/types/miru"
import {SEND_TRANSMISSION_PACKAGE} from "@/queries/SendTransmissionPackage"
import {IPermissions} from "@/types/keycloak"
import {UPLOAD_SIGNATURE} from "@/queries/UploadSignature"
import {MiruExportWizard} from "@/components/MiruExportWizard"
import {CREATE_TRANSMISSION_PACKAGE} from "@/queries/CreateTransmissionPackage"

const WizardSteps = {
    Start: 0,
    Ceremony: 1,
    Tally: 2,
    Results: 3,
    Export: 4,
}

export interface IExpanded {
    [key: string]: boolean
}

export const TallyCeremony: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t, i18n} = useTranslation()
    const {tallyId, setTallyId, setCreatingFlag} = useElectionEventTallyStore()
    const notify = useNotify()
    const {globalSettings} = useContext(SettingsContext)

    const [openModal, setOpenModal] = useState(false)
    const [openCeremonyModal, setOpenCeremonyModal] = useState(false)
    const [transmissionLoading, setTransmissionLoading] = useState<boolean>(false)
    const [page, setPage] = useState<number>(WizardSteps.Start)
    const [tally, setTally] = useState<Sequent_Backend_Tally_Session>()
    const [isButtonDisabled, setIsButtonDisabled] = useState<boolean>(true)
    const [templateId, setTemplateId] = useState<string | undefined>(undefined)
    const [isTallyElectionListDisabled, setIsTallyElectionListDisabled] = useState<boolean>(false)
    const [localTallyId, setLocalTallyId] = useState<string | null>(null)
    const [tenantId] = useTenantStore()

    const [selectedElections, setSelectedElections] = useState<string[]>([])
    const [selectedTrustees, setSelectedTrustees] = useState<boolean>(false)

    const [CreateTallyCeremonyMutation] =
        useMutation<CreateTallyCeremonyMutation>(CREATE_TALLY_CEREMONY)
    const [UpdateTallyCeremonyMutation] =
        useMutation<UpdateTallyCeremonyMutation>(UPDATE_TALLY_CEREMONY)

    const [SendTransmissionPackage] = useMutation<SendTransmissionPackageMutation>(
        SEND_TRANSMISSION_PACKAGE,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.TALLY_WRITE,
                },
            },
        }
    )

    const [uploadSignature] = useMutation<UploadSignatureMutation>(UPLOAD_SIGNATURE, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.TALLY_WRITE,
            },
        },
    })

    const [expandedData, setExpandedData] = useState<IExpanded>({
        "tally-data-progress": true,
        "tally-data-logs": true,
        "tally-data-general": false,
        "tally-data-results": false,
    })

    const [expandedResults, setExpandedResults] = useState<IExpanded>({
        "tally-results-progress": false,
        "tally-results-logs": true,
        "tally-results-general": true,
        "tally-results-results": true,
    })

    const [expandedExports, setExpandedDataExports] = useState<IExpanded>({
        "tally-miru-upload": false,
        "tally-miru-signatures": false,
        "tally-download-package": false,
        "tally-miru-servers": false,
    })

    const {data} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: localTallyId || tallyId,
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchIntervalInBackground: true,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: keyCeremony} = useGetList<Sequent_Backend_Keys_Ceremony>(
        "sequent_backend_keys_ceremony",
        {
            pagination: {page: 1, perPage: 9999},
            filter: {election_event_id: record?.id, tenant_id: tenantId},
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: tallySessionExecutions} = useGetList<Sequent_Backend_Tally_Session_Execution>(
        "sequent_backend_tally_session_execution",
        {
            pagination: {page: 1, perPage: 1},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tally_session_id: tallyId,
                tenant_id: tenantId,
            },
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    let resultsEventId = tallySessionExecutions?.[0]?.results_event_id ?? null

    const [selectedTallySessionData, setSelectedTallySessionData] =
        useState<IMiruTransmissionPackageData | null>(null)

    const tallySessionData = useMemo(() => {
        try {
            let strData = data?.annotations?.[MIRU_TALLY_SESSION_ANNOTATION_KEY]
            if (!strData) {
                return []
            }
            let parsed = JSON.parse(strData) as IMiruTallySessionData
            return parsed
        } catch (e) {
            return []
        }
    }, [data?.annotations?.[MIRU_TALLY_SESSION_ANNOTATION_KEY]])
    const tallySessionDataRef = useRef(tallySessionData)

    useEffect(() => {
        tallySessionDataRef.current = tallySessionData
    }, [tallySessionData])

    useEffect(() => {
        if (!selectedTallySessionData || !tallySessionData) {
            return
        }
        let found = tallySessionData.find(
            (el) =>
                el.area_id === selectedTallySessionData.area_id &&
                el.election_id === selectedTallySessionData.election_id
        )
        if (found && JSON.stringify(found) !== JSON.stringify(selectedTallySessionData)) {
            setSelectedTallySessionData(found ?? null)
        }
    }, [tallySessionData, selectedTallySessionData])

    const {data: resultsEvent, refetch} = useGetList<Sequent_Backend_Results_Event>(
        "sequent_backend_results_event",
        {
            pagination: {page: 1, perPage: 1},
            filter: {
                tenant_id: tenantId,
                election_event_id: record?.id,
                id: resultsEventId,
            },
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: tallyTemplates} = useGetList<Sequent_Backend_Communication_Template>(
        "sequent_backend_communication_template",
        {
            filter: {
                tenant_id: tenantId,
                communication_type: ICommunicationType.TALLY_REPORT,
            },
        }
    )

    useEffect(() => {
        if (data) {
            setPage(
                !tallyId && data.execution_status !== ITallyExecutionStatus.CANCELLED
                    ? WizardSteps.Start
                    : data.execution_status === ITallyExecutionStatus.STARTED ||
                      data.execution_status === ITallyExecutionStatus.CONNECTED ||
                      data.execution_status === ITallyExecutionStatus.CANCELLED
                    ? WizardSteps.Ceremony
                    : data.execution_status === ITallyExecutionStatus.IN_PROGRESS
                    ? WizardSteps.Tally
                    : data.execution_status === ITallyExecutionStatus.SUCCESS
                    ? WizardSteps.Results
                    : WizardSteps.Start
            )
            setTally(data)
        }
    }, [data])

    useEffect(() => {
        if (page === WizardSteps.Start) {
            setIsButtonDisabled(
                page === WizardSteps.Start && selectedElections.length === 0 ? true : false
            )
        }
    }, [selectedElections])

    useEffect(() => {
        if (page === WizardSteps.Ceremony) {
            setIsButtonDisabled(tally?.execution_status !== ITallyExecutionStatus.CONNECTED)
        }
        if (page === WizardSteps.Tally) {
            setIsButtonDisabled(tally?.execution_status !== ITallyExecutionStatus.SUCCESS)
        }
    }, [tally])

    const handleNext = () => {
        if (page === WizardSteps.Start) {
            setOpenModal(true)
        } else if (page === WizardSteps.Ceremony) {
            setOpenCeremonyModal(true)
        } else if (page === WizardSteps.Tally) {
            setPage(WizardSteps.Results)
        } else {
            setPage(page < 2 ? page + 1 : 0)
        }
    }

    const confirmStartAction = async () => {
        try {
            setIsButtonDisabled(true)
            setIsTallyElectionListDisabled(true)
            const {data, errors} = await CreateTallyCeremonyMutation({
                variables: {
                    tenant_id: record?.tenant_id,
                    election_event_id: record?.id,
                    keys_ceremony_id: keyCeremony?.[0]?.id,
                    election_ids: selectedElections,
                    configuration: {
                        report_content_template_id: templateId,
                    },
                },
            })

            if (errors || !data?.create_tally_ceremony) {
                notify(t("tally.createTallyError"), {type: "error"})
                return
            }

            if (data) {
                notify(t("tally.createTallySuccess"), {type: "success"})
                setLocalTallyId(data.create_tally_ceremony.tally_session_id)
                setTallyId(data.create_tally_ceremony.tally_session_id)
            }
        } catch (error) {
            notify(t("tally.startTallyCeremonyError"), {type: "error"})
        } finally {
            refetch()
            setIsButtonDisabled(false)
        }
    }

    const confirmCeremonyAction = async () => {
        try {
            const {data: nextStatus, errors} = await UpdateTallyCeremonyMutation({
                variables: {
                    election_event_id: record?.id,
                    tally_session_id: tallyId,
                    status: ITallyExecutionStatus.IN_PROGRESS,
                },
            })

            if (errors) {
                notify(t("tally.startTallyError"), {type: "error"})
                return
            }

            if (nextStatus) {
                notify(t("tally.startTallySuccess"), {type: "success"})
                setCreatingFlag(false)
            }
        } catch (error) {
            notify(t("tally.startTallyError"), {type: "error"})
        }
    }

    let documents: IResultDocuments | null = useMemo(
        () =>
            (!!resultsEventId &&
                !!resultsEvent &&
                resultsEvent?.[0]?.id === resultsEventId &&
                (resultsEvent[0]?.documents as IResultDocuments | null)) ||
            null,
        [resultsEventId, resultsEvent, resultsEvent?.[0]?.id]
    )
    const handleSetTemplate = (event: SelectChangeEvent) => setTemplateId(event.target.value)

    const handleMiruExportSuccess = (e: {
        election_id?: string
        area_id?: string
        existingPackage?: IMiruTransmissionPackageData
    }) => {
        //check for task completion and fetch data
        //set new page status(navigate to miru wizard)

        if (e.existingPackage) {
            setSelectedTallySessionData(e.existingPackage)
            setPage(WizardSteps.Export)
        } else {
            let packageData: IMiruTransmissionPackageData | null = null
            let retry = 0

            let intervalId = setInterval(() => {
                if (!!packageData || retry >= 5) {
                    notify(t("miruExport.create.error"), {type: "error"})
                    clearInterval(intervalId)
                    return
                }
                const found =
                    tallySessionDataRef.current?.find(
                        (datum) =>
                            datum.area_id === e.area_id && datum.election_id === e.election_id
                    ) ?? null

                if (found) {
                    packageData = found
                    clearInterval(intervalId)
                    setSelectedTallySessionData(packageData)
                    setPage(WizardSteps.Export)
                } else {
                    retry = retry + 1
                }
            }, globalSettings.QUERY_POLL_INTERVAL_MS)
        }
    }

    const handleSendTransmissionPackage = async () => {
        try {
            setTransmissionLoading(true)

            const {data: nextStatus, errors} = await SendTransmissionPackage({
                variables: {
                    electionId: selectedTallySessionData?.election_id,
                    tallySessionId: tallyId,
                    areaId: selectedTallySessionData?.area_id,
                },
            })

            if (errors) {
                setTransmissionLoading(false)
                notify(t("miruExport.send.error"), {type: "error"})
                return
            }

            if (nextStatus) {
                setTransmissionLoading(false)
                notify(t("miruExport.send.success"), {type: "success"})
                // onSuccess?.()
            }
        } catch (error) {
            console.log(`Caught error: ${error}`)
            notify(t("miruExport.send.error"), {type: "error"})
        }
    }

    const [uploading, setUploading] = useState<boolean>(false)
    const [errors, setErrors] = useState<String | null>(null)

    const handleUploadSignature = async (files: FileList | null) => {
        setErrors(null)
        setUploading(false)
        if (!files || files.length === 0) {
            setErrors("No file selected")
            return
        }
        const firstFile = files[0]
        const readFileContent = (file: File) => {
            return new Promise<string>((resolve, reject) => {
                const fileReader = new FileReader()
                fileReader.onload = () => resolve(fileReader.result as string)
                fileReader.onerror = (error) => reject(error)
                // Read the file as a data URL (base64 encoded string)
                fileReader.readAsText(file)
            })
        }
        try {
            const fileContent = await readFileContent(firstFile)
            console.log(`uploadPrivateKey(): fileContent: ${fileContent}`)
            if (fileContent == null) {
                setErrors(t("Error uploading signature"))
                return
            }
            setUploading(true)
            const {data, errors} = await uploadSignature({
                variables: {
                    electionId: selectedTallySessionData?.election_id,
                    tallySessionId: tally?.id,
                    areaId: selectedTallySessionData?.area_id,
                    signature: fileContent,
                },
            })
            setUploading(false)
            if (errors) {
                setErrors(t("tally.errorUploadingSignature", {error: errors.toString()}))
                return
            }
        } catch (exception: any) {
            setUploading(false)
            setErrors(t("keysGeneration.checkStep.errorUploading", {error: exception.toString()}))
        }
    }

    const [CreateTransmissionPackage] = useMutation<CreateTransmissionPackageMutation>(
        CREATE_TRANSMISSION_PACKAGE,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.TALLY_WRITE,
                },
            },
        }
    )

    const handleCreateTransmissionPackage = useCallback(
        async ({area_id, election_id}: {area_id: string; election_id: string | null}) => {
            const found = tallySessionData.find(
                (datum) => datum.area_id === area_id && datum.election_id === election_id
            )

            if (!election_id) {
                notify(t("miruExport.create.error"), {type: "error"})
                console.log("Unable to get election id.")
                return
            }

            if (found) {
                handleMiruExportSuccess?.({existingPackage: found})

                return
            }

            try {
                const {data: nextStatus, errors} = await CreateTransmissionPackage({
                    variables: {
                        electionId: election_id,
                        tallySessionId: tallyId,
                        areaId: area_id,
                    },
                })

                if (errors) {
                    notify(t("miruExport.create.error"), {type: "error"})
                    return
                }

                if (nextStatus) {
                    notify(t("miruExport.create.success"), {type: "success"})
                    handleMiruExportSuccess?.({area_id, election_id})
                }
            } catch (error) {
                console.log(`Caught error: ${error}`)
                notify(t("miruExport.create.error"), {type: "error"})
            }
        },
        [tallySessionData, tally]
    )

    return (
        <>
            <WizardStyles.WizardWrapper>
                <TallyStyles.StyledHeader>
                    <BreadCrumbSteps
                        labels={[
                            "tally.breadcrumbSteps.start",
                            "tally.breadcrumbSteps.ceremony",
                            "tally.breadcrumbSteps.tally",
                            "tally.breadcrumbSteps.results",
                        ]}
                        selected={page}
                        variant={BreadCrumbStepsVariant.Circle}
                        colorPreviousSteps={true}
                    />
                </TallyStyles.StyledHeader>

                {resultsEventId && record?.id ? (
                    <ResultsDataLoader
                        resultsEventId={resultsEventId}
                        electionEventId={record?.id}
                    />
                ) : null}
                {page === WizardSteps.Start && (
                    <>
                        <ElectionHeader
                            title={"tally.ceremonyTitle"}
                            subtitle={"tally.ceremonySubTitle"}
                        />

                        <TallyElectionsList
                            update={(elections) => setSelectedElections(elections)}
                            disabled={isTallyElectionListDisabled}
                            electionEventId={record?.id}
                        />
                        <FormControl fullWidth>
                            <ElectionHeader
                                title={"tally.templateTitle"}
                                subtitle={"tally.templateSubTitle"}
                            />

                            <Select
                                id="tally-results-template"
                                value={templateId}
                                label={t("tally.templateTitle")}
                                placeholder={t("tally.templateTitle")}
                                onChange={handleSetTemplate}
                            >
                                {(tallyTemplates ?? []).map((tallyTemplate) => (
                                    <MenuItem key={tallyTemplate.id} value={tallyTemplate.id}>
                                        {tallyTemplate.template?.name}
                                    </MenuItem>
                                ))}
                            </Select>
                        </FormControl>
                    </>
                )}

                {page === WizardSteps.Ceremony && (
                    <>
                        <TallyElectionsList
                            electionEventId={record?.id}
                            disabled={true}
                            update={(elections) => setSelectedElections(elections)}
                        />

                        <TallyTrusteesList
                            tally={tally}
                            update={(trustees) => {
                                setSelectedTrustees(trustees)
                            }}
                        />
                    </>
                )}

                {page === WizardSteps.Tally && (
                    <>
                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expandedData["tally-data-progress"]}
                            onChange={() =>
                                setExpandedData((prev: IExpanded) => ({
                                    ...prev,
                                    "tally-data-progress": !prev["tally-data-progress"],
                                }))
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="tally-data-progress" />}
                            >
                                <WizardStyles.AccordionTitle>
                                    {t("tally.tallyTitle")}
                                </WizardStyles.AccordionTitle>
                                <WizardStyles.CeremonyStatus
                                    sx={{
                                        backgroundColor: statusColor(
                                            tally?.execution_status ?? ITallyExecutionStatus.STARTED
                                        ),
                                        color: theme.palette.background.default,
                                    }}
                                    label={t("keysGeneration.ceremonyStep.executionStatus", {
                                        status: tally?.execution_status,
                                    })}
                                />
                            </AccordionSummary>
                            <WizardStyles.AccordionDetails>
                                <TallyElectionsProgress />
                            </WizardStyles.AccordionDetails>
                        </Accordion>

                        <TallyLogs tallySessionExecution={tallySessionExecutions?.[0]} />

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expandedResults["tally-data-general"]}
                            onChange={() =>
                                setExpandedResults((prev: IExpanded) => ({
                                    ...prev,
                                    "tally-data-general": !prev["tally-data-general"],
                                }))
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="tally-data-general" />}
                            >
                                <WizardStyles.AccordionTitle>
                                    {t("tally.generalInfoTitle")}
                                </WizardStyles.AccordionTitle>
                            </AccordionSummary>
                            <WizardStyles.AccordionDetails>
                                <TallyStartDate />
                                <TallyElectionsResults
                                    tenantId={tally?.tenant_id}
                                    electionEventId={tally?.election_event_id}
                                    electionIds={tally?.election_ids}
                                    resultsEventId={resultsEventId}
                                />
                            </WizardStyles.AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expandedData["tally-data-results"]}
                            onChange={() =>
                                setExpandedData((prev: IExpanded) => ({
                                    ...prev,
                                    "tally-data-results": !prev["tally-data-results"],
                                }))
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="tally-data-results" />}
                            >
                                <WizardStyles.AccordionTitle>
                                    {t("tally.resultsTitle")}
                                </WizardStyles.AccordionTitle>
                            </AccordionSummary>
                            <WizardStyles.AccordionDetails>
                                <TallyResults
                                    tally={tally}
                                    resultsEventId={resultsEventId}
                                    onCreateTransmissionPackage={handleCreateTransmissionPackage}
                                />
                            </WizardStyles.AccordionDetails>
                        </Accordion>
                    </>
                )}

                {page === WizardSteps.Results && (
                    <>
                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expandedData["tally-results-progress"]}
                            onChange={() =>
                                setExpandedData((prev: IExpanded) => ({
                                    ...prev,
                                    "tally-results-progress": !prev["tally-results-progress"],
                                }))
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="tally-results-progress" />}
                            >
                                <WizardStyles.AccordionTitle>
                                    {t("tally.tallyTitle")}
                                </WizardStyles.AccordionTitle>
                                <WizardStyles.CeremonyStatus
                                    sx={{
                                        backgroundColor: statusColor(
                                            tally?.execution_status ?? ITallyExecutionStatus.STARTED
                                        ),
                                        color: theme.palette.background.default,
                                    }}
                                    label={t("keysGeneration.ceremonyStep.executionStatus", {
                                        status: tally?.execution_status,
                                    })}
                                />
                            </AccordionSummary>
                            <WizardStyles.AccordionDetails>
                                <TallyElectionsProgress />
                            </WizardStyles.AccordionDetails>
                        </Accordion>

                        <TallyLogs tallySessionExecution={tallySessionExecutions?.[0]} />

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expandedResults["tally-results-general"]}
                            onChange={() =>
                                setExpandedResults((prev: IExpanded) => ({
                                    ...prev,
                                    "tally-results-general": !prev["tally-results-general"],
                                }))
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="tally-results-general" />}
                            >
                                <WizardStyles.AccordionTitle>
                                    {t("tally.generalInfoTitle")}
                                </WizardStyles.AccordionTitle>
                            </AccordionSummary>
                            <WizardStyles.AccordionDetails>
                                <TallyStartDate />
                                <TallyElectionsResults
                                    tenantId={tally?.tenant_id}
                                    electionEventId={tally?.election_event_id}
                                    electionIds={tally?.election_ids}
                                    resultsEventId={resultsEventId}
                                />
                            </WizardStyles.AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expandedResults["tally-results-results"]}
                            onChange={() =>
                                setExpandedResults((prev: IExpanded) => ({
                                    ...prev,
                                    "tally-results-results": !prev["tally-results-results"],
                                }))
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="tally-data-results" />}
                            >
                                <WizardStyles.AccordionTitle>
                                    {t("tally.resultsTitle")}
                                </WizardStyles.AccordionTitle>
                                <TallyStyles.StyledSpacing>
                                    {resultsEvent?.[0] && documents ? (
                                        <ExportElectionMenu
                                            documents={documents}
                                            electionEventId={resultsEvent?.[0].election_event_id}
                                            itemName={resultsEvent?.[0]?.name ?? "event"}
                                        />
                                    ) : null}
                                </TallyStyles.StyledSpacing>
                            </AccordionSummary>
                            <WizardStyles.AccordionDetails style={{zIndex: 100}}>
                                <TallyResults
                                    tally={tally}
                                    resultsEventId={resultsEventId}
                                    onCreateTransmissionPackage={handleCreateTransmissionPackage}
                                />
                            </WizardStyles.AccordionDetails>
                        </Accordion>
                    </>
                )}

                {page === WizardSteps.Export && (
                    <MiruExportWizard
                        expandedExports={expandedExports}
                        resultsEvent={resultsEvent}
                        setExpandedDataExports={setExpandedDataExports}
                        transmissionLoading={transmissionLoading}
                        handleSendTransmissionPackage={handleSendTransmissionPackage}
                        selectedTallySessionData={selectedTallySessionData}
                        uploading={uploading}
                        documents={documents}
                        errors={errors}
                        handleUploadSignature={handleUploadSignature}
                    />
                )}

                <TallyStyles.StyledFooter>
                    {page < WizardSteps.Results && (
                        <CancelButton
                            className="list-actions"
                            onClick={() => {
                                setTallyId(null)
                                setCreatingFlag(false)
                            }}
                        >
                            {t("tally.common.cancel")}
                        </CancelButton>
                    )}
                    {page < WizardSteps.Results &&
                        tally?.execution_status !== ITallyExecutionStatus.CANCELLED && (
                            <NextButton
                                color="primary"
                                onClick={handleNext}
                                disabled={isButtonDisabled}
                            >
                                <>
                                    {page === WizardSteps.Start
                                        ? t("tally.common.ceremony")
                                        : page === WizardSteps.Ceremony
                                        ? t("tally.common.start")
                                        : page === WizardSteps.Tally
                                        ? t("tally.common.results")
                                        : t("tally.common.next")}
                                    <ChevronRightIcon
                                        style={{
                                            transform:
                                                i18n.dir(i18n.language) === "rtl"
                                                    ? "rotate(180deg)"
                                                    : "rotate(0)",
                                        }}
                                    />
                                </>
                            </NextButton>
                        )}
                </TallyStyles.StyledFooter>
            </WizardStyles.WizardWrapper>

            <Dialog
                variant="info"
                open={openModal}
                ok={t("tally.common.dialog.ok")}
                cancel={t("tally.common.dialog.cancel")}
                title={t("tally.common.dialog.title")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmStartAction()
                    }
                    setOpenModal(false)
                }}
            >
                {t("tally.common.dialog.message")}
            </Dialog>

            <Dialog
                variant="info"
                open={openCeremonyModal}
                ok={t("tally.common.dialog.okTally")}
                cancel={t("tally.common.dialog.cancel")}
                title={t("tally.common.dialog.tallyTitle")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmCeremonyAction()
                    }
                    setOpenCeremonyModal(false)
                }}
            >
                {t("tally.common.dialog.ceremony")}
            </Dialog>
        </>
    )
}
