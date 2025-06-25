// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    Accordion,
    AccordionSummary,
    Box,
    CircularProgress,
    PaletteColor,
    TextField,
    Tooltip,
} from "@mui/material"
import React, {useCallback, useContext, useEffect, useMemo, useRef, useState} from "react"
import {WizardStyles} from "./styles/WizardStyles"
import {TallyStyles} from "./styles/TallyStyles"
import {MiruServers} from "./MiruServers"
import {MiruSignatures} from "./MiruSignatures"
import {theme, DropFile, Dialog} from "@sequentech/ui-essentials"
import {Logs} from "./Logs"
import {MiruPackageDownload} from "./MiruPackageDownload"
import {IExpanded} from "@/resources/Tally/TallyCeremony"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import CellTowerIcon from "@mui/icons-material/CellTower"
import RestartAltIcon from "@mui/icons-material/RestartAlt"
import {
    CreateTransmissionPackageMutation,
    GetUploadUrlMutation,
    SendTransmissionPackageMutation,
    Sequent_Backend_Area,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Results_Event,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
    UploadSignatureMutation,
} from "@/gql/graphql"
import {
    IMiruTallySessionData,
    IMiruTransmissionPackageData,
    MIRU_TALLY_SESSION_ANNOTATION_KEY,
} from "@/types/miru"
import {IResultDocuments} from "@/types/results"
import {useTranslation} from "react-i18next"
import {useGetList, useGetOne, useNotify, useRecordContext} from "react-admin"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {useMutation} from "@apollo/client"
import {SEND_TRANSMISSION_PACKAGE} from "@/queries/SendTransmissionPackage"
import {UPLOAD_SIGNATURE} from "@/queries/UploadSignature"
import {AuthContext} from "@/providers/AuthContextProvider"
import {ElectionHeaderStyles} from "./styles/ElectionHeaderStyles"
import {useAtomValue} from "jotai"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {CREATE_TRANSMISSION_PACKAGE} from "@/queries/CreateTransmissionPackage"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {translateElection} from "@sequentech/ui-core"
import {ETasksExecution} from "@/types/tasksExecution"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {WidgetProps} from "@/components/Widget"
import {CancelButton} from "@/resources/Tally/styles"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"

interface IMiruExportWizardProps {}

