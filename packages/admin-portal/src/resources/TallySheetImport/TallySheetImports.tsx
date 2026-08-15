// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, useCallback, useContext, useEffect, useMemo, useState} from "react"
import {useLazyQuery, useMutation, useQuery} from "@apollo/client"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import UploadFileIcon from "@mui/icons-material/UploadFile"
import VisibilityIcon from "@mui/icons-material/Visibility"
import DoneIcon from "@mui/icons-material/Done"
import CloseIcon from "@mui/icons-material/Close"
import DownloadIcon from "@mui/icons-material/Download"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Alert,
    Box,
    Button,
    Chip,
    CircularProgress,
    Divider,
    Drawer,
    FormControl,
    IconButton,
    InputLabel,
    MenuItem,
    Select,
    SelectChangeEvent,
    Stack,
    Tooltip,
    Typography,
} from "@mui/material"
import {diffLines} from "diff"
import {
    Button as RaButton,
    DatagridConfigurable,
    FunctionField,
    List,
    TextField,
    TextInput,
    WrapperField,
    useGetList,
    useGetOne,
    useListContext,
    useNotify,
    useRecordContext,
    useRefresh,
} from "react-admin"
import {useTranslation} from "react-i18next"
import ElectionHeader from "@/components/ElectionHeader"
import {ListActions} from "@/components/ListActions"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {
    CREATE_TALLY_SHEET_IMPORT,
    PREVIEW_TALLY_SHEET_IMPORT,
    REVIEW_TALLY_SHEET_IMPORT,
} from "@/queries/TallySheetImport"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {ThreeStateDatagridHeader} from "@/components/ThreeStateDatagridHeader"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {
    FetchDocumentQuery,
    GetUploadUrlMutation,
    GetUploadUrlMutationVariables,
    PreviewTallySheetImportMutationVariables,
    ReviewTallySheetImportMutationVariables,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
import {IPermissions} from "@/types/keycloak"
import {
    TALLY_SHEET_VOTING_CHANNELS,
    TallySheetVotingChannel,
    downloadUrl,
    isTallySheetVotingChannel,
} from "@sequentech/ui-core"
import {DropFile} from "@sequentech/ui-essentials"
import {LIST_USERS} from "@/queries/GetUsers"
import {
    ETallySheetImportChangeType,
    ETallySheetImportItemStatus,
    ETallySheetImportReviewDecision,
    ETallySheetImportSourceFormat,
    ETallySheetImportStatus,
} from "@/types/TallySheets"

type GetUploadUrlData = GetUploadUrlMutation
type GetUploadUrlVariables = GetUploadUrlMutationVariables

interface TallySheetImportSummary {
    imported_ballot_box_count: number
    changed_ballot_box_count: number
    new_ballot_box_count: number
    unchanged_ballot_box_count: number
    conflicted_ballot_box_count: number
    validation_error_count: number
}

interface TallySheetImportValidationError {
    code: string
    message: string
    channel?: TallySheetVotingChannel | null
    area_name?: string | null
    contest_external_id?: string | null
    candidate_external_id?: string | null
    field?: string | null
}

interface TallySheetImportPreviewItem {
    channel: TallySheetVotingChannel
    area_id: string
    area_name: string
    contest_id: string
    contest_name: string
    election_id: string
    baseline_tally_sheet_id?: string | null
    baseline_version?: number | null
    previous_csv?: string | null
    incoming_csv: string
    incoming_content_hash: string
    change_type: ETallySheetImportChangeType
}

interface TallySheetImportPreview {
    document_id: string
    source_format: ETallySheetImportSourceFormat
    selected_channel: TallySheetVotingChannel
    summary: TallySheetImportSummary
    items: TallySheetImportPreviewItem[]
    validation_errors: TallySheetImportValidationError[]
}

// The `preview`/`import` action fields are opaque jsonb in the Hasura schema
// (see PREVIEW_TALLY_SHEET_IMPORT et al.), so codegen types them as `any`;
// only their envelope shape (and mutation variables) come from codegen.
interface PreviewTallySheetImportData {
    preview_tally_sheet_import?: {
        preview: TallySheetImportPreview
    } | null
}

interface CreateTallySheetImportData {
    create_tally_sheet_import?: {
        import: TallySheetImportRecord
    } | null
}

interface ReviewTallySheetImportData {
    review_tally_sheet_import?: {
        import: TallySheetImportRecord
    } | null
}

// PreviewTallySheetImportMutationVariables and CreateTallySheetImportMutationVariables
// share the same shape (electionEventId, documentId, sha256, sourceFormat, selectedChannel).
type TallySheetImportVariables = Omit<
    PreviewTallySheetImportMutationVariables,
    "sourceFormat" | "selectedChannel"
> & {
    sourceFormat: ETallySheetImportSourceFormat
    selectedChannel: TallySheetVotingChannel
}

type ReviewTallySheetImportVariables = Omit<ReviewTallySheetImportMutationVariables, "decision"> & {
    decision: ETallySheetImportReviewDecision
}

interface UserSummary {
    id?: string | null
    username?: string | null
}

interface ListUsersByIdData {
    get_users?: {
        items: UserSummary[]
    } | null
}

interface ListUsersByIdVariables {
    tenant_id: string
    userIds: string[]
    limit: number
    offset: number
    showVotesInfo: boolean
}

interface TallySheetImportRecord {
    id: string
    source_document_id: string
    source_file_name?: string | null
    source_format: ETallySheetImportSourceFormat
    selected_channel: TallySheetVotingChannel
    status: ETallySheetImportStatus
    source_sha256?: string | null
    created_at?: string | null
    created_by_user_id: string
    summary?: TallySheetImportSummary | null
    validation_report?: TallySheetImportValidationError[] | null
    canonical_csv_sha256?: string | null
}

interface TallySheetImportItemRecord {
    id: string
    import_id: string
    election_id: string
    area_id: string
    contest_id: string
    channel: TallySheetVotingChannel
    generated_tally_sheet_id?: string | null
    baseline_approved_tally_sheet_id?: string | null
    baseline_approved_version?: number | null
    change_type: ETallySheetImportChangeType
    status: ETallySheetImportItemStatus
    previous_csv?: string | null
    incoming_csv: string
    source_refs?: {
        area_name?: string
        contest_external_id?: string
        candidate_external_ids?: string[]
    } | null
}

const emptySummary: TallySheetImportSummary = {
    imported_ballot_box_count: 0,
    changed_ballot_box_count: 0,
    new_ballot_box_count: 0,
    unchanged_ballot_box_count: 0,
    conflicted_ballot_box_count: 0,
    validation_error_count: 0,
}

const OMIT_FIELDS = ["id", "source_document_id", "source_sha256", "canonical_csv_sha256"]

const DETAIL_ITEMS_PER_PAGE = 100

const Filters: Array<ReactElement> = [
    <TextInput label="File" source="source_file_name" key="file" />,
    <TextInput label="Status" source="status" key="status" />,
    <TextInput label="Created by" source="created_by_user_id" key="created_by" />,
]

interface TallySheetImportsProps {
    openImportId?: string | null
    onOpenImportHandled?: () => void
}

export const TallySheetImports: React.FC<TallySheetImportsProps> = ({
    openImportId,
    onOpenImportHandled,
}) => {
    const electionEvent = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const refresh = useRefresh()
    const {t} = useTranslation()
    const [uploadOpen, setUploadOpen] = useState(false)
    const [detailImport, setDetailImport] = useState<TallySheetImportRecord | null>(null)
    const [file, setFile] = useState<File | null>(null)
    const [uploadedDocumentId, setUploadedDocumentId] = useState<string | null>(null)
    const [uploadedSourceSha256, setUploadedSourceSha256] = useState<string | null>(null)
    const [sourceFormat, setSourceFormat] = useState<ETallySheetImportSourceFormat>(
        ETallySheetImportSourceFormat.ESS_ENHANCED_XML
    )
    const [selectedChannel, setSelectedChannel] = useState<TallySheetVotingChannel>(
        TallySheetVotingChannel.Paper
    )
    const [preview, setPreview] = useState<TallySheetImportPreview | null>(null)
    const [isWorking, setIsWorking] = useState(false)
    const [pendingDetailImportId, setPendingDetailImportId] = useState<string | null>(null)
    const [duplicateSourceSha256, setDuplicateSourceSha256] = useState<string | null>(null)
    const [duplicateSourceImport, setDuplicateSourceImport] =
        useState<TallySheetImportRecord | null>(null)
    const [detailItemsPage, setDetailItemsPage] = useState(1)
    const detailImportRecords = useMemo(() => (detailImport ? [detailImport] : []), [detailImport])

    const canView = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_IMPORT_VIEW)
    const canCreate = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.TALLY_SHEET_IMPORT_CREATE
    )
    const canReview = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.TALLY_SHEET_IMPORT_REVIEW
    )
    const detailCreatorUsernames = useCreatorUsernames(detailImportRecords, tenantId)

    const {data: duplicateSourceImports = []} = useGetList<TallySheetImportRecord>(
        "sequent_backend_tally_sheet_import",
        {
            pagination: {page: 1, perPage: 1},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEvent?.id,
                source_sha256: duplicateSourceSha256,
            },
        },
        {
            enabled: canView && !!tenantId && !!electionEvent?.id && !!duplicateSourceSha256,
        }
    )

    const {
        data: detailItems = [],
        total: detailItemsTotal,
        isLoading: isLoadingItems,
        refetch: refetchItems,
    } = useGetList<TallySheetImportItemRecord>(
        "sequent_backend_tally_sheet_import_item",
        {
            pagination: {page: detailItemsPage, perPage: DETAIL_ITEMS_PER_PAGE},
            sort: {field: "created_at", order: "ASC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEvent?.id,
                import_id: detailImport?.id,
            },
        },
        {enabled: !!detailImport?.id}
    )

    const {data: pendingDetailImport} = useGetOne<TallySheetImportRecord>(
        "sequent_backend_tally_sheet_import",
        {id: pendingDetailImportId || ""},
        {enabled: canView && !!pendingDetailImportId}
    )

    const [getUploadUrl] = useMutation<GetUploadUrlData, GetUploadUrlVariables>(GET_UPLOAD_URL)
    const [previewImport] = useMutation<PreviewTallySheetImportData, TallySheetImportVariables>(
        PREVIEW_TALLY_SHEET_IMPORT
    )
    const [createImport] = useMutation<CreateTallySheetImportData, TallySheetImportVariables>(
        CREATE_TALLY_SHEET_IMPORT
    )
    const [reviewImport] = useMutation<ReviewTallySheetImportData, ReviewTallySheetImportVariables>(
        REVIEW_TALLY_SHEET_IMPORT
    )
    const [fetchDocument] = useLazyQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        fetchPolicy: "no-cache",
    })

    useEffect(() => {
        if (openImportId) {
            setPendingDetailImportId(openImportId)
        }
    }, [openImportId])

    useEffect(() => {
        if (!pendingDetailImportId) {
            return
        }

        const matchedImport =
            pendingDetailImport?.id === pendingDetailImportId ? pendingDetailImport : undefined
        if (matchedImport) {
            setDetailImport(matchedImport)
            setPendingDetailImportId(null)
            onOpenImportHandled?.()
        }
    }, [onOpenImportHandled, pendingDetailImport, pendingDetailImportId])

    useEffect(() => {
        setDuplicateSourceImport(duplicateSourceImports[0] ?? null)
    }, [duplicateSourceImports])

    useEffect(() => {
        setDetailItemsPage(1)
    }, [detailImport?.id])

    const resetUpload = useCallback(() => {
        setFile(null)
        setUploadedDocumentId(null)
        setUploadedSourceSha256(null)
        setDuplicateSourceSha256(null)
        setDuplicateSourceImport(null)
        setPreview(null)
        setIsWorking(false)
    }, [])

    const closeUpload = useCallback(() => {
        setUploadOpen(false)
        resetUpload()
    }, [resetUpload])

    const handleFiles = useCallback((files: FileList) => {
        const nextFile = files?.[0] ?? null
        setFile(nextFile)
        setUploadedDocumentId(null)
        setUploadedSourceSha256(null)
        setDuplicateSourceSha256(null)
        setDuplicateSourceImport(null)
        setPreview(null)
    }, [])

    const uploadSelectedFile = useCallback(async (): Promise<{
        documentId: string
        sha256?: string
    }> => {
        if (!file || !electionEvent?.id) {
            throw new Error(t("tallySheetImport.notifications.selectFile"))
        }
        const sha256 = await hashFileSha256(file)
        const mediaType =
            file.type ||
            (sourceFormat === ETallySheetImportSourceFormat.ESS_ENHANCED_XML
                ? "text/xml"
                : "text/csv")
        const {data} = await getUploadUrl({
            variables: {
                name: file.name,
                media_type: mediaType,
                size: file.size,
                is_public: false,
                election_event_id: electionEvent.id,
            },
        })
        const upload = data?.get_upload_url
        if (!upload?.url || !upload.document_id) {
            throw new Error(t("tallySheetImport.notifications.uploadUrlError"))
        }
        const uploadResponse = await fetch(upload.url, {
            method: "PUT",
            headers: {"Content-Type": mediaType},
            body: file,
        })
        if (!uploadResponse.ok) {
            throw new Error(t("tallySheetImport.notifications.uploadError"))
        }
        setUploadedDocumentId(upload.document_id)
        setUploadedSourceSha256(sha256 ?? null)
        setDuplicateSourceSha256(sha256 ?? null)
        return {documentId: upload.document_id, sha256}
    }, [electionEvent?.id, file, getUploadUrl, sourceFormat, t])

    const handlePreview = useCallback(async () => {
        if (!electionEvent?.id) {
            return
        }
        setIsWorking(true)
        try {
            const {documentId, sha256} = await uploadSelectedFile()
            const {data} = await previewImport({
                variables: {
                    electionEventId: electionEvent.id,
                    documentId,
                    sha256,
                    sourceFormat,
                    selectedChannel,
                },
            })
            const nextPreview = data?.preview_tally_sheet_import?.preview
            if (!nextPreview) {
                throw new Error(t("tallySheetImport.notifications.previewEmpty"))
            }
            setPreview(nextPreview)
        } catch (error) {
            notify(
                error instanceof Error
                    ? error.message
                    : t("tallySheetImport.notifications.previewError"),
                {type: "error"}
            )
        } finally {
            setIsWorking(false)
        }
    }, [
        electionEvent?.id,
        notify,
        previewImport,
        selectedChannel,
        sourceFormat,
        t,
        uploadSelectedFile,
    ])

    const handleCreate = useCallback(async () => {
        if (!electionEvent?.id || !uploadedDocumentId) {
            return
        }
        setIsWorking(true)
        try {
            const {data} = await createImport({
                variables: {
                    electionEventId: electionEvent.id,
                    documentId: uploadedDocumentId,
                    sha256: uploadedSourceSha256,
                    sourceFormat,
                    selectedChannel,
                },
            })
            if (!data?.create_tally_sheet_import?.import) {
                throw new Error(t("tallySheetImport.notifications.importEmpty"))
            }
            notify(t("tallySheetImport.notifications.created"), {type: "success"})
            closeUpload()
            refresh()
        } catch (error) {
            notify(
                error instanceof Error
                    ? error.message
                    : t("tallySheetImport.notifications.createError"),
                {type: "error"}
            )
        } finally {
            setIsWorking(false)
        }
    }, [
        closeUpload,
        createImport,
        electionEvent?.id,
        notify,
        refresh,
        selectedChannel,
        sourceFormat,
        t,
        uploadedSourceSha256,
        uploadedDocumentId,
    ])

    const handleReview = useCallback(
        async (decision: ETallySheetImportReviewDecision) => {
            if (!electionEvent?.id || !detailImport?.id) {
                return
            }
            setIsWorking(true)
            try {
                const {data} = await reviewImport({
                    variables: {
                        electionEventId: electionEvent.id,
                        importId: detailImport.id,
                        decision,
                    },
                })
                const nextImport = data?.review_tally_sheet_import?.import
                if (!nextImport) {
                    throw new Error(t("tallySheetImport.notifications.reviewEmpty"))
                }
                setDetailImport(nextImport)
                if (nextImport.status === ETallySheetImportStatus.CONFLICTED) {
                    notify(t("tallySheetImport.notifications.conflicted"), {type: "warning"})
                } else {
                    notify(
                        decision === ETallySheetImportReviewDecision.APPROVE
                            ? t("tallySheetImport.notifications.approved")
                            : t("tallySheetImport.notifications.disapproved"),
                        {type: "success"}
                    )
                }
                await refetchItems()
                refresh()
            } catch (error) {
                notify(
                    error instanceof Error
                        ? error.message
                        : t("tallySheetImport.notifications.reviewError"),
                    {type: "error"}
                )
            } finally {
                setIsWorking(false)
            }
        },
        [detailImport?.id, electionEvent?.id, notify, refetchItems, refresh, reviewImport, t]
    )

    const handleDownloadSource = useCallback(
        async (item: TallySheetImportRecord) => {
            if (!electionEvent?.id) {
                return
            }
            try {
                const {data} = await fetchDocument({
                    variables: {
                        electionEventId: electionEvent.id,
                        documentId: item.source_document_id,
                    },
                })
                const url = data?.fetchDocument?.url
                if (!url) {
                    throw new Error(t("tallySheetImport.notifications.sourceUrlError"))
                }
                await downloadUrl(url, item.source_file_name || `tally-sheet-import-${item.id}`)
            } catch (error) {
                notify(
                    error instanceof Error
                        ? error.message
                        : t("tallySheetImport.notifications.sourceDownloadError"),
                    {type: "error"}
                )
            }
        },
        [electionEvent?.id, fetchDocument, notify, t]
    )

    const hasPreviewErrors = (preview?.validation_errors?.length ?? 0) > 0

    const Empty = () => (
        <ResourceListStyles.EmptyBox sx={{minHeight: 360, mx: 0}}>
            <Typography variant="h4" paragraph>
                {t("tallySheetImport.empty")}
            </Typography>
            {canCreate ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t("tallySheetImport.emptyBody")}
                    </Typography>
                    <Button onClick={() => setUploadOpen(true)} startIcon={<UploadFileIcon />}>
                        {t("tallySheetImport.actions.create")}
                    </Button>
                </>
            ) : null}
        </ResourceListStyles.EmptyBox>
    )

    if (!canView) {
        return null
    }

    return (
        <Box sx={{mb: 3}}>
            <ElectionHeader
                title={String(t("tallySheetImport.title"))}
                subtitle={String(t("tallySheetImport.subtitle"))}
            />

            <List
                resource="sequent_backend_tally_sheet_import"
                actions={
                    <ListActions
                        withImport={false}
                        withExport={false}
                        extraActions={
                            canCreate
                                ? [
                                      <RaButton
                                          key="import-tally-sheets"
                                          onClick={() => setUploadOpen(true)}
                                          label={String(t("tallySheetImport.actions.create"))}
                                      >
                                          <UploadFileIcon />
                                      </RaButton>,
                                  ]
                                : []
                        }
                    />
                }
                filter={{
                    tenant_id: tenantId || undefined,
                    election_event_id: electionEvent?.id || undefined,
                }}
                filters={Filters}
                empty={<Empty />}
                sx={{
                    "flexGrow": 2,
                    "minWidth": 0,
                    "& .RaList-content": {
                        maxWidth: "100%",
                        overflowX: "auto",
                    },
                    "& .MuiTableContainer-root": {
                        maxWidth: "100%",
                        overflowX: "auto",
                    },
                }}
            >
                <Box sx={{maxWidth: "100%", minWidth: 0, overflowX: "auto"}}>
                    <TallySheetImportsDatagrid
                        tenantId={tenantId}
                        onReview={setDetailImport}
                        onDownloadSource={handleDownloadSource}
                    />
                </Box>
            </List>

            <Drawer
                anchor="right"
                open={uploadOpen}
                onClose={closeUpload}
                PaperProps={{sx: {width: {xs: "100vw", sm: 720}, maxWidth: "100vw"}}}
            >
                <Stack gap={2} sx={{p: 3}}>
                    <ElectionHeader title={String(t("tallySheetImport.createTitle"))} subtitle="" />
                    <Stack direction="row" gap={2}>
                        <FormControl size="small" fullWidth>
                            <InputLabel>{t("tallySheetImport.fields.format")}</InputLabel>
                            <Select
                                label={String(t("tallySheetImport.fields.format"))}
                                value={sourceFormat}
                                onChange={(event: SelectChangeEvent) => {
                                    setSourceFormat(
                                        event.target.value as ETallySheetImportSourceFormat
                                    )
                                    setUploadedDocumentId(null)
                                    setUploadedSourceSha256(null)
                                    setPreview(null)
                                }}
                            >
                                <MenuItem value={ETallySheetImportSourceFormat.ESS_ENHANCED_XML}>
                                    {t("tallySheetImport.sourceFormat.ESS_ENHANCED_XML")}
                                </MenuItem>
                                <MenuItem value={ETallySheetImportSourceFormat.CANONICAL_CSV}>
                                    {t("tallySheetImport.sourceFormat.CANONICAL_CSV")}
                                </MenuItem>
                            </Select>
                        </FormControl>
                        <FormControl size="small" fullWidth>
                            <InputLabel>{t("tallySheetImport.fields.channel")}</InputLabel>
                            <Select
                                label={String(t("tallySheetImport.fields.channel"))}
                                value={selectedChannel}
                                onChange={(event: SelectChangeEvent) => {
                                    const channel = event.target.value
                                    if (!isTallySheetVotingChannel(channel)) return

                                    setSelectedChannel(channel)
                                    setUploadedDocumentId(null)
                                    setUploadedSourceSha256(null)
                                    setPreview(null)
                                }}
                            >
                                {TALLY_SHEET_VOTING_CHANNELS.map((channel) => (
                                    <MenuItem key={channel} value={channel}>
                                        {t(`tallySheetImport.channel.${channel}`)}
                                    </MenuItem>
                                ))}
                            </Select>
                        </FormControl>
                    </Stack>

                    <DropFile
                        handleFiles={handleFiles}
                        accept=".xml,.csv,text/xml,application/xml,text/csv"
                        formatLabel={String(t("tallySheetImport.fields.supportedFormats"))}
                    />

                    {duplicateSourceImport ? (
                        <Alert
                            severity="warning"
                            action={
                                <Button
                                    color="inherit"
                                    size="small"
                                    onClick={() => {
                                        setUploadOpen(false)
                                        setDetailImport(duplicateSourceImport)
                                    }}
                                >
                                    {t("tallySheetImport.actions.openExisting")}
                                </Button>
                            }
                        >
                            {t("tallySheetImport.notifications.duplicateSource")}
                        </Alert>
                    ) : null}

                    <Stack direction="row" justifyContent="flex-end" gap={1}>
                        <Button onClick={closeUpload} disabled={isWorking}>
                            {t("tallySheetImport.actions.cancel")}
                        </Button>
                        <Button
                            onClick={handlePreview}
                            disabled={!file || isWorking}
                            variant="contained"
                        >
                            {t("tallySheetImport.actions.preview")}
                        </Button>
                        <Button
                            onClick={handleCreate}
                            disabled={!preview || hasPreviewErrors || isWorking}
                            color="success"
                            variant="contained"
                        >
                            {t("tallySheetImport.actions.save")}
                        </Button>
                    </Stack>

                    {isWorking ? <CircularProgress size={24} /> : null}
                    {preview ? <PreviewPanel preview={preview} /> : null}
                </Stack>
            </Drawer>

            <Drawer
                anchor="right"
                open={!!detailImport}
                onClose={() => setDetailImport(null)}
                PaperProps={{sx: {width: {xs: "100vw", md: 900}, maxWidth: "100vw"}}}
            >
                {detailImport ? (
                    <Stack gap={2} sx={{p: 3}}>
                        <Stack direction="row" justifyContent="space-between" alignItems="center">
                            <ElectionHeader
                                title={String(t("tallySheetImport.detailTitle"))}
                                subtitle={detailImport.id}
                            />
                            <Stack direction="row" gap={1}>
                                {canReview &&
                                (detailImport.status === ETallySheetImportStatus.PENDING_REVIEW ||
                                    detailImport.status === ETallySheetImportStatus.CONFLICTED) ? (
                                    <>
                                        <Button
                                            color="success"
                                            variant="contained"
                                            startIcon={<DoneIcon />}
                                            disabled={isWorking}
                                            onClick={() =>
                                                handleReview(
                                                    ETallySheetImportReviewDecision.APPROVE
                                                )
                                            }
                                        >
                                            {t("tallySheetImport.actions.approve")}
                                        </Button>
                                        <Button
                                            color="error"
                                            variant="outlined"
                                            startIcon={<CloseIcon />}
                                            disabled={isWorking}
                                            onClick={() =>
                                                handleReview(
                                                    ETallySheetImportReviewDecision.DISAPPROVE
                                                )
                                            }
                                        >
                                            {t("tallySheetImport.actions.disapprove")}
                                        </Button>
                                    </>
                                ) : null}
                                <Button onClick={() => setDetailImport(null)}>
                                    {t("tallySheetImport.actions.close")}
                                </Button>
                            </Stack>
                        </Stack>
                        <ImportMetadata
                            item={detailImport}
                            creatorUsernames={detailCreatorUsernames}
                        />
                        <ImportSummary summary={detailImport.summary ?? emptySummary} />
                        <Status status={detailImport.status} />
                        {detailImport.validation_report?.length ? (
                            <ValidationErrors errors={detailImport.validation_report} />
                        ) : null}
                        <Divider />
                        {isLoadingItems ? (
                            <CircularProgress size={24} />
                        ) : (
                            <Stack gap={1}>
                                {detailItems.map((item) => (
                                    <Accordion
                                        key={item.id}
                                        disableGutters
                                        slotProps={{
                                            transition: {
                                                unmountOnExit: true,
                                            },
                                        }}
                                    >
                                        <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                                            <Stack
                                                direction="row"
                                                spacing={1}
                                                alignItems="center"
                                                sx={{width: "100%"}}
                                            >
                                                <Typography sx={{flexGrow: 1}}>
                                                    {item.source_refs?.area_name || item.area_id} /{" "}
                                                    {item.source_refs?.contest_external_id ||
                                                        item.contest_id}
                                                </Typography>
                                                <Status status={item.change_type} />
                                                <Status status={item.status} />
                                            </Stack>
                                        </AccordionSummary>
                                        <AccordionDetails>
                                            <Stack gap={1}>
                                                <Typography variant="body2">
                                                    {t(
                                                        "tallySheetImport.fields.generatedTallySheet"
                                                    )}
                                                    :{" "}
                                                    {item.generated_tally_sheet_id ||
                                                        t("tallySheetImport.fields.none")}
                                                </Typography>
                                                {item.source_refs?.candidate_external_ids
                                                    ?.length ? (
                                                    <Typography variant="body2">
                                                        {t(
                                                            "tallySheetImport.fields.sourceCandidates"
                                                        )}
                                                        :{" "}
                                                        {item.source_refs.candidate_external_ids.join(
                                                            ", "
                                                        )}
                                                    </Typography>
                                                ) : null}
                                                <CsvDiffView
                                                    previous={item.previous_csv ?? ""}
                                                    incoming={item.incoming_csv}
                                                />
                                            </Stack>
                                        </AccordionDetails>
                                    </Accordion>
                                ))}
                                <DetailItemsPagination
                                    page={detailItemsPage}
                                    perPage={DETAIL_ITEMS_PER_PAGE}
                                    total={detailItemsTotal}
                                    itemCount={detailItems.length}
                                    onChangePage={setDetailItemsPage}
                                />
                            </Stack>
                        )}
                    </Stack>
                ) : null}
            </Drawer>
        </Box>
    )
}

