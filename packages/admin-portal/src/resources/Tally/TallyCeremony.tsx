// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {BreadCrumbSteps, BreadCrumbStepsVariant, Dialog} from "@sequentech/ui-essentials"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import {useTranslation} from "react-i18next"
import ElectionHeader from "@/components/ElectionHeader"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {Accordion, AccordionDetails, AccordionSummary} from "@mui/material"
import {ElectionStyles} from "@/components/styles/ElectionStyles"
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
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
    Sequent_Backend_Tally_Session,
} from "@/gql/graphql"
import {CancelButton, NextButton} from "./styles"

const WizardSteps = {
    Start: 0,
    Ceremony: 1,
    Tally: 2,
    Results: 3,
}

interface IExpanded {
    [key: string]: boolean
}

export const TallyCeremony: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const {t} = useTranslation()
    const {tallyId, setTallyId, setCreatingFlag} = useElectionEventTallyStore()
    const notify = useNotify()

    const [openModal, setOpenModal] = useState(false)
    const [openCeremonyModal, setOpenCeremonyModal] = useState(false)
    const [page, setPage] = useState<number>(WizardSteps.Start)
    const [tally, setTally] = useState<Sequent_Backend_Tally_Session>()
    const [isButtonDisabled, setIsButtonDisabled] = useState<boolean>(true)

    const [selectedElections, setSelectedElections] = useState<string[]>([])
    const [selectedTrustees, setSelectedTrustees] = useState<boolean>(false)

    const [CreateTallyCeremonyMutation] = useMutation(CREATE_TALLY_CEREMONY)
    const [UpdateTallyCeremonyMutation] = useMutation(UPDATE_TALLY_CEREMONY)

    const [expandedData, setExpandedData] = useState<IExpanded>({
        "tally-data-general": true,
        "tally-data-logs": true,
        "tally-data-results": true,
    })

    const [expandedResults, setExpandedResults] = useState<IExpanded>({
        "tally-results-general": true,
        "tally-results-results": true,
    })

    const {data} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: tallyId,
        },
        {
            refetchInterval: 5000,
        }
    )

    const {data: keyCeremony} = useGetList<Sequent_Backend_Keys_Ceremony>(
        "sequent_backend_keys_ceremony",
        {
            pagination: {page: 1, perPage: 9999},
            filter: {election_event_id: record?.id, tenant_id: record?.tenant_id},
        }
    )

    useEffect(() => {
        if (data) {
            if (tally?.last_updated_at !== data.last_updated_at) {
                setPage(
                    data.execution_status === ITallyExecutionStatus.STARTED ||
                        data.execution_status === ITallyExecutionStatus.CONNECTED
                        ? WizardSteps.Ceremony
                        : data.execution_status === ITallyExecutionStatus.IN_PROGRESS
                        ? WizardSteps.Tally
                        : data.execution_status === ITallyExecutionStatus.SUCCESS
                        ? WizardSteps.Results
                        : WizardSteps.Start
                )
                setTally(data)
            }
        }
    }, [data])

    useEffect(() => {
        setIsButtonDisabled(
            page === WizardSteps.Start && selectedElections.length === 0 ? true : false
        )
    }, [selectedElections])

    useEffect(() => {
        console.log("TallyCeremony :: status :: ", tally?.execution_status)
    }, [selectedTrustees])

    useEffect(() => {
        console.log("TallyCeremony :: isButtonDisabled :: ", isButtonDisabled)
    }, [isButtonDisabled])

    useEffect(() => {
        if (page === WizardSteps.Ceremony) {
            console.log("cambiame el button coño")
            setIsButtonDisabled(tally?.execution_status !== ITallyExecutionStatus.CONNECTED)
        }
    }, [tally])

    const handleNext = () => {
        if (page === WizardSteps.Start) {
            setOpenModal(true)
        } else if (page === WizardSteps.Ceremony) {
            setOpenCeremonyModal(true)
        } else if (page === WizardSteps.Tally) {
            setPage(page < 2 ? page + 1 : 0)
        } else {
            setPage(page < 2 ? page + 1 : 0)
        }
    }

    const confirmStartAction = async () => {
        try {
            const {data, errors} = await CreateTallyCeremonyMutation({
                variables: {
                    tenant_id: record?.tenant_id,
                    election_event_id: record?.id,
                    keys_ceremony_id: keyCeremony?.[0].id,
                    election_ids: selectedElections,
                },
            })

            if (errors) {
                notify(t("tally.createTallyError"), {type: "error"})
            }

            if (data) {
                notify(t("tally.createTallySuccess"), {type: "success"})
                console.log("TallyCeremony :: confirmStartAction :: data", data)

                const {data: nextStatus, errors} = await UpdateTallyCeremonyMutation({
                    variables: {
                        election_event_id: record?.id,
                        tally_session_id: data.create_tally_ceremony.tally_session_id,
                        status: ITallyExecutionStatus.STARTED,
                    },
                })

                if (errors) {
                    notify(t("tally.startTallyError"), {type: "error"})
                }

                if (nextStatus) {
                    notify(t("tally.startTallySuccess"), {type: "success"})
                    setCreatingFlag(false)
                    setPage(WizardSteps.Ceremony)
                }
            }
        } catch (error) {
            console.log("TallyCeremony :: confirmStartAction :: error", error)
            notify(t("tally.startTallyError"), {type: "error"})
        }
    }

    const confirmCeremonyAction = async () => {
        try {
            if (selectedTrustees) {
                const {data: nextStatus, errors} = await UpdateTallyCeremonyMutation({
                    variables: {
                        election_event_id: record?.id,
                        tally_session_id: tallyId,
                        status: ITallyExecutionStatus.IN_PROGRESS,
                    },
                })

                if (errors) {
                    notify(t("tally.startTallyError"), {type: "error"})
                }

                if (nextStatus) {
                    notify(t("tally.startTallySuccess"), {type: "success"})
                    setCreatingFlag(false)
                    setPage(WizardSteps.Ceremony)
                }
            }
        } catch (error) {
            console.log("TallyCeremony :: confirmCeremonyAction :: error", error)
            notify(t("tally.startTallyError"), {type: "error"})
        }
    }

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

                {page === WizardSteps.Start && (
                    <>
                        <ElectionHeader
                            title={"tally.ceremonyTitle"}
                            subtitle={"tally.ceremonySubTitle"}
                        />

                        <TallyElectionsList
                            update={(elections) => setSelectedElections(elections)}
                        />
                    </>
                )}

                {page === WizardSteps.Ceremony && (
                    <>
                        <TallyElectionsList
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
                            expanded={expandedData["tally-data-general"]}
                            onChange={() =>
                                setExpandedData((prev: IExpanded) => ({
                                    ...prev,
                                    "tally-data-general": !prev["tally-data-general"],
                                }))
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="tally-data-general" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionHeader title={"tally.tallyTitle"} subtitle="" />
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <TallyElectionsProgress />
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expandedData["tally-data-logs"]}
                            onChange={() =>
                                setExpandedData((prev: IExpanded) => ({
                                    ...prev,
                                    "tally-data-logs": !prev["tally-data-logs"],
                                }))
                            }
                        >
                            <AccordionSummary expandIcon={<ExpandMoreIcon id="tally-data-logs" />}>
                                <ElectionStyles.Wrapper>
                                    <ElectionHeader title={"tally.logsTitle"} subtitle="" />
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <TallyLogs />
                            </AccordionDetails>
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
                                <ElectionStyles.Wrapper>
                                    <ElectionHeader title={"tally.resultsTitle"} subtitle="" />
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <TallyResults tally={tally} />
                            </AccordionDetails>
                        </Accordion>
                    </>
                )}

                {page === WizardSteps.Results && (
                    <>
                        <TallyStyles.StyledSpacing>
                            <ListActions
                                withImport={false}
                                withColumns={false}
                                withFilter={false}
                            />
                        </TallyStyles.StyledSpacing>

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
                                <ElectionStyles.Wrapper>
                                    <ElectionHeader title={"tally.generalInfoTitle"} subtitle="" />
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <TallyStartDate />
                                <TallyElectionsResults
                                    tenantId={tally?.tenant_id}
                                    electionEventId={tally?.election_event_id}
                                    electionIds={tally?.election_ids}
                                />
                            </AccordionDetails>
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
                                <ElectionStyles.Wrapper>
                                    <ElectionHeader title={t("tally.resultsTitle")} subtitle="" />
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <TallyResults tally={tally} />
                            </AccordionDetails>
                        </Accordion>
                    </>
                )}

                <TallyStyles.StyledFooter>
                    <CancelButton
                        className="list-actions"
                        onClick={() => {
                            setTallyId(null)
                            setCreatingFlag(false)
                        }}
                    >
                        {t("tally.common.cancel")}
                    </CancelButton>
                    {page < WizardSteps.Results && (
                        <NextButton
                            color="primary"
                            onClick={handleNext}
                            disabled={isButtonDisabled}
                        >
                            <>
                                {t("tally.common.next")}
                                <ChevronRightIcon />
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
                ok={t("tally.common.dialog.ok")}
                cancel={t("tally.common.dialog.cancel")}
                title={t("tally.common.dialog.title")}
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