export const MiruExportWizard: React.FC<IMiruExportWizardProps> = ({}) => {
    const elementRef = useRef(null)
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {globalSettings} = useContext(SettingsContext)
    const {
        tallyId,
        electionEventId,
        setElectionEventIdFlag,
        setMiruAreaId,
        selectedTallySessionData,
        setSelectedTallySessionData,
    } = useElectionEventTallyStore()
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const [transmissionLoading, setTransmissionLoading] = useState<boolean>(false)
    const [regenTransmissionLoading, setRegenTransmissionLoading] = useState<boolean>(false)
    const [uploading, setUploading] = useState<boolean>(false)
    const [errors, setErrors] = useState<String | null>(null)
    const [tally, setTally] = useState<Sequent_Backend_Tally_Session>()
    const [passwordState, setPasswordState] = useState<string>("")
    const [signatureId, setSignatureId] = useState<string>("")
    const authContext = useContext(AuthContext)
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()

    const {data: areaData} = useGetOne<Sequent_Backend_Area>(
        "sequent_backend_area",
        {id: selectedTallySessionData?.area_id || electionEventId},
        {enabled: !!selectedTallySessionData?.area_id}
    )

    const isTrustee = useMemo(() => {
        let sbeiUsersStr: string | undefined = record?.annotations?.["miru:sbei-users"]
        if (!sbeiUsersStr) {
            return false
        }

        let areaUsersStr: string | undefined = areaData?.annotations?.["miru:area-trustee-users"]
        if (!areaUsersStr) {
            return false
        }
        try {
            let username = authContext.username
            let sbeiUsers: Array<{username: string; miru_id: string}> = JSON.parse(sbeiUsersStr)

            let validSbeiUsers = sbeiUsers
                .filter((user) => user.username === username)
                .map((user) => user.miru_id)

            if (!validSbeiUsers) {
                return false
            }
            let areaUsers: Array<string> = JSON.parse(areaUsersStr)
            return areaUsers.some((areaUserId) => validSbeiUsers.includes(areaUserId))
        } catch (error) {
            console.log(error)
            return false
        }
    }, [
        record?.annotations?.["miru:sbei-users"],
        areaData?.annotations?.["miru:area-trustee-users"],
    ])

    const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.DOCUMENT_UPLOAD,
            },
        },
    })

    const [uploadSignature] = useMutation<UploadSignatureMutation>(UPLOAD_SIGNATURE, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.MIRU_SIGN,
            },
        },
    })

    const handleUploadSignature = async ({
        documentId,
        password,
    }: {
        documentId: string
        password: string
    }) => {
        try {
            const {errors} = await uploadSignature({
                variables: {
                    electionId: selectedTallySessionData?.election_id,
                    tallySessionId: tally?.id,
                    areaId: selectedTallySessionData?.area_id,
                    documentId: documentId,
                    password: password,
                },
            })
            setUploading(false)
            setSignatureId("")
            setPasswordState("")

            if (errors) {
                setErrors(t("tally.errorUploadingSignature", {error: errors.toString()}))
            } else {
                notify(t("Signing Successful"), {type: "success"})
            }
        } catch (errors) {
            setErrors(t("tally.errorUploadingSignature", {error: String(errors)}))
            setPasswordState("")
            setUploading(false)
            setSignatureId("")
        }
    }

    const handleFiles = async (files: FileList | null) => {
        // https://fullstackdojo.medium.com/s3-upload-with-presigned-url-react-and-nodejs-b77f348d54cc

        const theFile = files?.[0]
        console.log({theFile, selectedTallySessionData})

        try {
            if (theFile) {
                let {data, errors} = await getUploadUrl({
                    variables: {
                        name: theFile.name,
                        media_type: theFile.type,
                        size: theFile.size,
                        is_public: false,
                        election_event_id: electionEventId,
                    },
                })
                console.log({data, errors})
                if (data?.get_upload_url?.document_id && data?.get_upload_url?.url) {
                    await fetch(data.get_upload_url.url, {
                        method: "PUT",
                        headers: {
                            "Content-Type": theFile.type,
                        },
                        body: theFile,
                    })

                    setSignatureId(data?.get_upload_url?.document_id)
                    // handleUploadSignature({doc_id: data?.get_upload_url?.document_id, password: '1234'})// for testing
                } else {
                    setUploading(false)
                    setErrors(t("keysGeneration.checkStep.errorUploading"))
                }
            }
        } catch (error) {
            console.log(error)
            setUploading(false)
            setErrors(t("keysGeneration.checkStep.errorUploading"))
        }
    }

    const [expandedExports, setExpandedDataExports] = useState<IExpanded>({
        "tally-miru-upload": true,
        "tally-miru-signatures": false,
        "tally-download-package": false,
        "tally-miru-servers": false,
    })

    const {t, i18n} = useTranslation()

    const {data} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: tallyId,
        },
        {
            refetchInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
            refetchIntervalInBackground: true,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

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

    useEffect(() => {
        if (data) {
            setTally(data)
        }
    }, [data])

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
            refetchInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const [SendTransmissionPackage] = useMutation<SendTransmissionPackageMutation>(
        SEND_TRANSMISSION_PACKAGE,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.MIRU_SEND,
                },
            },
        }
    )

    const handleSendTransmissionPackage = useCallback(async () => {
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
    }, [
        setTransmissionLoading,
        selectedTallySessionData?.election_id,
        tallyId,
        selectedTallySessionData?.area_id,
        t,
        notify,
    ])

    let tallySessionExecution = tallySessionExecutions?.[0] ?? null
    let resultsEventId = tallySessionExecution?.results_event_id ?? null
    const tallySessionDataRef = useRef(tallySessionData)

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

    const signaturesStatusColor: () => string = () => {
        let signed = signedCount()
        let trustees = trusteeCount()
        let minimum = minimumSignatures()

        return signed === 0
            ? theme.palette.warning.main
            : signed === minimum
            ? theme.palette.info.main
            : theme.palette.brandSuccess
    }

    const signedCount: () => number = () => {
        let signatures =
            selectedTallySessionData?.documents[selectedTallySessionData?.documents.length - 1]
                .signatures ?? []

        return signatures.filter((signature) => !!signature.signature && !!signature.pub_key).length
    }

    const trusteeCount: () => number = () => {
        let trustees = tallySessionExecution?.status?.trustees ?? []
        return trustees.length
    }

    const serversStatusColor: () => string = () => {
        let sentTo = serverSentToCount()
        let servers = serversTotalCount()

        return sentTo < servers ? theme.palette.info.main : theme.palette.brandSuccess
    }

    const serverSentToCount: () => number = () => {
        let sentTo =
            selectedTallySessionData?.documents[selectedTallySessionData?.documents.length - 1]
                .servers_sent_to ?? []

        return sentTo.filter((server) => server.status == "SUCCESS").length
    }

    const serversTotalCount: () => number = () => {
        let servers = selectedTallySessionData?.servers ?? []
        return servers.length
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

    const [confirmSendMiruModal, setConfirmSendMiruModal] = useState(false)
    const [confirmRegenerateMiruModal, setConfirmRegenerateMiruModal] = useState(false)

    const tallyData = useAtomValue(tallyQueryData)

    const area: Sequent_Backend_Area | null = useMemo(
        () =>
            tallyData?.sequent_backend_area?.find(
                (area) => selectedTallySessionData?.area_id === area.id
            ) ?? null,
        [selectedTallySessionData?.area_id, tallyData?.sequent_backend_area]
    )

    const election = useMemo(
        () =>
            tallyData?.sequent_backend_election?.find(
                (election) => selectedTallySessionData?.election_id === election.id
            ) ?? null,
        [selectedTallySessionData?.election_id, tallyData?.sequent_backend_election]
    )

    let minimumSignatures = () => {
        return selectedTallySessionData?.threshold ?? 1
    }

    const disableSendButton = useMemo(() => {
        return serversTotalCount() === serverSentToCount() || signedCount() < minimumSignatures()
    }, [serversTotalCount, serverSentToCount, signedCount, trusteeCount, minimumSignatures])

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

    const handleMiruExportSuccess = (e: {
        election_id?: string
        area_id?: string
        existingPackage?: IMiruTransmissionPackageData
    }) => {
        //check for task completion and fetch data
        //set new page status(navigate to miru wizard)
        if (e.existingPackage) {
            setSelectedTallySessionData(e.existingPackage)
            setMiruAreaId(e.existingPackage.area_id)
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
                    // setMiruElectionId(packageData.election_id)
                    // setMiruAreaId(packageData.area_id)
                } else {
                    retry = retry + 1
                }
            }, globalSettings.QUERY_FAST_POLL_INTERVAL_MS)
        }
    }

    const handleCreateTransmissionPackage = useCallback(
        async ({
            area_id,
            election_id,
            force,
        }: {
            area_id: string
            election_id: string | null
            force: boolean
        }) => {
            setRegenTransmissionLoading(true)
            const found = tallySessionData.find(
                (datum) => datum.area_id === area_id && datum.election_id === election_id
            )

            if (!election_id) {
                setRegenTransmissionLoading(false)
                notify(t("miruExport.create.error"), {type: "error"})
                console.log("Unable to get election id.")
                return
            }

            if (found && !force) {
                setRegenTransmissionLoading(false)
                handleMiruExportSuccess?.({existingPackage: found})
                return
            }

            let currWidget: WidgetProps | undefined
            try {
                if (!isTrustee) {
                    currWidget = addWidget(ETasksExecution.CREATE_TRANSMISSION_PACKAGE)
                }
                const {data: nextStatus, errors} = await CreateTransmissionPackage({
                    variables: {
                        electionEventId: record?.id,
                        electionId: election_id,
                        tallySessionId: tallyId,
                        areaId: area_id,
                        force,
                    },
                })

                if (errors) {
                    currWidget && updateWidgetFail(currWidget.identifier)
                    !currWidget && notify(t("miruExport.create.error"), {type: "error"})
                } else if (nextStatus) {
                    const task_id = nextStatus?.create_transmission_package?.task_execution?.id
                    currWidget && setWidgetTaskId(currWidget.identifier, task_id)
                    !currWidget && notify(t("miruExport.create.success"), {type: "success"})
                    handleMiruExportSuccess?.({area_id, election_id})
                }
                setRegenTransmissionLoading(false)
            } catch (error) {
                setRegenTransmissionLoading(false)
                currWidget && updateWidgetFail(currWidget.identifier)
                console.log(`Caught error: ${error}`)
                !currWidget && notify(t("miruExport.create.error"), {type: "error"})
            }
        },
        [tallySessionData, tally]
    )

    const eventName =
        (election &&
            (translateElection(election, "alias", i18n.language) ||
                translateElection(election, "name", i18n.language))) ||
        election?.alias ||
        election?.name ||
        "-"

    const canDownloadMiru = authContext.hasRole(IPermissions.MIRU_DOWNLOAD)
    const canSendMiru = authContext.hasRole(IPermissions.MIRU_SEND)
    const canCreateMiru = authContext.hasRole(IPermissions.MIRU_CREATE)

    const goBack = () => {
        setSelectedTallySessionData(null)
    }

    return (
        <>
            <TallyStyles.MiruHeader>
                <ElectionHeaderStyles.ThinWrapper>
                    <ElectionHeaderStyles.Title ref={elementRef}>
                        {t("tally.transmissionPackage.title", {
                            name: area?.name,
                            eventName,
                        })}
                    </ElectionHeaderStyles.Title>
                    <ElectionHeaderStyles.SubTitle>
                        {t("tally.transmissionPackage.description")}
                    </ElectionHeaderStyles.SubTitle>
                </ElectionHeaderStyles.ThinWrapper>

                <TallyStyles.MiruToolbar>
                    {canSendMiru ? (
                        <Tooltip
                            title={
                                disableSendButton
                                    ? "Have not reached minimum number of SBEI Member signatures or Transmission Package has already been sent to all servers"
                                    : ""
                            }
                        >
                            <span>
                                <TallyStyles.MiruToolbarButton
                                    aria-label="send transmission package"
                                    aria-haspopup="true"
                                    onClick={() => setConfirmSendMiruModal(true)}
                                    disabled={disableSendButton}
                                >
                                    <>
                                        {transmissionLoading ? (
                                            <CircularProgress size={16} />
                                        ) : (
                                            <CellTowerIcon />
                                        )}
                                        <span
                                            title={t(
                                                "tally.transmissionPackage.actions.send.title"
                                            )}
                                        >
                                            {t("tally.transmissionPackage.actions.send.title")}
                                        </span>
                                    </>
                                </TallyStyles.MiruToolbarButton>
                            </span>
                        </Tooltip>
                    ) : null}
                    {canDownloadMiru ? (
                        <MiruPackageDownload
                            areaName={area?.name}
                            documents={selectedTallySessionData?.documents ?? []}
                            tenantId={tenantId ?? ""}
                            electionEventId={electionEventId ?? ""}
                            electionId={selectedTallySessionData?.election_id}
                            tallySessionId={tallyId || undefined}
                            eventName={eventName}
                        />
                    ) : null}
                    {canCreateMiru ? (
                        <TallyStyles.MiruToolbarButton
                            aria-label="regenerate transmission package"
                            aria-haspopup="true"
                            onClick={() => setConfirmRegenerateMiruModal(true)}
                        >
                            <>
                                {regenTransmissionLoading ? (
                                    <CircularProgress size={16} />
                                ) : (
                                    <RestartAltIcon />
                                )}
                                <span
                                    title={t("tally.transmissionPackage.actions.regenerate.title")}
                                >
                                    {t("tally.transmissionPackage.actions.regenerate.title")}
                                </span>
                            </>
                        </TallyStyles.MiruToolbarButton>
                    ) : null}
                </TallyStyles.MiruToolbar>
            </TallyStyles.MiruHeader>
            {isTrustee && (
                <Accordion
                    sx={{width: "100%"}}
                    expanded={expandedExports["tally-miru-upload"]}
                    onChange={() =>
                        setExpandedDataExports((prev: IExpanded) => ({
                            ...prev,
                            "tally-miru-upload": !prev["tally-miru-upload"],
                        }))
                    }
                >
                    <AccordionSummary>
                        <Box className="flex flex-col items-start">
                            <WizardStyles.AccordionTitle>
                                {t("tally.uploadTransmissionPackage")}
                            </WizardStyles.AccordionTitle>
                            <WizardStyles.AccordionSubTitle>
                                {t("tally.uploadTransmissionPackageDesc")}
                            </WizardStyles.AccordionSubTitle>
                        </Box>
                    </AccordionSummary>
                    <WizardStyles.AccordionDetails style={{zIndex: 100}}>
                        <DropFile handleFiles={handleFiles} />
                        <WizardStyles.StatusBox>
                            {uploading ? <WizardStyles.DownloadProgress /> : null}
                            {errors ? (
                                <WizardStyles.ErrorMessage variant="body2">
                                    {errors}
                                </WizardStyles.ErrorMessage>
                            ) : null}
                        </WizardStyles.StatusBox>
                    </WizardStyles.AccordionDetails>
                </Accordion>
            )}
            <Accordion
                sx={{width: "100%"}}
                expanded={expandedExports["tally-miru-signatures"]}
                onChange={() =>
                    setExpandedDataExports((prev: IExpanded) => ({
                        ...prev,
                        "tally-miru-signatures": !prev["tally-miru-signatures"],
                    }))
                }
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="tally-miru-signatures" />}>
                    <WizardStyles.AccordionTitle>
                        {t("tally.transmissionPackage.signatures.title")}
                    </WizardStyles.AccordionTitle>
                    <WizardStyles.CeremonyStatus
                        sx={{
                            backgroundColor: signaturesStatusColor(),
                            color: theme.palette.background.default,
                            textTransform: "uppercase",
                        }}
                        label={t("tally.transmissionPackage.signatures.status", {
                            signed: signedCount(),
                            total: trusteeCount(),
                            minimum: minimumSignatures(),
                        })}
                    />
                </AccordionSummary>
                <WizardStyles.AccordionDetails style={{zIndex: 100}}>
                    <WizardStyles.AccordionSubTitle>
                        {t("tally.transmissionPackage.signatures.description")}
                    </WizardStyles.AccordionSubTitle>
                    <MiruSignatures
                        signatures={
                            selectedTallySessionData?.documents[
                                selectedTallySessionData?.documents.length - 1
                            ].signatures ?? []
                        }
                        area={area}
                    />
                </WizardStyles.AccordionDetails>
            </Accordion>
            <Accordion
                sx={{width: "100%"}}
                expanded={expandedExports["tally-miru-servers"]}
                onChange={() =>
                    setExpandedDataExports((prev: IExpanded) => ({
                        ...prev,
                        "tally-miru-servers": !prev["tally-miru-servers"],
                    }))
                }
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="tally-miru-servers" />}>
                    <WizardStyles.AccordionTitle>
                        {t("tally.transmissionPackage.destinationServers.title")}
                    </WizardStyles.AccordionTitle>
                    <WizardStyles.CeremonyStatus
                        sx={{
                            backgroundColor: serversStatusColor(),
                            color: theme.palette.background.default,
                            textTransform: "uppercase",
                        }}
                        label={t("tally.transmissionPackage.destinationServers.status", {
                            signed: serverSentToCount(),
                            total: serversTotalCount(),
                        })}
                    />
                </AccordionSummary>
                <WizardStyles.AccordionDetails style={{zIndex: 100}}>
                    <WizardStyles.AccordionSubTitle>
                        {t("tally.transmissionPackage.destinationServers.description")}
                    </WizardStyles.AccordionSubTitle>
                    <MiruServers
                        servers={selectedTallySessionData?.servers ?? []}
                        serversSentTo={
                            selectedTallySessionData?.documents[
                                selectedTallySessionData?.documents.length - 1
                            ].servers_sent_to ?? []
                        }
                    />
                </WizardStyles.AccordionDetails>
            </Accordion>

            <Logs logs={selectedTallySessionData?.logs} />

            <WizardStyles.FooterContainer>
                <WizardStyles.StyledFooter>
                    <CancelButton onClick={goBack} className="list-actions">
                        <ArrowBackIosIcon />
                        {t("common.label.back")}
                    </CancelButton>
                </WizardStyles.StyledFooter>
            </WizardStyles.FooterContainer>
            <Dialog
                variant="info"
                open={confirmSendMiruModal}
                ok={t("tally.transmissionPackage.actions.send.dialog.confirm")}
                cancel={t("tally.transmissionPackage.actions.send.dialog.cancel")}
                title={t("tally.transmissionPackage.actions.send.dialog.title")}
                handleClose={(result: boolean) => {
                    setConfirmSendMiruModal(false)
                    if (result) {
                        handleSendTransmissionPackage()
                    }
                }}
            >
                {t("tally.transmissionPackage.actions.send.dialog.description", {
                    name: area?.name,
                })}
            </Dialog>
            <Dialog
                variant="info"
                open={confirmRegenerateMiruModal}
                ok={t("tally.transmissionPackage.actions.regenerate.dialog.confirm")}
                cancel={t("tally.transmissionPackage.actions.regenerate.dialog.cancel")}
                title={t("tally.transmissionPackage.actions.regenerate.dialog.title")}
                handleClose={(result: boolean) => {
                    setConfirmRegenerateMiruModal(false)
                    if (
                        result &&
                        selectedTallySessionData?.area_id &&
                        selectedTallySessionData?.election_id
                    ) {
                        handleCreateTransmissionPackage({
                            area_id: selectedTallySessionData?.area_id,
                            election_id: selectedTallySessionData?.election_id,
                            force: true,
                        })
                    }
                }}
            >
                {t("tally.transmissionPackage.actions.regenerate.dialog.description", {
                    name: area?.name,
                })}
            </Dialog>
            <Dialog
                variant="info"
                open={!!signatureId}
                ok={t("tally.transmissionPackage.actions.sign.dialog.confirm")}
                cancel={t("tally.transmissionPackage.actions.regenerate.dialog.cancel")}
                title={t("tally.transmissionPackage.actions.sign.dialog.title")}
                handleClose={(result: boolean) => {
                    if (!result || !signatureId || !passwordState) {
                        setSignatureId("")
                        setPasswordState("")
                        setUploading(false)
                        return
                    }
                    handleUploadSignature({documentId: signatureId, password: passwordState})
                }}
            >
                <Box>
                    <TextField
                        dir={i18n.dir(i18n.language)}
                        label={t("tally.transmissionPackage.actions.sign.dialog.input.placeholder")}
                        size="small"
                        value={passwordState}
                        onChange={(e) => setPasswordState(e.target.value)}
                        type="password"
                    />
                </Box>
            </Dialog>
        </>
    )
}
