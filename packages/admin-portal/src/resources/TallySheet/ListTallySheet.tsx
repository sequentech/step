// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
import {Button, Chip, Tooltip, Typography} from "@mui/material"
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
    <TextInput label="Labels" source="labels" key={5} />,
    <TextInput label="Annotations" source="annotations" key={6} />,
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
    const [selectedTallySheet, setSelectedTallySheet] = React.useState<
        Sequent_Backend_Tally_Sheet | undefined
    >(undefined)
    const [openDisapproveDialog, setOpenDisapproveDialog] = React.useState(false)
    const [openApproveDialog, setOpenApproveDialog] = React.useState(false)
    const [tallySheetId, setTallySheetId] = React.useState<Identifier | undefined>()
    const [reviewTallySheet] = useMutation<ReviewTallySheetMutation>(REVIEW_TALLY_SHEET, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.TALLY_SHEET_REVIEW,
            },
        },
    })

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
                deleted_at: {
                    format: "hasura-raw-query",
                    value: {_is_null: true},
                },
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
        const selected = sheetsDescVersions?.find((s) => {
            return s.id === id
        })
        const latestVersion = sheetsDescVersions?.find((s) => {
            return (
                s.area_id === selected?.area_id &&
                s.contest_id === selected?.contest_id &&
                s.channel === selected?.channel
            )
        })
        doAction(WizardSteps.Edit, latestVersion?.id)
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
                        <IconButton icon={faPlus as any} fontSize="24px" />
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
            icon: (
                <Tooltip title={String(t("tallysheet.common.add"))}>
                    <Add />
                </Tooltip>
            ),
            action: addAction,
            showAction: () => canCreate,
            label: String(t("tallysheet.common.add")),
        },
        {
            icon: (
                <Tooltip title={String(t("tallysheet.common.versions"))}>
                    <WorkHistory />
                </Tooltip>
            ),
            action: versionsTableAction,
            showAction: () => canView,
            label: String(t("tallysheet.common.versions")),
        },
    ]

    return (
        <>
            {showVersionsTable && selectedTallySheet && (
                <ListTallySheetVersions
                    tallySheet={selectedTallySheet}
                    approveAction={approveAction}
                    disapproveAction={disapproveAction}
                    doAction={doAction}
                    reload={reload}
                    setShowVersionsTable={setShowVersionsTable}
                />
            )}
            {!showVersionsTable && (
                <List
                    disableSyncWithLocation
                    queryOptions={{
                        refetchInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
                        meta: {distinctBallotBoxes: true},
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
                        deleted_at: {
                            format: "hasura-raw-query",
                            value: {_is_null: true},
                        },
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
                        <TextField source="id" sortable={false} />
                        <FunctionField
                            source="channel"
                            sortable={false}
                            label={String(t("tallysheet.label.channel"))}
                            render={(record: Sequent_Backend_Tally_Sheet) => (
                                <Chip label={record.channel} />
                            )}
                        />

                        <FunctionField
                            label={String(t("tallysheet.table.contest"))}
                            render={(record: Sequent_Backend_Tally_Sheet) => (
                                <ContestItem record={record.contest_id} />
                            )}
                        />

                        <FunctionField
                            label={String(t("tallysheet.table.area"))}
                            render={(record: Sequent_Backend_Tally_Sheet) => (
                                <AreaItem record={record.area_id} />
                            )}
                        />

                        <FunctionField
                            label={String(t("tallysheet.table.latestVersion"))}
                            render={(record: Sequent_Backend_Tally_Sheet) =>
                                getLatestVersion(
                                    record.area_id,
                                    record.contest_id,
                                    record.channel as string
                                )
                            }
                        />

                        <FunctionField
                            label={String(t("tallysheet.table.approvedVersion"))}
                            render={(record: Sequent_Backend_Tally_Sheet) =>
                                getLatestApprovedVersion(
                                    record.area_id as string,
                                    record.contest_id as string,
                                    record.channel as string
                                )
                            }
                        />

                        <FunctionField
                            source="labels"
                            label={String(t("tallysheet.table.labels"))}
                            render={(record: Sequent_Backend_Tally_Sheet) =>
                                formatJsonValue(record.labels)
                            }
                        />

                        <FunctionField
                            source="annotations"
                            label={String(t("tallysheet.table.annotations"))}
                            render={(record: Sequent_Backend_Tally_Sheet) =>
                                formatJsonValue(record.annotations)
                            }
                        />

                        <WrapperField source="actions" label="Actions">
                            <FunctionField
                                label={String(t("tallysheet.table.area"))}
                                render={(record: Sequent_Backend_Tally_Sheet) => (
                                    <ActionsColumn actions={actions(record)} />
                                )}
                            />
                        </WrapperField>
                    </DatagridConfigurable>
                </List>
            )}
            <Dialog
                variant="warning"
                open={openDisapproveDialog}
                ok={String(t("tallysheet.common.disapprove"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("tallysheet.common.disapprove"))}
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
                ok={String(t("tallysheet.common.approve"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("tallysheet.common.approve"))}
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

const formatJsonValue = (value: unknown) => {
    if (value === null || value === undefined || value === "") {
        return "-"
    }
    if (typeof value === "string") {
        return value
    }
    try {
        return JSON.stringify(value)
    } catch (_error) {
        return String(value)
    }
}
