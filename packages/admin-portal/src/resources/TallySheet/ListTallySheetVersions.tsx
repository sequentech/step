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
    useNotify,
    SelectInput,
    RaRecord,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {ListActionsMenu} from "../../components/ListActionsMenu"
import {Tooltip, Typography} from "@mui/material"
import {
    ReviewTallySheetMutation,
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Sheet,
} from "../../gql/graphql"
import {Dialog} from "@sequentech/ui-essentials"
import {Action} from "../../components/ActionButons"
import {useTranslation} from "react-i18next"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import VisibilityIcon from "@mui/icons-material/Visibility"
import UnpublishedIcon from "@mui/icons-material/Unpublished"
import PublishedWithChangesIcon from "@mui/icons-material/PublishedWithChanges"
import {WizardSteps} from "./TallySheetWizardCopy"
import {useMutation} from "@apollo/client"
import {ContestItem} from "@/components/ContestItem"
import {AreaItem} from "@/components/AreaItem"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {AuthContext} from "@/providers/AuthContextProvider"
import {EStatus} from "@/types/TallySheets"
import {WizardStyles} from "@/components/styles/WizardStyles"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {REVIEW_TALLY_SHEET} from "@/queries/reviewTallySheet"

const OMIT_FIELDS = ["id"]

export const tallySheetStatusChoices = [
    {id: "PENDING", name: "PENDING"},
    {id: "APPROVED", name: "APPROVED"},
    {id: "DISAPPROVED", name: "DISAPPROVED"},
]

const Filters: Array<ReactElement> = [
    <TextInput label="Version" source="version" key={1} />,
    <SelectInput label="Status" source="status" key={4} choices={tallySheetStatusChoices} />,
]
interface TTallySheetListVersions {
    election: Sequent_Backend_Election
    tallySheet: Sequent_Backend_Tally_Sheet
    doAction: (action: number, id?: Identifier) => void
    setShowVersionsTable: (show: boolean) => void
    setTallySheetRecord: React.Dispatch<React.SetStateAction<RaRecord<Identifier> | undefined>>
}

export const ListTallySheetVersions: React.FC<TTallySheetListVersions> = (props) => {
    const {election, tallySheet, doAction, setShowVersionsTable, setTallySheetRecord} = props

    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)
    const notify = useNotify()
    const [openDisapproveDialog, setOpenDisapproveDialog] = React.useState(false)
    const [openApproveDialog, setOpenApproveDialog] = React.useState(false)

    const authContext = useContext(AuthContext)
    const canView = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_VIEW)
    const canReview = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_REVIEW)

    const [reviewTallySheet] = useMutation<ReviewTallySheetMutation>(REVIEW_TALLY_SHEET)

    const viewAction = (id: Identifier) => {
        doAction(WizardSteps.Review)
    }

    const approveAction = (id: Identifier) => {
        setOpenApproveDialog(true)
    }

    const disapproveAction = (id: Identifier) => {
        setOpenDisapproveDialog(true)
    }

    const actions: (record: Sequent_Backend_Tally_Sheet) => Action[] = (record) => [
        {
            icon: <VisibilityIcon />,
            action: viewAction,
            showAction: () => canView,
            label: t("tallysheet.common.show"),
            saveRecordAction: setTallySheetRecord,
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
            saveRecordAction: setTallySheetRecord,
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
            saveRecordAction: setTallySheetRecord,
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

    const confirmReviewAction = async (newStatus: EStatus) => {
        const {data, errors} = await reviewTallySheet({
            variables: {
                electionEventId: election.election_event_id,
                tallySheetId: tallySheet?.id,
                newStatus,
            },
        })
        if (errors) {
            notify(t("tallysheet.message.reviewError"), {type: "error"})
        } else {
            notify(t("tallysheet.message.reviewSuccess"), {type: "success"})
        }
    }

    return (
        <>
            <List
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
                sort={{field: "version", order: "ASC"}}
            >
                <DatagridConfigurable
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
                        label={t("tallysheet.table.contest")}
                        render={(record: Sequent_Backend_Tally_Sheet) => (
                            <ContestItem record={record.contest_id} />
                        )}
                    />

                    <FunctionField
                        label={t("tallysheet.table.area")}
                        render={(record: Sequent_Backend_Tally_Sheet) => (
                            <AreaItem record={record.area_id} />
                        )}
                    />

                    <TextField source="version" label={t("tallysheet.versionsTable.version")} />

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
                title={String(t("tallysheet.common.disapprove"))}
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
