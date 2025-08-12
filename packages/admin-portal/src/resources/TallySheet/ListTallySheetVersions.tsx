// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useEffect} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    TextInput,
    Identifier,
    useDelete,
    WrapperField,
    FunctionField,
    useRefresh,
    useNotify,
    useGetList,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {ListActionsMenu} from "../../components/ListActionsMenu"
import {Button, Tooltip, Typography} from "@mui/material"
import {
    ReviewTallySheetMutation,
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Sheet,
} from "../../gql/graphql"
import {Dialog, IconButton} from "@sequentech/ui-essentials"
import {Action, ActionsColumn} from "../../components/ActionButons"
import {useTranslation} from "react-i18next"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import VisibilityIcon from "@mui/icons-material/Visibility"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import UnpublishedIcon from "@mui/icons-material/Unpublished"
import PublishedWithChangesIcon from "@mui/icons-material/PublishedWithChanges"
import {WizardSteps} from "./TallySheetWizard"
import {useMutation} from "@apollo/client"
import {ContestItem} from "@/components/ContestItem"
import {AreaItem} from "@/components/AreaItem"
import {Add, WorkHistory} from "@mui/icons-material"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {AuthContext} from "@/providers/AuthContextProvider"
import {EStatus} from "@/types/TallySheets"
import { channel } from "diagnostics_channel"

const OMIT_FIELDS = ["id, area_id"]

const Filters: Array<ReactElement> = [
    <TextInput label="Area" source="area_id" key={0} />,
    <TextInput label="Contest" source="contest_id" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Channel" source="channel" key={3} />,
    <TextInput label="Version" source="version" key={4} />,
    <TextInput label="Created by" source="created_by" key={5} />,
    <TextInput label="Reviewed by" source="reviewed_by" key={6} />,
]

interface TTallySheetListVersions {
    tallySheet: Sequent_Backend_Tally_Sheet
    approveAction: (id: Identifier) => void
    disapproveAction: (id: Identifier) => void
    doAction: (action: number, id?: Identifier) => void
    reload: string | null
}

export const ListTallySheetVersions: React.FC<TTallySheetListVersions> = (props) => {
    const {tallySheet: tallySheet, doAction, reload, approveAction, disapproveAction} = props

    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()
    const {globalSettings} = useContext(SettingsContext)
    const notify = useNotify()
    const [openDisapproveDialog, setOpenDisapproveDialog] = React.useState(false)
    const [openApproveDialog, setOpenApproveDialog] = React.useState(false)
    const [tallySheetId, setTallySheetId] = React.useState<Identifier | undefined>()

    const authContext = useContext(AuthContext)
    const canView = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_VIEW)
    const canReview = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_REVIEW)

    const viewAction = (id: Identifier) => {
        doAction(WizardSteps.View, id)
    }

    const actions: (record: Sequent_Backend_Tally_Sheet) => Action[] = (record) => [
        {
            icon: <VisibilityIcon />,
            action: viewAction,
            showAction: () => canView,
            label: t("tallysheet.common.show"),
        },
        {
            icon: (
                <Tooltip title={t("tallysheet.common.approve")}>
                    <PublishedWithChangesIcon />
                </Tooltip>
            ),
            action: approveAction,
            showAction: () => canReview && record.status === EStatus.PENDING,
            label: t("tallysheet.common.approve"),
        },
        {
            icon: (
                <Tooltip title={t("tallysheet.common.disapprove")}>
                    <UnpublishedIcon />
                </Tooltip>
            ),
            action: disapproveAction,
            showAction: () => canReview && record.status === EStatus.PENDING,
            label: t("tallysheet.common.disapprove"),
        },
    ]
    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("tallysheet.empty.header")}
            </Typography>
        </ResourceListStyles.EmptyBox>
    )

    return (
        <>
            <List
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
                }}
                resource="sequent_backend_tally_sheet"
                actions={
                    <ListActions
                        withImport={false}
                        withExport={false}
                    />
                }
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: tallySheet.tenant_id || undefined,
                    election_event_id: tallySheet.election_event_id || undefined,
                    election_id: tallySheet.election_id || undefined,
                    area_id: tallySheet.area_id || undefined,
                    contest_id: tallySheet.contest_id || undefined,
                }}
                filters={Filters}
                empty={<Empty />}
            >
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="channel" />

                    <FunctionField
                        label={t("tallysheet.table.contest")}
                        render={(record: any) => <ContestItem record={record.contest_id} />}
                    />

                    <FunctionField
                        label={t("tallysheet.table.area")}
                        render={(record: Sequent_Backend_Tally_Sheet) => (
                            <AreaItem record={record.area_id} />
                        )}
                    />

                    <FunctionField
                        key={"Version"}
                        label={t("tallysheet.versionsTable.version")}
                        render={(record: any) => <TextField source="version" />}
                    />

                    <FunctionField
                        key={"Created by"}
                        label={t("tallysheet.versionsTable.createdBy")}
                        render={(record: any) => <TextField source="created_by" />}
                    />
                    <FunctionField
                        key={"Reviewed by"}
                        label={t("tallysheet.versionsTable.reviewedBy")}
                        render={(record: any) => <TextField source="reviewed_by" />}
                    />

                    <WrapperField source="actions" label="Actions">
                        <FunctionField
                            label={t("tallysheet.table.area")}
                            render={(record: Sequent_Backend_Tally_Sheet) => (
                                <ListActionsMenu actions={actions(record)} />
                            )}
                        />
                    </WrapperField>
                </DatagridConfigurable>
            </List>
        </>
    )
}
