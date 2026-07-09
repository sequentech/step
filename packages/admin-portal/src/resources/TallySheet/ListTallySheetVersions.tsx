// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useMemo} from "react"
import {
    DatagridConfigurable,
    FunctionField,
    Identifier,
    List,
    TextField,
    TextInput,
    WrapperField,
    useGetMany,
    useListContext,
    useNotify,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {IconButton, Stack, Box, Chip, Tooltip, Typography} from "@mui/material"
import {useLazyQuery} from "@apollo/client"
import DownloadIcon from "@mui/icons-material/Download"
import OpenInNewIcon from "@mui/icons-material/OpenInNew"
import {FetchDocumentQuery, Sequent_Backend_Tally_Sheet} from "../../gql/graphql"
import {Action, ActionsColumn} from "../../components/ActionButons"
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
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {downloadUrl} from "@sequentech/ui-core"
import {useLocation, useNavigate} from "react-router-dom"

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
    <TextInput label="Import" source="import_id" key={8} />,
    <TextInput label="Labels" source="labels" key={9} />,
    <TextInput label="Annotations" source="annotations" key={10} />,
]

type TallySheetVersionRecord = Sequent_Backend_Tally_Sheet & {
    import_id?: string | null
}

interface TallySheetImportReference {
    id: string
    status: string
    source_document_id?: string | null
    source_file_name?: string | null
}

const ImportedVersionSourceContext = React.createContext<Map<string, TallySheetImportReference>>(
    new Map<string, TallySheetImportReference>()
)

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
        approveAction,
        disapproveAction,
        setShowVersionsTable,
    } = props

    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)

    const authContext = useContext(AuthContext)
    const canView = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_VIEW)
    const canReview = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_REVIEW)
    const canViewImport = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.TALLY_SHEET_IMPORT_VIEW
    )

    const viewAction = (id: Identifier) => {
        doAction(WizardSteps.View, id)
    }

    const actions: (record: Sequent_Backend_Tally_Sheet) => Action[] = (record) => [
        {
            icon: (
                <Tooltip title={t("tallysheet.common.show")}>
                    <VisibilityIcon />
                </Tooltip>
            ),
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
                <ImportedVersionSourceContextProvider canViewImport={canViewImport}>
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
                                record.reviewed_at ? (
                                    <TextField source="reviewed_by_user_id" />
                                ) : (
                                    "-"
                                )
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

                        <FunctionField
                            source="labels"
                            label={t("tallysheet.table.labels")}
                            render={(record: Sequent_Backend_Tally_Sheet) =>
                                formatJsonValue(record.labels)
                            }
                        />

                        <FunctionField
                            source="annotations"
                            label={t("tallysheet.table.annotations")}
                            render={(record: Sequent_Backend_Tally_Sheet) =>
                                formatJsonValue(record.annotations)
                            }
                        />

                        <FunctionField
                            key="source-import"
                            label={t("tallysheet.versionsTable.sourceImport")}
                            render={(record: TallySheetVersionRecord) => (
                                <ImportedVersionSource
                                    record={record}
                                    electionEventId={String(tallySheet.election_event_id)}
                                    canViewImport={canViewImport}
                                />
                            )}
                        />

                        <WrapperField source="actions" label="Actions">
                            <FunctionField
                                label={t("tallysheet.table.area")}
                                render={(record: Sequent_Backend_Tally_Sheet) => (
                                    <ActionsColumn actions={actions(record)} />
                                )}
                            />
                        </WrapperField>
                    </DatagridConfigurable>
                </ImportedVersionSourceContextProvider>
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

const ImportedVersionSourceContextProvider: React.FC<{
    children: React.ReactNode
    canViewImport: boolean
}> = ({children, canViewImport}) => {
    const {data: versions = []} = useListContext<TallySheetVersionRecord>()
    const importIds = useMemo(
        () =>
            Array.from(
                new Set(
                    versions
                        .map((version) => version.import_id)
                        .filter((importId): importId is string => !!importId)
                )
            ),
        [versions]
    )

    const {data: imports = []} = useGetMany<TallySheetImportReference>(
        "sequent_backend_tally_sheet_import",
        {ids: importIds},
        {enabled: canViewImport && importIds.length > 0}
    )

    const importsById = useMemo(() => {
        const importMap = new Map<string, TallySheetImportReference>()
        imports.forEach((item) => {
            importMap.set(item.id, item)
        })
        return importMap
    }, [imports])

    return (
        <ImportedVersionSourceContext.Provider value={importsById}>
            {children}
        </ImportedVersionSourceContext.Provider>
    )
}

interface ImportedVersionSourceProps {
    record: TallySheetVersionRecord
    electionEventId: string
    canViewImport: boolean
}

const ImportedVersionSource: React.FC<ImportedVersionSourceProps> = ({
    record,
    electionEventId,
    canViewImport,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const location = useLocation()
    const navigate = useNavigate()
    const importId = record.import_id
    const importReferencesById = useContext(ImportedVersionSourceContext)
    const sourceImport = importId ? importReferencesById.get(importId) : undefined
    const [fetchDocument] = useLazyQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        fetchPolicy: "no-cache",
    })

    if (!importId || !canViewImport) {
        return <Typography variant="body2">-</Typography>
    }

    const openImport = (event: React.MouseEvent<HTMLButtonElement>) => {
        event.stopPropagation()
        const nextSearch = new URLSearchParams(location.search)
        nextSearch.set("tabId", "tally-sheet-imports")
        nextSearch.set("tallySheetImportId", importId)
        navigate({pathname: location.pathname, search: `?${nextSearch.toString()}`})
    }

    const downloadSource = async () => {
        if (!sourceImport?.source_document_id) {
            return
        }

        try {
            const {data} = await fetchDocument({
                variables: {
                    electionEventId,
                    documentId: sourceImport.source_document_id,
                },
            })
            const url = data?.fetchDocument?.url
            if (!url) {
                throw new Error(String(t("tallySheetImport.notifications.sourceUrlError")))
            }
            await downloadUrl(
                url,
                sourceImport.source_file_name || `tally-sheet-import-${importId}`
            )
        } catch (error) {
            notify(
                error instanceof Error
                    ? error.message
                    : t("tallySheetImport.notifications.sourceDownloadError"),
                {type: "error"}
            )
        }
    }

    return (
        <Stack gap={0.5}>
            <Typography variant="caption" color="text.secondary">
                {t("tallysheet.versionsTable.importStatus")}: {sourceImport?.status || "-"}
            </Typography>
            <Stack direction="row" gap={1} flexWrap="wrap">
                <Tooltip title={String(t("tallysheet.versionsTable.openImport"))}>
                    <IconButton size="small" onClick={openImport}>
                        <OpenInNewIcon fontSize="small" />
                    </IconButton>
                </Tooltip>
                <Tooltip title={String(t("tallysheet.versionsTable.sourceFile"))}>
                    <span>
                        <IconButton
                            size="small"
                            onClick={downloadSource}
                            disabled={!sourceImport?.source_document_id}
                        >
                            <DownloadIcon fontSize="small" />
                        </IconButton>
                    </span>
                </Tooltip>
            </Stack>
        </Stack>
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