const DetailItemsPagination: React.FC<{
    page: number
    perPage: number
    total?: number
    itemCount: number
    onChangePage: (page: number) => void
}> = ({page, perPage, total, itemCount, onChangePage}) => {
    const {t} = useTranslation()

    if (total === undefined || total <= perPage) {
        return null
    }

    const rangeStart = (page - 1) * perPage + 1
    const rangeEnd = rangeStart + itemCount - 1

    return (
        <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{pt: 1}}>
            <Typography variant="body2" color="text.secondary">
                {t("tallySheetImport.pagination.range", {rangeStart, rangeEnd, total})}
            </Typography>
            <Stack direction="row" gap={1}>
                <Button disabled={page <= 1} onClick={() => onChangePage(page - 1)}>
                    {t("tallySheetImport.pagination.previous")}
                </Button>
                <Button disabled={rangeEnd >= total} onClick={() => onChangePage(page + 1)}>
                    {t("tallySheetImport.pagination.next")}
                </Button>
            </Stack>
        </Stack>
    )
}

const useCreatorUsernames = (
    records: Array<Pick<TallySheetImportRecord, "created_by_user_id">>,
    tenantId?: string | null
) => {
    const userIds = useMemo(
        () =>
            Array.from(
                new Set(
                    records
                        .map((record) => record.created_by_user_id)
                        .filter((userId): userId is string => !!userId)
                )
            ),
        [records]
    )

    const {data} = useQuery<ListUsersByIdData, ListUsersByIdVariables>(LIST_USERS, {
        variables: {
            tenant_id: tenantId || "",
            userIds,
            limit: userIds.length,
            offset: 0,
            showVotesInfo: false,
        },
        skip: !tenantId || userIds.length === 0,
        fetchPolicy: "cache-first",
    })

    return useMemo(() => {
        const usernames = new Map<string, string>()
        data?.get_users?.items.forEach((user) => {
            if (user.id && user.username) {
                usernames.set(user.id, user.username)
            }
        })
        return usernames
    }, [data])
}

