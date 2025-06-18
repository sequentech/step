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
    CircularProgress,
    styled,
    Alert,
} from "@mui/material"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import CellTowerIcon from "@mui/icons-material/CellTower"
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
import {useMutation, useQuery} from "@apollo/client"
import {ETallyType, ITallyExecutionStatus} from "@/types/ceremonies"
import {
    EAllowTally,
    EInitializeReportPolicy,
    EInitReport,
    EVotingStatus,
    isArray,
} from "@sequentech/ui-core"

import {
    CreateTallyCeremonyMutation,
    CreateTransmissionPackageMutation,
    SendTransmissionPackageMutation,
    Sequent_Backend_Area,
    Sequent_Backend_Template,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
    Sequent_Backend_Results_Event,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
    UpdateTallyCeremonyMutation,
    ListKeysCeremonyQuery,
    Sequent_Backend_Contest,
} from "@/gql/graphql"
import {CancelButton, NextButton} from "./styles"
import {statusColor} from "./constants"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ExportElectionMenu, IResultDocumentsData} from "@/components/tally/ExportElectionMenu"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {IResultDocuments} from "@/types/results"
import {ResultsDataLoader} from "./ResultsDataLoader"
import {
    IMiruTallySessionData,
    IMiruTransmissionPackageData,
    MIRU_TALLY_SESSION_ANNOTATION_KEY,
} from "@/types/miru"
import {IPermissions} from "@/types/keycloak"
import {CREATE_TRANSMISSION_PACKAGE} from "@/queries/CreateTransmissionPackage"
import {useAtomValue} from "jotai"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {AuthContext} from "@/providers/AuthContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {LIST_KEYS_CEREMONY} from "@/queries/ListKeysCeremonies"
import {useKeysPermissions} from "../ElectionEvent/useKeysPermissions"

const WizardSteps = {
    Start: 0,
    Ceremony: 1,
    Tally: 2,
    Results: 3,
    Export: 4,
}

const StyledCircularProgress = styled(CircularProgress)`
    width: 14px !important;
    height: 14px !important;
`
export interface IExpanded {
    [key: string]: boolean
}

