// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Identifier, RaRecord, useRecordContext, Datagrid} from "react-admin"
import Button from "@mui/material/Button"

import {Sequent_Backend_Election_Event} from "../../gql/graphql"
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
import {GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import Checkbox from "@mui/material/Checkbox"
import {Accordion, AccordionDetails, AccordionSummary} from "@mui/material"
import {ElectionStyles} from "@/components/styles/ElectionStyles"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {ListActions} from "@/components/ListActions"

export const TallyCeremony: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const [tallyId, setTallyId] = useElectionEventTallyStore()

    const [open, setOpen] = useState(false)
    const [openModal, setOpenModal] = useState(false)
    const [deleteId, setDeleteId] = useState<Identifier | undefined>()
    const [closeDrawer, setCloseDrawer] = useState("")
    const [recordId, setRecordId] = useState<Identifier | undefined>(undefined)
    const [page, setPage] = useState<number>(0)
    const [showTrustees, setShowTrustees] = useState(false)
    const [expanded, setExpanded] = useState("tally-data-tally")

    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: "Permission",
            width: 350,
            editable: false,
        },
        {
            field: "active",
            headerName: "Active",
            width: 70,
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
                    <Accordion sx={{width: "100%"}}>
                        <AccordionSummary expandIcon={<ExpandMoreIcon id="tally-data-tally" />}>
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

                    <Accordion sx={{width: "100%"}}>
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

                    <Accordion sx={{width: "100%"}}>
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

                    <Accordion sx={{width: "100%"}}>
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

                    <Accordion sx={{width: "100%"}}>
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
                <NextButton color="primary" onClick={handleNext}>
                    <>
                        {t("tally.common.next")}
                        <ChevronRightIcon />
                    </>
                </NextButton>
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
