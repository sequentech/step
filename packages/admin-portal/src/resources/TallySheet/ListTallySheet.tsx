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
import {REVIEW_TALLY_SHEET} from "@/queries/reviewTallySheet"
import {ContestItem} from "@/components/ContestItem"
import {AreaItem} from "@/components/AreaItem"
import {Add, WorkHistory} from "@mui/icons-material"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {AuthContext} from "@/providers/AuthContextProvider"
import {EStatus} from "@/types/TallySheets"
import {ListTallySheetVersions} from "./ListTallySheetVersions"

const OMIT_FIELDS = ["id"]

const Filters: Array<ReactElement> = [
    <TextInput label="Area" source="area_id" key={0} />,
    <TextInput label="Contest" source="contest_id" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Channel" source="channel" key={3} />,
    <TextInput label="Latest version" source="version" key={4} />,
]

interface TTallySheetList {
    election: Sequent_Backend_Election
    doAction: (action: number, id?: Identifier) => void
    reload: string | null
}

export const ListTallySheet: React.FC<TTallySheetList> = (props) => {
    const {election: election, doAction, reload} = props

    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()
    const {globalSettings} = useContext(SettingsContext)
    const notify = useNotify()
    const [showVersionsTable, setShowVersionsTable] = React.useState(false)
    const [selectedTallySheet, setSelectedTallySheet] = React.useState<Sequent_Backend_Tally_Sheet | undefined>(undefined)
    const [openDisapproveDialog, setOpenDisapproveDialog] = React.useState(false)
    const [openApproveDialog, setOpenApproveDialog] = React.useState(false)
    const [tallySheetId, setTallySheetId] = React.useState<Identifier | undefined>()
    const [reviewTallySheet] = useMutation<ReviewTallySheetMutation>(REVIEW_TALLY_SHEET)

    const authContext = useContext(AuthContext)
    const canCreate = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_CREATE)
    const canView = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_VIEW)
    const canReview = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_REVIEW)

    const {data: sheetsDescVersions} = useGetList<Sequent_Backend_Tally_Sheet>(
        "sequent_backend_tally_sheet",
        {
            filter: {
                tenant_id: tenantId,
                election_event_id: election.election_event_id,
                election_id: election.id,
            },
            pagination: {
                page: 1,
                perPage: 100,
            },
            sort: {
                field: "version",
                order: "DESC",
            },
        }
    )

    const getLatestVersion = (area_id: string, contest_id: string, channel: string) => {
        const approvedVersion = sheetsDescVersions?.find(
            (sheet) =>
                sheet.area_id === area_id &&
                sheet.contest_id === contest_id &&
                sheet.channel === channel
        )
        return approvedVersion?.version ?? "-"
    }

    const getLatestApprovedVersion = (area_id: string, contest_id: string, channel: string) => {
        const approvedVersion = sheetsDescVersions?.find(
            (sheet) =>
                sheet.area_id === area_id &&
                sheet.contest_id === contest_id &&
                sheet.channel === channel &&
                sheet.status === EStatus.APPROVED
        )
        return approvedVersion?.version ?? "-"
    }

    useEffect(() => {
        localStorage.removeItem("tallySheetData")
    }, [])

    useEffect(() => {
        if (reload) {
            refresh()
        }
    }, [reload, refresh])

    const createAction = () => {
        localStorage.removeItem("tallySheetData")
        doAction(WizardSteps.Start)
    }

    const addAction = (id: Identifier) => {
        localStorage.removeItem("tallySheetData")
        doAction(WizardSteps.Edit, id)
    }

    const versionsTableAction = (id: Identifier) => {
        setShowVersionsTable(true)
        setTallySheetId(id)
        setSelectedTallySheet(sheetsDescVersions?.find((s) => s.id === id))
    }

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("tallysheet.empty.header")}
            </Typography>
            {canCreate && (
                <>
                    <Button onClick={createAction}>
                        <IconButton icon={faPlus} fontSize="24px" />
                        {t("tallysheet.empty.action")}
                    </Button>
                    <Typography variant="body1" paragraph>
                        {t("common.resources.noResult.askCreate")}
                    </Typography>
                </>
            )}
        </ResourceListStyles.EmptyBox>
    )

    if (!canView) {
        return <Empty />
    }

    const viewAction = (id: Identifier) => {
        doAction(WizardSteps.View, id)
    }

    const approveAction = (id: Identifier) => {
        setTallySheetId(id)
        setOpenApproveDialog(true)
    }

    const disapproveAction = (id: Identifier) => {
        setTallySheetId(id)
        setOpenDisapproveDialog(true)
    }

    const confirmReviewAction = async (newStatus: EStatus) => {
        const {data, errors} = await reviewTallySheet({
            variables: {
                electionEventId: election.election_event_id,
                tallySheetId: tallySheetId,
                newStatus,
            },
        })
        // if (data && !data?.publish_tally_sheet?.tally_sheet_id) {
        //     console.log("(unpublished) tally sheet not found, probably it's already published")
        // }
        if (errors) {
            // add error notification
            notify(t("tallysheet.message.reviewError"), {type: "error"})
        } else {
            notify(t("tallysheet.message.reviewSuccess"), {type: "success"})
        }
        setTallySheetId(undefined)
    }

    const actions: (record: Sequent_Backend_Tally_Sheet) => Action[] = (record) => [
        {
            icon: <Add />,
            action: addAction,
            showAction: () => canCreate,
            label: t("tallysheet.common.add"),
        },
        {
            icon: <WorkHistory />,
            action: versionsTableAction,
            showAction: () => canView,
            label: t("tallysheet.common.versions"),
        },
    ]

    return (
        <>
            { showVersionsTable && selectedTallySheet && (
                <ListTallySheetVersions
                    tallySheet={selectedTallySheet}
                    approveAction={approveAction}
                    disapproveAction={disapproveAction}
                    doAction={doAction}
                    reload={reload}
                />
            )}
            { !showVersionsTable && (
                <List
                    queryOptions={{
                        refetchInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
                    }}
                    resource="sequent_backend_tally_sheet"
                    actions={
                        <ListActions
                            withImport={false}
                            withExport={false}
                            extraActions={[
                                <Button key={0} onClick={createAction}>
                                    <Add />
                                    {t("tallysheet.empty.add")}
                                </Button>,
                            ]}
                        />
                    }
                    sx={{flexGrow: 2}}
                    filter={{
                        tenant_id: election.tenant_id || undefined,
                        election_event_id: election.election_event_id || undefined,
                        election_id: election.id || undefined,
                        version: 1,
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
                            label={t("tallysheet.table.latestVersion")}
                            render={(record: any) =>
                                getLatestVersion(
                                        record.area_id,
                                        record.contest_id,
                                        record.channel
                                    )
                            }
                        />
                        
                        <FunctionField
                            label={t("tallysheet.table.approvedVersion")}
                            render={(record: any) =>
                                getLatestApprovedVersion(
                                        record.area_id,
                                        record.contest_id,
                                        record.channel
                                    )
                            }
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
            )}
            <Dialog
                variant="warning"
                open={openDisapproveDialog}
                ok={t("tallysheet.common.disapprove")}
                cancel={t("common.label.cancel")}
                title={t("tallysheet.common.disapprove")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmReviewAction(EStatus.DISAPPROVED)
                    }
                    setOpenDisapproveDialog(false)
                }}
            >
                {t("tallysheet.common.warningDisapprove")}
            </Dialog>

            <Dialog
                variant="info"
                open={openApproveDialog}
                ok={t("tallysheet.common.approve")}
                cancel={t("common.label.cancel")}
                title={t("tallysheet.common.disapprove")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmReviewAction(EStatus.APPROVED)
                    }
                    setOpenApproveDialog(false)
                }}
            >
                {t("tallysheet.common.warningApprove")}
            </Dialog>
        </>
    )
}