export const TallyCeremony: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t, i18n} = useTranslation()
    const {
        tallyId,
        setTallyId,
        isCreatingType,
        setCreatingFlag,
        setElectionEventIdFlag,
        setMiruAreaId,
        selectedTallySessionData,
        setSelectedTallySessionData,
    } = useElectionEventTallyStore()
    const notify = useNotify()
    const {globalSettings} = useContext(SettingsContext)

    // const [selectedTallySessionData, setSelectedTallySessionData] =
    // useState<IMiruTransmissionPackageData | null>(null)
    const [openModal, setOpenModal] = useState(false)
    const [confirmSendMiruModal, setConfirmSendMiruModal] = useState(false)
    const [openCeremonyModal, setOpenCeremonyModal] = useState(false)
    const [transmissionLoading, setTransmissionLoading] = useState<boolean>(false)
    const [page, setPage] = useState<number>(WizardSteps.Start)
    const [pristine, setPristine] = useState<boolean>(true)
    const [tally, setTally] = useState<Sequent_Backend_Tally_Session>()
    const [isButtonDisabled, setIsButtonDisabled] = useState<boolean>(true)
    const [templateId, setTemplateId] = useState<string | undefined>(undefined)
    const [isTallyElectionListDisabled, setIsTallyElectionListDisabled] = useState<boolean>(false)
    const [localTallyId, setLocalTallyId] = useState<string | null>(null)
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const isTrustee = authContext.isAuthorized(true, tenantId, IPermissions.TRUSTEE_CEREMONY)
    const [selectedElections, setSelectedElections] = useState<string[]>([])
    const [selectedTrustees, setSelectedTrustees] = useState<boolean>(false)
    const [keysCeremonyId, setKeysCeremonyId] = useState<string | undefined>(undefined)
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [isTallyCompleted, setIsTallyCompleted] = useState<boolean>(false)
    const [isConfirming, setIsConfirming] = useState<boolean>(false)

    const [CreateTallyCeremonyMutation] =
        useMutation<CreateTallyCeremonyMutation>(CREATE_TALLY_CEREMONY)
    const [UpdateTallyCeremonyMutation] =
        useMutation<UpdateTallyCeremonyMutation>(UPDATE_TALLY_CEREMONY)

    const {canExportCeremony, showTallyBackButton} = useKeysPermissions()

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

    const {data: tallySession, refetch: refetchTallySession} =
        useGetOne<Sequent_Backend_Tally_Session>(
            "sequent_backend_tally_session",
            {
                id: localTallyId || tallyId,
            },
            {
                refetchInterval: isTallyCompleted
                    ? undefined
                    : globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
                refetchIntervalInBackground: true,
                refetchOnWindowFocus: false,
                refetchOnReconnect: false,
                refetchOnMount: false,
                enabled: !!localTallyId || !!tallyId,
            }
        )

    const {data: keysCeremonies} = useQuery<ListKeysCeremonyQuery>(LIST_KEYS_CEREMONY, {
        variables: {
            tenantId: tenantId,
            electionEventId: record?.id,
        },
        context: {
            headers: {
                "x-hasura-role": isTrustee
                    ? IPermissions.TRUSTEE_CEREMONY
                    : IPermissions.ADMIN_CEREMONY,
            },
        },
    })

    // TODO: fix the "perPage 9999"
    const {data: elections} = useGetList<Sequent_Backend_Election>("sequent_backend_election", {
        pagination: {page: 1, perPage: 9999},
        filter: {
            election_event_id: record?.id,
            tenant_id: tenantId,
            id: tallySession
                ? {
                      format: "hasura-raw-query",
                      value: {
                          _in: tallySession?.election_ids ?? [],
                      },
                  }
                : undefined,
        },
    })

    const {data: contests} = useGetList<Sequent_Backend_Contest>("sequent_backend_contest", {
        pagination: {page: 1, perPage: 9999},
        filter: {
            election_event_id: record?.id,
            tenant_id: tenantId,
            election_id: {
                format: "hasura-raw-query",
                value: {
                    _in: tallySession?.election_ids ?? [],
                },
            },
        },
    })

    const {data: allTallySessions} = useGetList<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            pagination: {page: 1, perPage: 1},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                election_event_id: record?.id,
                tenant_id: tenantId,
            },
        },
        {
            refetchInterval: isTallyCompleted
                ? undefined
                : globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
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
            refetchInterval: isTallyCompleted
                ? undefined
                : globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
            enabled: !!tallyId && !!tenantId,
        }
    )

    let resultsEventId = tallySessionExecutions?.[0]?.results_event_id ?? null

    const resultsSQLiteDocumentId = tallySessionExecutions?.[0]?.documents?.sqlite ?? null

    const tallySessionData = useMemo(() => {
        try {
            let strData = tallySession?.annotations?.[MIRU_TALLY_SESSION_ANNOTATION_KEY]
            if (!strData) {
                return []
            }
            let parsed = JSON.parse(strData) as IMiruTallySessionData
            return parsed
        } catch (e) {
            return []
        }
    }, [tallySession?.annotations?.[MIRU_TALLY_SESSION_ANNOTATION_KEY]])
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
            setElectionEventIdFlag(record?.id)
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
            enabled: !!tenantId && record?.id && !!resultsEventId,
        }
    )

    const [hasFinalResults, setHasFinalResults] = useState(false)

    useEffect(() => {
        if (tallySession?.is_execution_completed && !isTallyCompleted) {
            // Only mark as completed if we have the resultsEventId
            if (resultsEventId) {
                setIsTallyCompleted(true)
                setHasFinalResults(true)
            } else {
                // Force a refetch if we don't have resultsEventId yet
                refetchTallySession()
            }
        }
    }, [tallySession?.is_execution_completed, isTallyCompleted, resultsEventId])

    useEffect(() => {
        // Additional check in case resultsEventId comes after is_execution_completed
        if (tallySession?.is_execution_completed && resultsEventId && !hasFinalResults) {
            setIsTallyCompleted(true)
            setHasFinalResults(true)
        }
    }, [resultsEventId, tallySession?.is_execution_completed, hasFinalResults])

    useEffect(() => {
        if (tallySession) {
            setTally(tallySession)
            if (!tallyId && tallySession.execution_status !== ITallyExecutionStatus.CANCELLED) {
                setPage(WizardSteps.Start)
                return
            }
            if (
                tallySession.execution_status === ITallyExecutionStatus.STARTED ||
                tallySession.execution_status === ITallyExecutionStatus.CONNECTED ||
                tallySession.execution_status === ITallyExecutionStatus.CANCELLED
            ) {
                setPage(WizardSteps.Ceremony)
                return
            }
            if (tallySession.execution_status === ITallyExecutionStatus.IN_PROGRESS) {
                setPage(WizardSteps.Tally)
                return
            }
            if (tallySession.execution_status === ITallyExecutionStatus.SUCCESS) {
                setPage(WizardSteps.Results)
                return
            }
            setPage(WizardSteps.Start)
        }
    }, [tallySession])

    useEffect(() => {
        if (page === WizardSteps.Start) {
            let is_published = elections?.every(
                (election) =>
                    !selectedElections.includes(election.id) || election.status?.is_published
            )
            let newIsButtonDisabled =
                (page === WizardSteps.Start && selectedElections.length === 0 ? true : false) ||
                !is_published
            setIsButtonDisabled(newIsButtonDisabled)
        }
    }, [selectedElections])

    const isTallyAllowed = useMemo(() => {
        return (
            elections?.every((election) => {
                // Check if the voting period has ended for the election AND kiosk voting has also ended or disabled
                const isVotingPeriodEnded =
                    election.status?.voting_status === EVotingStatus.CLOSED &&
                    (!election.voting_channels?.kiosk ||
                        election.status?.kiosk_voting_status === EVotingStatus.CLOSED)

                return (
                    // If the election is not included in the current tally session, it's allowed
                    !(tallySession?.election_ids || []).find(
                        (election_id) => election.id == election_id
                    ) ||
                    // Otherwise, tallying is allowed if it is explicitly permitted OR if it requires the voting period to end and it has ended
                    ((election.status?.allow_tally === EAllowTally.ALLOWED ||
                        (election.status?.allow_tally === EAllowTally.REQUIRES_VOTING_PERIOD_END &&
                            isVotingPeriodEnded)) &&
                        // And the election must be published
                        election.status.is_published)
                )
            }) || false // Return `false` if elections array is undefined or empty
        )
    }, [elections, tallySession])

    const isInitAllowed = useMemo(() => {
        return (
            elections?.every((election) => {
                return (
                    // If the election is not included in the current tally session OR the election is published, it's allowed
                    !(tallySession?.election_ids || []).find(
                        (election_id) => election.id == election_id
                    ) || election.status.is_published
                )
            }) || false // Return `false` if elections array is undefined or empty
        )
    }, [elections, tallySession])

    useEffect(() => {
        if (page === WizardSteps.Ceremony) {
            let isStartAllowed =
                tallySession?.tally_type === ETallyType.ELECTORAL_RESULTS
                    ? isTallyAllowed
                    : isInitAllowed
            let newIsButtonDisabled =
                tally?.execution_status !== ITallyExecutionStatus.CONNECTED || !isStartAllowed
            console.log(
                `setIsButtonDisabled = ${newIsButtonDisabled}, tallySession?.tally_type = ${tallySession?.tally_type}`
            )
            if (newIsButtonDisabled !== isButtonDisabled) {
                setIsButtonDisabled(newIsButtonDisabled)
            }
        }

        if (page === WizardSteps.Tally) {
            let newIsButtonDisabled = tally?.execution_status !== ITallyExecutionStatus.SUCCESS

            if (newIsButtonDisabled !== isButtonDisabled) {
                setIsButtonDisabled(newIsButtonDisabled)
            }
        }
    }, [tally, page, elections, isButtonDisabled, isTallyAllowed])

    useEffect(() => {
        let singleKeysCeremony = keysCeremonies?.list_keys_ceremony?.items?.[0]
        if (!pristine || keysCeremonyId || !singleKeysCeremony) {
            return
        }
        setKeysCeremonyId(singleKeysCeremony.id)
    }, [pristine, keysCeremonies?.list_keys_ceremony?.items, keysCeremonyId])

    useEffect(() => {
        if (isCreatingType === ETallyType.INITIALIZATION_REPORT) {
            // An initialization report is considered succesfully created if:
            // 1. It's not in CANCELLED status.
            // 2. It's in a cancellable status or successful. Cancellable status
            //    are: NOT_STARTED, STARTED && CONNECTED.
            const hasInitializationReport = (electionId: string) => {
                return allTallySessions
                    ?.filter((tallySession) => tallySession.election_ids?.includes(electionId))
                    .some(
                        (tallySession) =>
                            tallySession?.tally_type === ETallyType.INITIALIZATION_REPORT &&
                            tallySession?.execution_status &&
                            tallySession.execution_status !== ITallyExecutionStatus.CANCELLED &&
                            [
                                ITallyExecutionStatus.SUCCESS,
                                ITallyExecutionStatus.NOT_STARTED,
                                ITallyExecutionStatus.STARTED,
                                ITallyExecutionStatus.CONNECTED,
                            ].includes(tallySession.execution_status as ITallyExecutionStatus)
                    )
            }

            // If there are no selected elections, or if there is an election that is not published,
            // or if the initialization report is either not allowed or already generated when allowed,
            // then `newStatus` will be `true`, and the button will be disabled.
            const newStatus =
                selectedElections.length == 0 ||
                elections
                    ?.filter((election) => selectedElections.includes(election.id))
                    .some(
                        (election) =>
                            !election.status?.is_published ||
                            hasInitializationReport(election.id) ||
                            (election.presentation?.initialization_report_policy ==
                                EInitializeReportPolicy.REQUIRED &&
                                election.status?.init_report == EInitReport.DISALLOWED) ||
                            election.initialization_report_generated
                    ) ||
                false
            console.log(`InitReport: setIsButtonDisabled = ${newStatus}`)
            setIsButtonDisabled(newStatus)
        }
    }, [selectedElections, elections, allTallySessions])

    const handleNext = () => {
        if (page === WizardSteps.Start) {
            setOpenModal(true)
        } else if (page === WizardSteps.Ceremony) {
            setIsButtonDisabled(true)
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
                    keys_ceremony_id: keysCeremonyId,
                    election_ids: selectedElections,
                    tally_type: isCreatingType,
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
        setIsButtonDisabled(true)
        setIsConfirming(true)
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
                setIsConfirming(false)
                setIsButtonDisabled(false)
                return
            }

            if (nextStatus) {
                notify(t("tally.startTallySuccess"), {type: "success"})
                refetchTallySession()
                setIsConfirming(false)
                setIsButtonDisabled(false)
                setCreatingFlag(null)
            }
        } catch (error) {
            setIsConfirming(false)
            setIsButtonDisabled(false)
            notify(t("tally.startTallyError"), {type: "error"})
        }
    }

    let documents: IResultDocumentsData | null = useMemo(() => {
        let documents =
            !!resultsEventId &&
            !!resultsEvent &&
            resultsEvent?.[0]?.id === resultsEventId &&
            (resultsEvent[0]?.documents as IResultDocuments | null)
        return documents
            ? {
                  documents,
                  name: resultsEvent?.[0]?.name ?? "event",
                  class_type: "event",
              }
            : null
    }, [resultsEventId, resultsEvent, resultsEvent?.[0]?.id, resultsEvent?.[0]?.name])

    const handleMiruExportSuccess = (e: {
        election_id?: string
        area_id?: string
        existingPackage?: IMiruTransmissionPackageData
    }) => {
        //check for task completion and fetch data
        //set new page status(navigate to miru wizard)
        if (e.existingPackage) {
            setSelectedTallySessionData(e.existingPackage)
            setElectionEventIdFlag(record?.id)
            setMiruAreaId(e.existingPackage.area_id)
            setTransmissionLoading(false)
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
                    setElectionEventIdFlag(record?.id)
                    setMiruAreaId(packageData.area_id)
                    setTransmissionLoading(false)
                } else {
                    retry = retry + 1
                }
            }, globalSettings.QUERY_FAST_POLL_INTERVAL_MS)
        }
    }

    const [CreateTransmissionPackage] = useMutation<CreateTransmissionPackageMutation>(
        CREATE_TRANSMISSION_PACKAGE,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.MIRU_CREATE,
                },
            },
        }
    )

    const handleCreateTransmissionPackage = useCallback(
        async ({area_id, election_id}: {area_id: string; election_id: string}) => {
            setTransmissionLoading(true)
            console.log({
                electionId: election_id,
                tallySessionId: tallyId,
                areaId: area_id,
            })

            const found = tallySessionData.find(
                (datum) => datum.area_id === area_id && datum.election_id === election_id
            )

            if (!election_id) {
                notify(t("miruExport.create.error"), {type: "error"})
                setTransmissionLoading(false)
                console.log("Unable to get election id.")
                return
            }

            if (found) {
                handleMiruExportSuccess?.({existingPackage: found})
                return
            }

            const currWidget = addWidget(ETasksExecution.CREATE_TRANSMISSION_PACKAGE)
            try {
                const {data: nextStatus, errors} = await CreateTransmissionPackage({
                    variables: {
                        electionEventId: record?.id,
                        electionId: election_id,
                        tallySessionId: tallyId,
                        areaId: area_id,
                        force: false,
                    },
                })

                console.log("createTransmissionPackage", {nextStatus, errors})

                if (errors) {
                    updateWidgetFail(currWidget.identifier)
                } else if (nextStatus) {
                    const task_id = nextStatus?.create_transmission_package?.task_execution?.id
                    setWidgetTaskId(currWidget.identifier, task_id, () => {
                        refetchTallySession()
                        handleMiruExportSuccess?.({area_id, election_id})
                    })
                }
                setTransmissionLoading(false)
            } catch (error) {
                updateWidgetFail(currWidget.identifier)
                console.log(`Caught error: ${error}`)
                setTransmissionLoading(false)
            }
        },
        [tallySessionData, tally]
    )

    const sortedKeysCeremonies = useMemo(() => {
        // Ensure keysCeremonies and its nested properties exist
        const items = keysCeremonies?.list_keys_ceremony?.items
        if (!items) return []

        // Create a shallow copy and sort it
        return [...items].sort((a, b) => {
            if (!a?.name || !b?.name) return 0
            return a.name.localeCompare(b.name)
        })
        // Dependency array: re-run only when the original items array changes
    }, [keysCeremonies?.list_keys_ceremony?.items])

    return (
        <TallyStyles.WizardContainer>
            <TallyStyles.ContentWrapper>
                <WizardStyles.WizardWrapper data-tally-id={`tally-id-${tallyId}`}>
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

                    {resultsEventId &&
                    record?.id &&
                    isArray(contests) &&
                    isArray(tallySession?.election_ids) ? (
                        <ResultsDataLoader
                            resultsEventId={resultsEventId}
                            electionEventId={record?.id}
                            isTallyCompleted={isTallyCompleted}
                            contests={contests ?? []}
                            electionIds={tallySession?.election_ids ?? []}
                            databaseName={resultsSQLiteDocumentId}
                        />
                    ) : null}
                    {page === WizardSteps.Start && (
                        <>
                            {/* 
                            This code snippet determines whether the "Next" button should be
                            disabled on the Start page of the wizard. The button is disabled if:
                            1. The current page is the Start page and no elections are selected.
                            2. The elections are not published. 
                            */}
                            {isButtonDisabled && (
                                <Alert severity="warning">
                                    {t("electionEventScreen.tally.notify.startDisabled")}
                                </Alert>
                            )}
                            <ElectionHeader
                                title={
                                    isCreatingType === ETallyType.ELECTORAL_RESULTS
                                        ? "tally.ceremonyTitle"
                                        : "tally.initializationTitle"
                                }
                                subtitle={"tally.ceremonySubTitle"}
                            />
                            <TallyElectionsList
                                elections={elections}
                                update={(elections) => setSelectedElections(elections)}
                                disabled={isTallyElectionListDisabled}
                                electionEventId={record?.id}
                                keysCeremonyId={keysCeremonyId ?? null}
                                tallySession={tallySession}
                            />
                            <FormControl fullWidth>
                                <ElectionHeader
                                    title={"tally.keysCeremonyTitle"}
                                    subtitle={"tally.keysCeremonySubTitle"}
                                />

                                <Select
                                    id="keys-ceremony-for-tally"
                                    value={keysCeremonyId ?? ""}
                                    label={t("tally.keysCeremonyTitle")}
                                    placeholder={t("tally.keysCeremonyTitle")}
                                    onChange={(props) => {
                                        if (!props?.target?.value) {
                                            return
                                        }
                                        setPristine(false)
                                        setKeysCeremonyId(props?.target?.value)
                                    }}
                                >
                                    {sortedKeysCeremonies.map((keysCeremony) => (
                                        <MenuItem key={keysCeremony.id} value={keysCeremony.id}>
                                            {keysCeremony?.name}
                                        </MenuItem>
                                    ))}
                                </Select>
                            </FormControl>
                        </>
                    )}

                    {page === WizardSteps.Ceremony && (
                        <>
                            {/* 
                            This code snippet determines whether the "Next" button should be
                            disabled on the Ceremony page of the wizard. The button is disabled if:
                            1. The tally object's execution_status is not equal to ITallyExecutionStatus.CONNECTED.
                            2. The isStartAllowed variable is false.
                            The tally session is not in the CONNECTED state or if the start of the ceremony 
                            is not allowed based on the tally type and the status of the elections.
                            */}
                            {isButtonDisabled && (
                                <Alert severity="warning">
                                    {t("electionEventScreen.tally.notify.ceremonyDisabled")}
                                </Alert>
                            )}
                            <TallyElectionsList
                                elections={elections}
                                electionEventId={record?.id}
                                disabled={true}
                                update={(elections) => setSelectedElections(elections)}
                                keysCeremonyId={keysCeremonyId ?? null}
                                tallySession={tallySession}
                            />

                            <TallyTrusteesList
                                tally={tally}
                                update={(trustees) => {
                                    setSelectedTrustees(trustees)
                                }}
                                tallySessionExecutions={tallySessionExecutions}
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
                                                tally?.execution_status ??
                                                    ITallyExecutionStatus.STARTED
                                            ),
                                            color: theme.palette.background.default,
                                        }}
                                        label={t("keysGeneration.ceremonyStep.executionStatus", {
                                            status: tally?.execution_status,
                                        })}
                                    />
                                </AccordionSummary>
                                <WizardStyles.AccordionDetails>
                                    <TallyElectionsProgress
                                        tally={tally}
                                        tallySessionExecutions={tallySessionExecutions}
                                        allElections={elections}
                                    />
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
                                        onCreateTransmissionPackage={
                                            handleCreateTransmissionPackage
                                        }
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
                                                tally?.execution_status ??
                                                    ITallyExecutionStatus.STARTED
                                            ),
                                            color: theme.palette.background.default,
                                        }}
                                        label={t("keysGeneration.ceremonyStep.executionStatus", {
                                            status: tally?.execution_status,
                                        })}
                                    />
                                </AccordionSummary>
                                <WizardStyles.AccordionDetails>
                                    <TallyElectionsProgress
                                        tally={tally}
                                        tallySessionExecutions={tallySessionExecutions}
                                        allElections={elections}
                                    />
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
                                onChange={() => {}}
                            >
                                <AccordionSummary
                                    expandIcon={
                                        <div
                                            onClick={(e) => {
                                                e.stopPropagation()
                                                setExpandedResults((prev: IExpanded) => ({
                                                    ...prev,
                                                    "tally-results-results":
                                                        !prev["tally-results-results"],
                                                }))
                                            }}
                                        >
                                            <ExpandMoreIcon id="tally-data-results" />
                                        </div>
                                    }
                                    onClick={(e) => e.stopPropagation()}
                                >
                                    <WizardStyles.AccordionTitle
                                        onClick={(e) => {
                                            e.stopPropagation()
                                            setExpandedResults((prev: IExpanded) => ({
                                                ...prev,
                                                "tally-results-results":
                                                    !prev["tally-results-results"],
                                            }))
                                        }}
                                    >
                                        {t("tally.resultsTitle")}
                                    </WizardStyles.AccordionTitle>
                                    <TallyStyles.StyledSpacing>
                                        {resultsEvent?.[0] &&
                                        documents &&
                                        canExportCeremony &&
                                        tally?.id ? (
                                            <ExportElectionMenu
                                                documentsList={[documents]}
                                                tallySessionId={tally.id}
                                                electionEventId={
                                                    resultsEvent?.[0].election_event_id
                                                }
                                                itemName={resultsEvent?.[0]?.name ?? "event"}
                                            />
                                        ) : null}
                                    </TallyStyles.StyledSpacing>
                                </AccordionSummary>
                                <WizardStyles.AccordionDetails style={{zIndex: 100}}>
                                    <TallyResults
                                        tally={tally}
                                        resultsEventId={resultsEventId}
                                        onCreateTransmissionPackage={
                                            handleCreateTransmissionPackage
                                        }
                                        loading={transmissionLoading}
                                    />
                                </WizardStyles.AccordionDetails>
                            </Accordion>
                        </>
                    )}
                </WizardStyles.WizardWrapper>
            </TallyStyles.ContentWrapper>

            <TallyStyles.FooterContainer>
                <TallyStyles.StyledFooter>
                    {showTallyBackButton ? (
                        <CancelButton
                            className="list-actions"
                            onClick={() => {
                                setTallyId(null)
                                setCreatingFlag(null)
                            }}
                        >
                            <ArrowBackIosIcon />
                            {t("common.label.back")}
                        </CancelButton>
                    ) : null}
                    {page < WizardSteps.Results &&
                        tally?.execution_status !== ITallyExecutionStatus.CANCELLED && (
                            <NextButton
                                color="primary"
                                onClick={handleNext}
                                disabled={isButtonDisabled}
                            >
                                <>
                                    {page === WizardSteps.Start
                                        ? isCreatingType === ETallyType.ELECTORAL_RESULTS
                                            ? t("tally.common.ceremony")
                                            : t("tally.common.initialization")
                                        : page === WizardSteps.Ceremony
                                        ? t("tally.common.start")
                                        : page === WizardSteps.Tally
                                        ? t("tally.common.results")
                                        : t("tally.common.next")}
                                    {isConfirming ? (
                                        <StyledCircularProgress color="inherit" />
                                    ) : (
                                        <ChevronRightIcon
                                            style={{
                                                transform:
                                                    i18n.dir(i18n.language) === "rtl"
                                                        ? "rotate(180deg)"
                                                        : "rotate(0)",
                                            }}
                                        />
                                    )}
                                </>
                            </NextButton>
                        )}
                </TallyStyles.StyledFooter>
            </TallyStyles.FooterContainer>

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
                    } else {
                        setIsButtonDisabled(false)
                    }
                    setOpenCeremonyModal(false)
                }}
            >
                {t("tally.common.dialog.ceremony")}
            </Dialog>
        </TallyStyles.WizardContainer>
    )
}