const formatCreatedBy = (record: TallySheetImportRecord, creatorUsernames: Map<string, string>) =>
    creatorUsernames.get(record.created_by_user_id) || record.created_by_user_id || "-"

const TallySheetImportsDatagrid: React.FC<{
    tenantId?: string | null
    onReview: (record: TallySheetImportRecord) => void
    onDownloadSource: (record: TallySheetImportRecord) => void
}> = ({tenantId, onReview, onDownloadSource}) => {
    const {t} = useTranslation()
    const {data = []} = useListContext<TallySheetImportRecord>()
    const creatorUsernames = useCreatorUsernames(data, tenantId)

    return (
        <DatagridConfigurable
            header={ThreeStateDatagridHeader}
            omit={OMIT_FIELDS}
            bulkActionButtons={false}
            sx={{
                "minWidth": 1180,
                "width": "max-content",
                "& .MuiTableCell-root": {
                    maxWidth: 220,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    verticalAlign: "top",
                    whiteSpace: "nowrap",
                },
                "& .column-source_file_name": {
                    maxWidth: 280,
                },
                "& .column-actions": {
                    maxWidth: 96,
                    width: 96,
                },
            }}
        >
            <TextField source="id" />
            <FunctionField
                source="created_at"
                label={String(t("tallySheetImport.table.created"))}
                render={(record: TallySheetImportRecord) => formatDate(record.created_at)}
            />
            <FunctionField
                source="created_by_user_id"
                label={String(t("tallySheetImport.table.createdBy"))}
                render={(record: TallySheetImportRecord) =>
                    formatCreatedBy(record, creatorUsernames)
                }
            />
            <FunctionField
                source="source_file_name"
                label={String(t("tallySheetImport.table.file"))}
                render={(record: TallySheetImportRecord) =>
                    record.source_file_name || record.source_document_id
                }
            />
            <FunctionField
                source="source_format"
                label={String(t("tallySheetImport.table.format"))}
                render={(record: TallySheetImportRecord) =>
                    t(`tallySheetImport.sourceFormat.${record.source_format}`)
                }
            />
            <FunctionField
                source="selected_channel"
                label={String(t("tallySheetImport.table.channel"))}
                render={(record: TallySheetImportRecord) =>
                    t(`tallySheetImport.channel.${record.selected_channel}`)
                }
            />
            <FunctionField
                source="status"
                label={String(t("tallySheetImport.table.status"))}
                render={(record: TallySheetImportRecord) => <Status status={record.status} />}
            />
            <FunctionField
                source="summary.imported_ballot_box_count"
                sortable={false}
                label={String(t("tallySheetImport.summary.imported"))}
                render={(record: TallySheetImportRecord) =>
                    (record.summary ?? emptySummary).imported_ballot_box_count
                }
            />
            <FunctionField
                source="summary.changed_ballot_box_count"
                sortable={false}
                label={String(t("tallySheetImport.summary.changed"))}
                render={(record: TallySheetImportRecord) =>
                    (record.summary ?? emptySummary).changed_ballot_box_count
                }
            />
            <FunctionField
                source="summary.new_ballot_box_count"
                sortable={false}
                label={String(t("tallySheetImport.summary.new"))}
                render={(record: TallySheetImportRecord) =>
                    (record.summary ?? emptySummary).new_ballot_box_count
                }
            />
            <FunctionField
                source="summary.unchanged_ballot_box_count"
                sortable={false}
                label={String(t("tallySheetImport.summary.unchanged"))}
                render={(record: TallySheetImportRecord) =>
                    (record.summary ?? emptySummary).unchanged_ballot_box_count
                }
            />
            <WrapperField source="actions" label={String(t("tallySheetImport.table.actions"))}>
                <FunctionField
                    render={(record: TallySheetImportRecord) => (
                        <ImportActions
                            record={record}
                            onReview={onReview}
                            onDownloadSource={onDownloadSource}
                        />
                    )}
                />
            </WrapperField>
        </DatagridConfigurable>
    )
}

