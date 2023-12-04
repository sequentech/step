// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useEffect} from "react"
import {
    Identifier,
    RaRecord,
    useRecordContext,
    Datagrid,
    Button,
} from "react-admin"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
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

export const TallyCeremony: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const [tallyId, setTallyId] = useElectionEventTallyStore()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [closeDrawer, setCloseDrawer] = React.useState("")
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)



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
        setOpenDeleteModal(true)
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

    const StyledFooter = styled.div`
        width: 100%;
        display: flex;
        justify-content: space-between;
        padding: 2rem 0;
    `

    return (
        <>
            <StyledHeader>
                <BreadCrumbSteps
                    labels={[
                        "tally.breadcrumbSteps.ceremony",
                        "tally.breadcrumbSteps.tally",
                        "tally.breadcrumbSteps.results",
                    ]}
                    selected={0}
                    variant={BreadCrumbStepsVariant.Circle}
                    colorPreviousSteps={true}
                />
            </StyledHeader>

            <ElectionHeader
                title={t("tally.electionTallyTitle")}
                subtitle={t("tally.electionTallySubTitle")}
            />

            <ElectionHeader
                title={t("tally.trusteeTallyTitle")}
                subtitle={t("tally.trusteeTallySubTitle")}
            />

            <StyledFooter>
                <Button onClick={() => setTallyId(null)}>
                    {t("electionEventScreen.tabs.tally.ceremony.start")}
                </Button>
                <Button color="primary" onClick={() => setOpen(true)}>
                    <>
                        {t("electionEventScreen.tabs.tally.ceremony.start")}
                        <ChevronRightIcon />
                    </>
                </Button>
            </StyledFooter>
        </>
    )
}
