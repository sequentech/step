// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Identifier, useRecordContext, useGetOne, useGetMany} from "react-admin"
import Button from "@mui/material/Button"

import {
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tally_Session,
} from "../../gql/graphql"
import {BreadCrumbSteps, BreadCrumbStepsVariant, Dialog} from "@sequentech/ui-essentials"
import {Action} from "../../components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import DescriptionIcon from "@mui/icons-material/Description"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "../../providers/TenantContextProvider"
import ElectionHeader from "@/components/ElectionHeader"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import styled from "@emotion/styled"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import Checkbox from "@mui/material/Checkbox"
import {Accordion, AccordionDetails, AccordionSummary} from "@mui/material"
import {ElectionStyles} from "@/components/styles/ElectionStyles"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {ListActions} from "@/components/ListActions"

interface TallyCeremonyProps {
    completed: boolean
}

export const TallyCeremony: React.FC<TallyCeremonyProps> = (props) => {
    const {completed} = props

    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const [tallyId, setTallyId] = useElectionEventTallyStore()

    const [open, setOpen] = useState(false)
    const [openModal, setOpenModal] = useState(false)
    const [deleteId, setDeleteId] = useState<Identifier | undefined>()
    const [closeDrawer, setCloseDrawer] = useState("")
    const [recordId, setRecordId] = useState<Identifier | undefined>(undefined)
    const [page, setPage] = useState<number>(completed ? 2 : 0)
    const [showTrustees, setShowTrustees] = useState(false)

    interface IExpanded {
        [key: string]: boolean
    }
    const {data, isLoading, error, refetch} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: tallyId,
        }
    )

    const {data: elections} = useGetMany("sequent_backend_election", {
        ids: data?.election_ids || [],
    })

    console.log("TallyCeremony :: data :: ", data)
    console.log("TallyCeremony :: elections :: ", elections)

    const [expandedData, setExpandedData] = useState<IExpanded>({
        "election-data-general": true,
        "election-data-logs": true,
        "election-data-results": true,
    })

    const [expandedResults, setExpandedResults] = useState<IExpanded>({
        "election-data-logs": true,
        "election-data-results": true,
    })

    let rows: Array<Sequent_Backend_Election & {id: string; active: boolean}> = (elections || [])
        .map((election) => ({
            ...election,
            id: election.id || "",
            name: election.name,
            active: false,
        }))

    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: "Elections",
            flex: 1,
            editable: false,
        },
        {
            field: "active",
            headerName: "Selected",
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, boolean>) => (
                <Checkbox checked={props.value} />
            ),
        },
    ]

    useEffect(() => {
        if (recordId) {
            setOpen(true)
        }
    }, [recordId])

    const handleCloseCreateDrawer = () => {
        setRecordId(undefined)
        setCloseDrawer(new Date().toISOString())
    }

    const handleCloseEditDrawer = () => {
        setOpen(false)
        setTimeout(() => {
            setRecordId(undefined)
        }, 400)
    }

    const editAction = (id: Identifier) => {
        console.log("edit action", id)
        setRecordId(id)
    }

    const editDetail = (id: Identifier) => {
        setTallyId(id as string)
    }

    const deleteAction = (id: Identifier) => {
        // deleteOne("sequent_backend_area", {id})
        setOpenModal(true)
        setDeleteId(id)
    }

    const actions: Action[] = [
        {icon: <EditIcon />, action: editAction},
        {icon: <DeleteIcon />, action: deleteAction},
        {icon: <DescriptionIcon />, action: editDetail},
    ]

    const StyledHeader = styled.div`
        width: 100%;
        display: flex;
        padding: 2rem 0;
    `
    const StyledSpacing = styled.div`
        padding: 2rem 0;
    `

    const StyledFooter = styled.div`
        width: 100%;
        display: flex;
        justify-content: space-between;
        padding: 2rem 0;
    `

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

    const confirmNextAction = () => {
        setShowTrustees(true)
    }

    return (
        <>
            <StyledHeader>
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
            </StyledHeader>

            {page === 0 && (
                <>
                    <ElectionHeader
                        title={t("tally.ceremonyTitle")}
                        subtitle={t("tally.ceremonySubTitle")}
                    />

                    <DataGrid
                        rows={rows}
                        columns={columns}
                        initialState={{
                            pagination: {
                                paginationModel: {
                                    pageSize: 10,
                                },
                            },
                        }}
                        pageSizeOptions={[10, 20, 50, 100]}
                        disableRowSelectionOnClick
                    />

                    {showTrustees && (
                        <ElectionHeader
                            title={t("tally.trusteeTallyTitle")}
                            subtitle={t("tally.trusteeTallySubTitle")}
                        />
                    )}
                </>
            )}

            {page === 1 && (
                <>
                    <Accordion
                        sx={{width: "100%"}}
                        expanded={expandedData["election-data-general"]}
                        onChange={() =>
                            setExpandedData((prev: IExpanded) => ({
                                ...prev,
                                "election-data-general": !prev["election-data-general"],
                            }))
                        }
                    >
                        <AccordionSummary expandIcon={<ExpandMoreIcon id="tally-data-general" />}>
                            <ElectionStyles.Wrapper>
                                <ElectionHeader title={t("tally.tallyTitle")} subtitle="" />
                            </ElectionStyles.Wrapper>
                        </AccordionSummary>
                        <AccordionDetails>
                            {/* <Tabs value={value} onChange={handleChange}>
                                {renderTabs(parsedValue)}
                            </Tabs>
                            {renderTabContent(parsedValue)} */}
                        </AccordionDetails>
                    </Accordion>

                    <Accordion
                        sx={{width: "100%"}}
                        expanded={expandedData["election-data-logs"]}
                        onChange={() =>
                            setExpandedData((prev: IExpanded) => ({
                                ...prev,
                                "election-data-logs": !prev["election-data-logs"],
                            }))
                        }
                    >
                        <AccordionSummary expandIcon={<ExpandMoreIcon id="tally-data-logs" />}>
                            <ElectionStyles.Wrapper>
                                <ElectionHeader title={t("tally.logsTitle")} subtitle="" />
                            </ElectionStyles.Wrapper>
                        </AccordionSummary>
                        <AccordionDetails>
                            {/* <Tabs value={value} onChange={handleChange}>
                                {renderTabs(parsedValue)}
                            </Tabs>
                            {renderTabContent(parsedValue)} */}
                        </AccordionDetails>
                    </Accordion>

                    <Accordion
                        sx={{width: "100%"}}
                        expanded={expandedData["election-data-results"]}
                        onChange={() =>
                            setExpandedData((prev: IExpanded) => ({
                                ...prev,
                                "election-data-results": !prev["election-data-results"],
                            }))
                        }
                    >
                        <AccordionSummary expandIcon={<ExpandMoreIcon id="tally-data-results" />}>
                            <ElectionStyles.Wrapper>
                                <ElectionHeader title={t("tally.resultsTitle")} subtitle="" />
                            </ElectionStyles.Wrapper>
                        </AccordionSummary>
                        <AccordionDetails>
                            {/* <Tabs value={value} onChange={handleChange}>
                                {renderTabs(parsedValue)}
                            </Tabs>
                            {renderTabContent(parsedValue)} */}
                        </AccordionDetails>
                    </Accordion>
                </>
            )}

            {page === 2 && (
                <>
                    <StyledSpacing>
                        <ListActions withImport={false} withColumns={false} withFilter={false} />
                    </StyledSpacing>

                    <Accordion
                        sx={{width: "100%"}}
                        expanded={expandedResults["tally-results-logs"]}
                        onChange={() =>
                            setExpandedResults((prev: IExpanded) => ({
                                ...prev,
                                "tally-results-logs": !prev["tally-results-logs"],
                            }))
                        }
                    >
                        <AccordionSummary expandIcon={<ExpandMoreIcon id="tally-data-logs" />}>
                            <ElectionStyles.Wrapper>
                                <ElectionHeader title={t("tally.generalInfoTitle")} subtitle="" />
                            </ElectionStyles.Wrapper>
                        </AccordionSummary>
                        <AccordionDetails>
                            {/* <Tabs value={value} onChange={handleChange}>
                                {renderTabs(parsedValue)}
                            </Tabs>
                            {renderTabContent(parsedValue)} */}
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
                        <AccordionSummary expandIcon={<ExpandMoreIcon id="tally-data-results" />}>
                            <ElectionStyles.Wrapper>
                                <ElectionHeader title={t("tally.resultsTitle")} subtitle="" />
                            </ElectionStyles.Wrapper>
                        </AccordionSummary>
                        <AccordionDetails>
                            {/* <Tabs value={value} onChange={handleChange}>
                                {renderTabs(parsedValue)}
                            </Tabs>
                            {renderTabContent(parsedValue)} */}
                        </AccordionDetails>
                    </Accordion>
                </>
            )}

            <StyledFooter>
                <CancelButton className="list-actions" onClick={() => setTallyId(null)}>
                    {t("tally.common.cancel")}
                </CancelButton>
                {page < 2 && (
                    <NextButton color="primary" onClick={handleNext}>
                        <>
                            {t("tally.common.next")}
                            <ChevronRightIcon />
                        </>
                    </NextButton>
                )}
            </StyledFooter>

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