const ImportMetadata: React.FC<{
    item: TallySheetImportRecord
    creatorUsernames: Map<string, string>
}> = ({item, creatorUsernames}) => {
    const {t} = useTranslation()

    return (
        <Stack gap={0.5}>
            <MetadataLine
                label={String(t("tallySheetImport.table.createdBy"))}
                value={formatCreatedBy(item, creatorUsernames)}
            />
            <MetadataLine
                label={String(t("tallySheetImport.table.created"))}
                value={formatDate(item.created_at)}
            />
        </Stack>
    )
}

const MetadataLine: React.FC<{label: string; value?: string | null}> = ({label, value}) => (
    <Typography variant="body2">
        <Typography component="span" variant="body2" color="text.secondary">
            {label}:{" "}
        </Typography>
        {value || "-"}
    </Typography>
)

const ImportActions: React.FC<{
    record: TallySheetImportRecord
    onReview: (record: TallySheetImportRecord) => void
    onDownloadSource: (record: TallySheetImportRecord) => void
}> = ({record, onReview, onDownloadSource}) => {
    const {t} = useTranslation()

    return (
        <Stack direction="row" justifyContent="flex-end" gap={0.5}>
            <Tooltip title={String(t("tallySheetImport.actions.review"))}>
                <IconButton size="small" onClick={() => onReview(record)}>
                    <VisibilityIcon fontSize="small" />
                </IconButton>
            </Tooltip>
            <Tooltip title={String(t("tallySheetImport.actions.source"))}>
                <IconButton size="small" onClick={() => onDownloadSource(record)}>
                    <DownloadIcon fontSize="small" />
                </IconButton>
            </Tooltip>
        </Stack>
    )
}

