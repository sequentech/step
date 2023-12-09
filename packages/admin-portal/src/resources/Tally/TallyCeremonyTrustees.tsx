// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import Button from "@mui/material/Button"
import {
    BreadCrumbSteps,
    BreadCrumbStepsVariant,
    Dialog,
    IconButton,
} from "@sequentech/ui-essentials"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import {useTranslation} from "react-i18next"
import ElectionHeader from "@/components/ElectionHeader"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import styled from "@emotion/styled"
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
import {useGetOne, useNotify} from "react-admin"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {UPDATE_TALLY_CEREMONY} from "@/queries/UpdateTallyCeremony"
import {useMutation} from "@apollo/client"
import {ITallyExecutionStatus} from "@/types/ceremonies"
import {faKey} from "@fortawesome/free-solid-svg-icons"
import {Box} from "@mui/material"
import {Sequent_Backend_Area, Sequent_Backend_Tally_Session} from "@/gql/graphql"

interface TallyCeremonyTrusteesProps {
    completed: boolean
}

export const TallyCeremonyTrustees: React.FC<TallyCeremonyTrusteesProps> = (props) => {
    const {completed} = props

    const {t} = useTranslation()
    const [tallyId, setTallyId] = useElectionEventTallyStore()
    const notify = useNotify()

    const [openModal, setOpenModal] = useState(false)
    const [page, setPage] = useState<number>(0)
    const [showTrustees, setShowTrustees] = useState(false)
    const [selectedElections, setSelectedElections] = useState<string[]>([])
    const [selectedTrustees, setSelectedTrustees] = useState<string[]>([])
    const [tally, setTally] = useState<Sequent_Backend_Tally_Session>()

    const [UpdateTallyCeremonyMutation] = useMutation(UPDATE_TALLY_CEREMONY)

    interface IExpanded {
        [key: string]: boolean
    }

    const {data} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: tallyId,
        },
        {
            refetchInterval: 5000,
        }
    )

    useEffect(() => {
        if (data) {
            setPage(
                data.execution_status === ITallyExecutionStatus.CONNECTED
                    ? 1
                    : data.execution_status === ITallyExecutionStatus.IN_PROGRESS
                    ? 2
                    : 0
            )
            if (tally?.last_updated_at !== data.last_updated_at) {
                console.log("TallyCeremony :: data", data)
                setTally(data)
            }
        }
    }, [data])

    useEffect(() => {
        console.log("TallyCeremony :: tally", tally)
    }, [tally])

    const [expandedData, setExpandedData] = useState<IExpanded>({
        "tally-data-general": true,
        "tally-data-logs": true,
        "tally-data-results": true,
    })

    const [expandedResults, setExpandedResults] = useState<IExpanded>({
        "tally-results-general": true,
        "tally-results-results": true,
    })

    const CancelButton = styled(Button)`
        background-color: ${({theme}) => theme.palette.white};
        color: ${({theme}) => theme.palette.brandColor};
        border-color: ${({theme}) => theme.palette.brandColor};
        padding: 0 4rem;

        &:hover {
            background-color: ${({theme}) => theme.palette.brandColor};
        }
    `

    const NextButton = styled(Button)`
        background-color: ${({theme}) => theme.palette.brandColor};
        color: ${({theme}) => theme.palette.white};
        border-color: ${({theme}) => theme.palette.brandColor};
        padding: 0 4rem;

        &:hover {
            background-color: ${({theme}) => theme.palette.white};
            color: ${({theme}) => theme.palette.brandColor};
        }
    `

    const handleNext = () => {
        if (page === 0) {
            if (showTrustees) {
                setPage(page < 2 ? page + 1 : 0)
            } else {
                setOpenModal(true)
            }
        } else {
            setPage(page < 2 ? page + 1 : 0)
        }
    }

    const confirmNextAction = async () => {
        const {data, errors} = await UpdateTallyCeremonyMutation({
            variables: {
                election_event_id: tally?.election_event_id,
                tally_session_id: tally?.id,
                status: ITallyExecutionStatus.STARTED,
            },
        })

        if (errors) {
            notify(t("tally.startTallyError"), {type: "error"})
        }

        if (data) {
            notify(t("tally.startTallySuccess"), {type: "success"})
            setShowTrustees(true)
        }
        setShowTrustees(true)
    }

    const setDisabled = (): boolean => {
        if (!tally) {
            return true
        } else {
            if (
                page === 0 &&
                showTrustees &&
                tally?.execution_status !== ITallyExecutionStatus.CONNECTED
            ) {
                return true
            }
            return false
        }
    }

    return (
        <>
            <WizardStyles.WizardWrapper>
                <TallyStyles.StyledHeader>
                    <BreadCrumbSteps
                        labels={[
                            "tally.breadcrumbSteps.ceremony",
                            "tally.breadcrumbSteps.tally",
                            "tally.breadcrumbSteps.results",
                        ]}
                        selected={page}
                        variant={BreadCrumbStepsVariant.Circle}
                        colorPreviousSteps={true}
                    />
                </TallyStyles.StyledHeader>

                {page === 0 && (
                    <>
                        <ElectionHeader
                            title={t("tally.ceremonyTitle")}
                            subtitle={t("tally.ceremonySubTitle")}
                        />

                        <TallyElectionsList
                            update={(elections) => setSelectedElections(elections)}
                        />

                        {showTrustees && (
                            <>
                                <TallyStyles.StyledFooter>
                                    <ElectionHeader
                                        title={t("tally.trusteeTallyTitle")}
                                        subtitle={t("tally.trusteeTallySubTitle")}
                                    />
                                </TallyStyles.StyledFooter>

                                <Box
                                    sx={{
                                        width: "100%",
                                        display: "flex",
                                        justifyContent: "flex-end",
                                    }}
                                >
                                    <IconButton
                                        icon={faKey}
                                        sx={{
                                            color:
                                                tally?.execution_status ===
                                                ITallyExecutionStatus.CONNECTED
                                                    ? "#43E3A1"
                                                    : "#d32f2f",
                                        }}
                                    />
                                </Box>

                                <TallyTrusteesList
                                    update={(trustees) => setSelectedTrustees(trustees)}
                                />
                            </>
                        )}
                    </>
                )}

                {page === 1 && (
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
                                    <ElectionHeader title={t("tally.tallyTitle")} subtitle="" />
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
                                    <ElectionHeader title={t("tally.logsTitle")} subtitle="" />
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
                                    <ElectionHeader title={t("tally.resultsTitle")} subtitle="" />
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <TallyResults tally={tally} />
                            </AccordionDetails>
                        </Accordion>
                    </>
                )}

                {page === 2 && (
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
                                    <ElectionHeader
                                        title={t("tally.generalInfoTitle")}
                                        subtitle=""
                                    />
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
                    <CancelButton className="list-actions" onClick={() => setTallyId(null)}>
                        {t("tally.common.cancel")}
                    </CancelButton>
                    {page < 2 && (
                        <NextButton color="primary" onClick={handleNext} disabled={setDisabled()}>
                            <>
                                {t("tally.common.next")}
                                <ChevronRightIcon />
                            </>
                        </NextButton>
                    )}
                </TallyStyles.StyledFooter>
            </WizardStyles.WizardWrapper>

            <Dialog
                variant="warning"
                open={openModal}
                ok={t("tally.common.dialog.ok")}
                cancel={t("tally.common.dialog.cancel")}
                title={t("tally.common.dialog.title")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmNextAction()
                    }
                    setOpenModal(false)
                }}
            >
                {t("tally.common.dialog.message")}
            </Dialog>
        </>
    )
}
