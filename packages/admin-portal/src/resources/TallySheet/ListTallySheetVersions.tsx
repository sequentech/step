// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    TextInput,
    Identifier,
    WrapperField,
    FunctionField,
    useRefresh,
    useNotify,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {ListActionsMenu} from "../../components/ListActionsMenu"
import {Box, Chip, Tooltip, Typography} from "@mui/material"
import {Sequent_Backend_Tally_Sheet} from "../../gql/graphql"
import {Action} from "../../components/ActionButons"
import {useTranslation} from "react-i18next"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import VisibilityIcon from "@mui/icons-material/Visibility"
import UnpublishedIcon from "@mui/icons-material/Unpublished"
import PublishedWithChangesIcon from "@mui/icons-material/PublishedWithChanges"
import {WizardSteps} from "./TallySheetWizard"
import {ContestItem} from "@/components/ContestItem"
import {AreaItem} from "@/components/AreaItem"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {AuthContext} from "@/providers/AuthContextProvider"
import {EStatus} from "@/types/TallySheets"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"

const OMIT_FIELDS = ["id", "channel", "area_id", "contest_id"]

const Filters: Array<ReactElement> = [
    <TextInput label="Area" source="area_id" key={0} />,
    <TextInput label="Contest" source="contest_id" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Channel" source="channel" key={3} />,
    <TextInput label="Version" source="version" key={4} />,
    <TextInput label="Created by" source="created_by_user_id" key={5} />,
    <TextInput label="Reviewed by" source="reviewed_by_user_id" key={6} />,
    <TextInput label="Status" source="status" key={7} />,
]

interface TTallySheetListVersions {
    tallySheet: Sequent_Backend_Tally_Sheet
    approveAction: (id: Identifier) => void
    disapproveAction: (id: Identifier) => void
    doAction: (action: number, id?: Identifier) => void
    reload: string | null
    setShowVersionsTable: (show: boolean) => void
}

export const ListTallySheetVersions: React.FC<TTallySheetListVersions> = (props) => {
    const {
        tallySheet: tallySheet,
        doAction,
        reload,
        approveAction,
        disapproveAction,
        setShowVersionsTable,
    } = props

    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()
    const {globalSettings} = useContext(SettingsContext)
    const notify = useNotify()

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
    function goBack() {
        setShowVersionsTable(false)
    }

    return (
        <>
            <ElectionHeaderStyles.Wrapper>
                <Box display="flex" alignItems="center" gap={1}>
                    <ElectionHeaderStyles.SubTitle>
                        {t("tallysheet.versionsTable.title")}
                    </ElectionHeaderStyles.SubTitle>
                    <Chip label={tallySheet.channel} />
                    <AreaItem record={tallySheet.area_id} />
                    <ContestItem record={tallySheet.contest_id} />
                </Box>
            </ElectionHeaderStyles.Wrapper>
            <List
                disableSyncWithLocation
                sort={{field: "version", order: "DESC"}}
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
                }}
                resource="sequent_backend_tally_sheet"
                actions={<ListActions withImport={false} withExport={false} />}
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: tallySheet.tenant_id || undefined,
                    election_event_id: tallySheet.election_event_id || undefined,
                    election_id: tallySheet.election_id || undefined,
                    area_id: tallySheet.area_id || undefined,
                    contest_id: tallySheet.contest_id || undefined,
                    channel: tallySheet.channel || undefined,
                }}
                filters={Filters}
                empty={<Empty />}
            >
                <DatagridConfigurable
                    bulkActionButtons={false}
                    omit={OMIT_FIELDS}
                    sx={{
                        flexGrow: 1,
                        overflowX: "auto",
                        width: "100%",
                        maxWidth: "100%",
                    }}
                >
                    <TextField source="id" />
                    <TextField source="channel" />

                    <FunctionField
                        source="contest_id"
                        label={t("tallysheet.table.contest")}
                        render={(record: Sequent_Backend_Tally_Sheet) => (
                            <ContestItem record={record.contest_id} />
                        )}
                    />

                    <FunctionField
                        source="area_id"
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
                        render={(record: any) => <TextField source="created_by_user_id" />}
                    />
                    <TextField source="created_at" />

                    <FunctionField
                        key={"Reviewed by"}
                        label={t("tallysheet.versionsTable.reviewedBy")}
                        render={(record: Sequent_Backend_Tally_Sheet) =>
                            record.reviewed_at ? <TextField source="reviewed_by_user_id" /> : "-"
                        }
                    />

                    <FunctionField
                        key={"reviewed_at"}
                        label={t("tallysheet.versionsTable.reviewedAt")}
                        render={(record: any) =>
                            record.reviewed_at ? <TextField source="reviewed_at" /> : "-"
                        }
                    />
                    <TextField source="status" />

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
            <WizardStyles.Toolbar>
                <WizardStyles.BackButton
                    color="info"
                    onClick={goBack}
                    className="ts-versions-back-button"
                >
                    <ArrowBackIosIcon />
                    {t("common.label.back")}
                </WizardStyles.BackButton>
            </WizardStyles.Toolbar>
        </>
    )
}