const PreviewPanel: React.FC<{preview: TallySheetImportPreview}> = ({preview}) => (
    <Stack gap={2}>
        <ImportSummary summary={preview.summary} />
        {preview.validation_errors.length ? (
            <ValidationErrors errors={preview.validation_errors} />
        ) : null}
        {preview.items.map((item) => (
            <Accordion
                key={`${item.area_id}:${item.contest_id}:${item.channel}`}
                disableGutters
                slotProps={{
                    transition: {
                        unmountOnExit: true,
                    },
                }}
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                    <Stack direction="row" spacing={1} alignItems="center" sx={{width: "100%"}}>
                        <Typography sx={{flexGrow: 1}}>
                            {item.area_name} / {item.contest_name || item.contest_id}
                        </Typography>
                        <Status status={item.change_type} />
                    </Stack>
                </AccordionSummary>
                <AccordionDetails>
                    <CsvDiffView previous={item.previous_csv ?? ""} incoming={item.incoming_csv} />
                </AccordionDetails>
            </Accordion>
        ))}
    </Stack>
)

const ImportSummary: React.FC<{summary: TallySheetImportSummary}> = ({summary}) => {
    const {t} = useTranslation()

    return (
        <Stack direction="row" gap={1} flexWrap="wrap">
            <SummaryChip
                label={t("tallySheetImport.summary.imported")}
                value={summary.imported_ballot_box_count}
            />
            <SummaryChip
                label={t("tallySheetImport.summary.changed")}
                value={summary.changed_ballot_box_count}
            />
            <SummaryChip
                label={t("tallySheetImport.summary.new")}
                value={summary.new_ballot_box_count}
            />
            <SummaryChip
                label={t("tallySheetImport.summary.unchanged")}
                value={summary.unchanged_ballot_box_count}
            />
            <SummaryChip
                label={t("tallySheetImport.summary.conflicted")}
                value={summary.conflicted_ballot_box_count}
            />
            <SummaryChip
                label={t("tallySheetImport.summary.errors")}
                value={summary.validation_error_count}
            />
        </Stack>
    )
}

