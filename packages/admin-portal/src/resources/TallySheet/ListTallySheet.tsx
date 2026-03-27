// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useEffect, useMemo} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    Identifier,
    WrapperField,
    FunctionField,
    useGetList,
    SelectInput,
    RaRecord,
    useRefresh,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {ListActionsMenu} from "../../components/ListActionsMenu"
import {Button, Typography} from "@mui/material"
import {Sequent_Backend_Election, Sequent_Backend_Tally_Sheet} from "../../gql/graphql"
import {IconButton} from "@sequentech/ui-essentials"
import {Action} from "../../components/ActionButons"
import {useTranslation} from "react-i18next"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {WizardSteps} from "./TallySheetWizardCopy"
import {ContestItem} from "@/components/ContestItem"
import {AreaItem} from "@/components/AreaItem"
import {Add, WorkHistory} from "@mui/icons-material"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {AuthContext} from "@/providers/AuthContextProvider"
import {EStatus} from "@/types/TallySheets"
import SelectArea from "@/components/area/SelectArea"
import {votingChannels} from "./EditTallySheet"
import SelectContest from "@/components/contest/SelectContest"

const OMIT_FIELDS = ["id"]

interface TTallySheetList {
    election: Sequent_Backend_Election
    doAction: (action: number) => void
    setTallySheetRecord: React.Dispatch<React.SetStateAction<RaRecord<Identifier> | undefined>>
    tallySheetRecord?: RaRecord<Identifier>
    reload: string | null
    setShowVersionsTable: React.Dispatch<React.SetStateAction<boolean>>
}

export const ListTallySheet: React.FC<TTallySheetList> = (props) => {
    const {
        election: election,
        doAction,
        reload,
        setTallySheetRecord,
        tallySheetRecord,
        setShowVersionsTable,
    } = props

    const {t} = useTranslation()
    const [tenantId] = useTenantStore()

    const refresh = useRefresh()
    const {globalSettings} = useContext(SettingsContext)

    const authContext = useContext(AuthContext)
    const canCreate = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_CREATE)
    const canView = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_VIEW)

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
        doAction(WizardSteps.Configuration)
    }

    const addAction = (id: Identifier) => {
        localStorage.removeItem("tallySheetData")
        doAction(WizardSteps.Configuration)
    }

    const versionsTableAction = (id: Identifier) => {
        setShowVersionsTable(true)
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

    const Filters = useMemo(() => {
        let filters: ReactElement[] = []
        filters.push(
            <SelectArea
                tenantId={tenantId}
                electionEventId={election.election_event_id}
                source="area_id"
                label={String(t("usersAndRolesScreen.users.fields.area"))}
            />
        )
        filters.push(
            <SelectContest
                tenantId={tenantId}
                electionEventId={election.election_event_id}
                electionId={election.id}
                source="contest_id"
                label={String("Contest")}
            />
        )
        filters.push(
            <SelectInput
                source="channel"
                name="channel"
                label={String(t("tallysheet.label.channel"))}
                choices={votingChannels}
            />
        )
        return filters
    }, [tenantId])

    if (!canView) {
        return <Empty />
    }

    const actions: (record: Sequent_Backend_Tally_Sheet) => Action[] = (record) => [
        {
            icon: <Add />,
            action: addAction,
            showAction: () => canCreate,
            label: String(t("tallysheet.common.add")),
            saveRecordAction: setTallySheetRecord,
        },
        {
            icon: <WorkHistory />,
            action: versionsTableAction,
            showAction: () => canView,
            label: String(t("tallysheet.common.versions")),
            saveRecordAction: setTallySheetRecord,
        },
    ]

    return (
        <>
            <List
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
                }}
                resource="tally_sheet_by_latest_verison"
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
                }}
                filters={Filters}
                empty={<Empty />}
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

                    <TextField
                        source="version"
                        label={String(t("tallysheet.table.latestVersion"))}
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

                    <WrapperField source="actions" label="Actions">
                        <FunctionField
                            label={String(t("tallysheet.table.area"))}
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