const SummaryChip: React.FC<{label: string; value: number}> = ({label, value}) => (
    <Box sx={{border: "1px solid", borderColor: "divider", borderRadius: 1, px: 1.5, py: 1}}>
        <Typography variant="caption" color="text.secondary">
            {label}
        </Typography>
        <Typography variant="h6">{value}</Typography>
    </Box>
)

const ValidationErrors: React.FC<{errors: TallySheetImportValidationError[]}> = ({errors}) => (
    <Alert severity="error">
        <Stack gap={0.5}>
            {errors.map((error, index) => (
                <Typography key={`${error.code}:${index}`} variant="body2">
                    {error.message}
                </Typography>
            ))}
        </Stack>
    </Alert>
)

const CsvDiffView: React.FC<{previous: string; incoming: string}> = ({previous, incoming}) => {
    const diff = useMemo(() => diffLines(previous, incoming), [incoming, previous])

    return (
        <Box
            component="pre"
            sx={{
                bgcolor: "grey.100",
                borderRadius: 1,
                maxHeight: 420,
                overflow: "auto",
                p: 1.5,
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
                fontSize: 12,
            }}
        >
            {diff.map((part, index) => (
                <Box
                    key={index}
                    component="span"
                    sx={{
                        display: "block",
                        bgcolor: part.added ? "#43e3a1" : part.removed ? "#fa958e" : "transparent",
                        textDecoration: part.removed ? "line-through" : "none",
                    }}
                >
                    {part.value}
                </Box>
            ))}
        </Box>
    )
}

const Status: React.FC<{status: string}> = ({status}) => {
    const {t} = useTranslation()
    const color =
        status === ETallySheetImportStatus.APPROVED
            ? "success"
            : status === ETallySheetImportStatus.DISAPPROVED ||
                status === ETallySheetImportStatus.FAILED_VALIDATION
              ? "error"
              : status === ETallySheetImportStatus.CONFLICTED
                ? "warning"
                : "default"
    return (
        <Chip size="small" label={t(`tallySheetImport.status.${status}`, status)} color={color} />
    )
}

const formatDate = (value?: string | null) => {
    if (!value) {
        return "-"
    }
    return new Date(value).toLocaleString()
}

const hashFileSha256 = async (file: File): Promise<string | undefined> => {
    const subtle = globalThis.crypto?.subtle
    if (!subtle) {
        return undefined
    }

    const digest = await subtle.digest("SHA-256", await file.arrayBuffer())
    return Array.from(new Uint8Array(digest))
        .map((byte) => byte.toString(16).padStart(2, "0"))
        .join("")
}
