// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useRef, useState} from "react"
import {
    DatagridConfigurable,
    FunctionField,
    Identifier,
    List,
    TextField,
    WrapperField,
    useGetOne,
    useListContext,
    useNotify,
    useRecordContext,
    useRefresh,
} from "react-admin"
import {useMutation} from "@apollo/client"
import {
    Alert,
    Box,
    Button,
    Chip,
    CircularProgress,
    Drawer,
    Tooltip,
    Typography,
} from "@mui/material"
import DeleteIcon from "@mui/icons-material/Delete"
import DownloadIcon from "@mui/icons-material/Download"
import UploadFileIcon from "@mui/icons-material/UploadFile"
import VisibilityIcon from "@mui/icons-material/Visibility"
import {Dialog, DropFile} from "@sequentech/ui-essentials"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {IMPORT_CERTIFICATE_AUTHORITY} from "@/queries/ImportCertificateAuthority"
import {DELETE_CERTIFICATE_AUTHORITY} from "@/queries/DeleteCertificateAuthority"
import {EXPORT_CERTIFICATE_AUTHORITY} from "@/queries/ExportCertificateAuthority"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {DrawerStyles} from "@/components/styles/DrawerStyles"
import ElectionHeader from "@/components/ElectionHeader"
import {ListActions} from "@/components/ListActions"
import {Button as ReactAdminButton} from "react-admin"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"

const RESOURCE = "sequent_backend_certificate_authority"
const FINGERPRINT_TRUNCATE_LENGTH = 24
const AUTO_HIDE_DURATION = 10000

const getExpiryStatus = (notAfter: string): "expired" | "expiringSoon" | "valid" => {
    const expiry = new Date(notAfter)
    const now = new Date()
    if (expiry < now) return "expired"
    if (expiry.getTime() - now.getTime() < 30 * 24 * 3600 * 1000) return "expiringSoon"
    return "valid"
}

const expiryChipColor = (status: "expired" | "expiringSoon" | "valid") => {
    if (status === "expired") return "error"
    if (status === "expiringSoon") return "warning"
    return "success"
}

const formatDate = (iso: string) => new Date(iso).toLocaleDateString()

const LabelValue: React.FC<{label: string; value?: string | null; mono?: boolean}> = ({
    label,
    value,
    mono,
}) => (
    <Box>
        <Typography variant="caption" color="text.secondary">
            {label}
        </Typography>
        <Typography
            variant="body2"
            sx={mono ? {fontFamily: "monospace", fontSize: "0.75rem", wordBreak: "break-all"} : {}}
        >
            {value || "—"}
        </Typography>
    </Box>
)

const ViewCAContent: React.FC<{id: Identifier; onClose: () => void}> = ({id, onClose}) => {
    const {t} = useTranslation()
    const {data: ca, isLoading} = useGetOne(RESOURCE, {id})

    if (isLoading) {
        return (
            <Box sx={{display: "flex", justifyContent: "center", p: 4}}>
                <CircularProgress />
            </Box>
        )
    }
    if (!ca) return null

    const expiryStatus = getExpiryStatus(ca.not_after)

    return (
        <Box sx={{p: 2}}>
            <ElectionHeader
                title={t("certificateAuthorities.viewDialog.title")}
                subtitle={ca.common_name}
            />
            <DrawerStyles.Wrapper>
                <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
                    <LabelValue
                        label={t("certificateAuthorities.columns.commonName")}
                        value={ca.common_name}
                    />
                    <Box>
                        <Typography variant="caption" color="text.secondary">
                            {t("certificateAuthorities.columns.type")}
                        </Typography>
                        <Box sx={{mt: 0.5}}>
                            <Chip
                                label={
                                    ca.subject === ca.issuer
                                        ? t("certificateAuthorities.type.root")
                                        : t("certificateAuthorities.type.intermediate")
                                }
                                size="small"
                                variant="outlined"
                            />
                        </Box>
                    </Box>
                    <LabelValue
                        label={t("certificateAuthorities.columns.issuerCn")}
                        value={ca.issuer_common_name}
                    />
                    <LabelValue
                        label={t("certificateAuthorities.viewDialog.subject")}
                        value={ca.subject}
                        mono
                    />
                    <LabelValue
                        label={t("certificateAuthorities.viewDialog.issuer")}
                        value={ca.issuer}
                        mono
                    />
                    <LabelValue
                        label={t("certificateAuthorities.columns.notBefore")}
                        value={formatDate(ca.not_before)}
                    />
                    <Box>
                        <Typography variant="caption" color="text.secondary">
                            {t("certificateAuthorities.columns.notAfter")}
                        </Typography>
                        <Box sx={{display: "flex", alignItems: "center", gap: 1, mt: 0.5}}>
                            <Typography variant="body2">{formatDate(ca.not_after)}</Typography>
                            <Chip
                                label={t(`certificateAuthorities.expiry.${expiryStatus}`)}
                                color={expiryChipColor(expiryStatus)}
                                size="small"
                            />
                        </Box>
                    </Box>
                    <LabelValue
                        label={t("certificateAuthorities.viewDialog.serialNumber")}
                        value={ca.serial_number}
                        mono
                    />
                    <LabelValue
                        label={t("certificateAuthorities.columns.fingerprint")}
                        value={ca.fingerprint_sha256}
                        mono
                    />
                    <Box>
                        <Typography variant="caption" color="text.secondary">
                            {t("certificateAuthorities.viewDialog.pemContent")}
                        </Typography>
                        <Box
                            component="pre"
                            sx={{
                                mt: 0.5,
                                p: 1,
                                bgcolor: "grey.100",
                                borderRadius: 1,
                                fontFamily: "monospace",
                                fontSize: "0.7rem",
                                whiteSpace: "pre",
                                overflowX: "auto",
                                maxHeight: 300,
                                overflowY: "auto",
                            }}
                        >
                            {ca.pem}
                        </Box>
                    </Box>
                </Box>
                <Box sx={{mt: 3}}>
                    <Button onClick={onClose}>{t("common.label.close")}</Button>
                </Box>
            </DrawerStyles.Wrapper>
        </Box>
    )
}

export const EditElectionEventCAs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const notify = useNotify()
    const refresh = useRefresh()

    const [importDrawerOpen, setImportDrawerOpen] = useState(false)
    const [viewId, setViewId] = useState<Identifier | undefined>()
    const [openDeleteModal, setOpenDeleteModal] = useState(false)
    const [deleteIds, setDeleteIds] = useState<Identifier[]>([])
    const [pemContent, setPemContent] = useState<string>("")
    const [fileError, setFileError] = useState<string | null>(null)
    const [openExportModal, setOpenExportModal] = useState(false)
    const [exportIds, setExportIds] = useState<Identifier[]>([])
    const [openBulkDeleteModal, setOpenBulkDeleteModal] = useState(false)
    const [bulkDeleteIds, setBulkDeleteIds] = useState<Identifier[]>([])
    const unselectAllRef = useRef<(() => void) | null>(null)

    const canWrite = authContext.isAuthorized(true, tenantId, IPermissions.CA_WRITE)
    const canRead = authContext.isAuthorized(true, tenantId, IPermissions.CA_READ)

    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()

    const [deleteCA] = useMutation(DELETE_CERTIFICATE_AUTHORITY, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.CA_WRITE,
            },
        },
        onCompleted: () => {
            refresh()
        },
        onError: () => {
            notify(t("certificateAuthorities.notify.deleteError", {error: ""}), {
                type: "error",
            })
            refresh()
        },
    })

    const [exportCA] = useMutation(EXPORT_CERTIFICATE_AUTHORITY, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.CA_READ,
            },
        },
    })

    const [importCA, {loading: importing}] = useMutation(IMPORT_CERTIFICATE_AUTHORITY, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.CA_WRITE,
            },
        },
        onCompleted: (result) => {
            const {inserted_count, skipped_count, errors} =
                result.import_certificate_authority ?? {}
            const hasInserted = (inserted_count ?? 0) > 0
            const hasSkipped = (skipped_count ?? 0) > 0
            const hasErrors = (errors?.length ?? 0) > 0

            if (hasInserted) {
                notify(
                    t("certificateAuthorities.notify.importSuccess", {inserted: inserted_count}),
                    {type: "success", autoHideDuration: AUTO_HIDE_DURATION}
                )
            }

            if (hasSkipped || hasErrors) {
                const redParts: string[] = []
                if (hasSkipped) {
                    redParts.push(
                        t("certificateAuthorities.notify.importSkipped", {count: skipped_count})
                    )
                }
                if (hasErrors) {
                    redParts.push(...(errors as string[]))
                }
                notify(
                    t("certificateAuthorities.notify.importErrors", {
                        errors: redParts.join("; "),
                    }),
                    {type: "error", autoHideDuration: AUTO_HIDE_DURATION}
                )
            }

            setImportDrawerOpen(false)
            setPemContent("")
            setFileError(null)
            refresh()
        },
        onError: (err) => {
            notify(t("certificateAuthorities.notify.importError", {error: err.message}), {
                type: "error",
                autoHideDuration: AUTO_HIDE_DURATION,
            })
        },
    })

    const handleDropFiles = (files: FileList) => {
        const file = files[0]
        if (!file) return
        const reader = new FileReader()
        reader.onload = (e) => {
            setPemContent((e.target?.result as string) ?? "")
            setFileError(null)
        }
        reader.onerror = () => setFileError(t("certificateAuthorities.fileReadError"))
        reader.readAsText(file)
    }

    const handleImport = () => {
        if (!pemContent || !record?.id) return
        importCA({variables: {electionEventId: record.id, pemContent}})
    }

    const handleImportDrawerClose = () => {
        setImportDrawerOpen(false)
        setPemContent("")
        setFileError(null)
    }

    const deleteAction = (id: Identifier) => {
        setDeleteIds([id])
        setOpenDeleteModal(true)
    }

    const confirmDeleteAction = () => {
        deleteCA({variables: {ids: deleteIds, electionEventId: record?.id}})
        setDeleteIds([])
    }

    const confirmBulkDeleteAction = () => {
        deleteCA({variables: {ids: bulkDeleteIds, electionEventId: record?.id}})
        setBulkDeleteIds([])
        unselectAllRef.current?.()
    }

    const handleExportAll = () => {
        setExportIds([])
        setOpenExportModal(true)
    }

    const confirmExportAction = async () => {
        if (!record?.id) return
        const currWidget = addWidget(ETasksExecution.EXPORT_CERTIFICATE_AUTHORITIES, undefined)
        try {
            const {data, errors} = await exportCA({
                variables: {
                    ids: exportIds,
                    electionEventId: record.id,
                },
            })
            if (errors || !data) {
                updateWidgetFail(currWidget.identifier)
                return
            }
            const taskId = data.export_certificate_authority?.task_execution?.id
            taskId
                ? setWidgetTaskId(currWidget.identifier, taskId)
                : updateWidgetFail(currWidget.identifier)
        } catch {
            updateWidgetFail(currWidget.identifier)
        }
        setOpenExportModal(false)
    }

    function BulkActions() {
        const {selectedIds, onUnselectItems} = useListContext()
        unselectAllRef.current = onUnselectItems

        return (
            <>
                {canRead && (
                    <ReactAdminButton
                        onClick={() => {
                            setExportIds(selectedIds)
                            setOpenExportModal(true)
                        }}
                        label={String(t("common.label.export"))}
                    >
                        <DownloadIcon />
                    </ReactAdminButton>
                )}
                {canWrite && (
                    <ReactAdminButton
                        onClick={() => {
                            setBulkDeleteIds(selectedIds)
                            setOpenBulkDeleteModal(true)
                        }}
                        label={String(t("common.label.delete"))}
                    >
                        <DeleteIcon />
                    </ReactAdminButton>
                )}
            </>
        )
    }

    const actions: Action[] = [
        {
            icon: <VisibilityIcon className="view-ca-icon" />,
            action: (id) => setViewId(id),
        },
        {
            icon: <DeleteIcon className="delete-ca-icon" />,
            action: deleteAction,
            showAction: () => canWrite,
        },
    ]

    function CertificateListActions() {
        const {selectedIds} = useListContext()
        return (
            <div style={{visibility: selectedIds.length > 0 ? "hidden" : "visible"}}>
                <ListActions
                    withImport={canWrite}
                    doImport={() => setImportDrawerOpen(true)}
                    withExport={true}
                    doExport={handleExportAll}
                    withFilter={false}
                />
            </div>
        )
    }

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h6">{t("certificateAuthorities.emptyHeader")}</Typography>
            {canWrite && (
                <Button
                    startIcon={<UploadFileIcon />}
                    onClick={() => setImportDrawerOpen(true)}
                    sx={{mt: 2}}
                >
                    {t("certificateAuthorities.importButton")}
                </Button>
            )}
        </ResourceListStyles.EmptyBox>
    )

    return (
        <>
            <ElectionHeader
                title={String(t("certificateAuthorities.title"))}
                subtitle={String(t("certificateAuthorities.subtitle"))}
            />
            <List
                resource={RESOURCE}
                actions={<CertificateListActions />}
                empty={<Empty />}
                storeKey={false}
                disableSyncWithLocation
                filter={{
                    tenant_id: tenantId || undefined,
                    election_event_id: record?.id || undefined,
                }}
            >
                <DatagridConfigurable rowClick={false} bulkActionButtons={<BulkActions />}>
                    <TextField
                        source="common_name"
                        label={t("certificateAuthorities.columns.commonName")}
                    />
                    <FunctionField
                        label={t("certificateAuthorities.columns.type")}
                        render={(ca: any) => (
                            <Chip
                                label={
                                    ca.subject === ca.issuer
                                        ? t("certificateAuthorities.type.root")
                                        : t("certificateAuthorities.type.intermediate")
                                }
                                size="small"
                                variant="outlined"
                            />
                        )}
                    />
                    <TextField
                        source="issuer_common_name"
                        label={t("certificateAuthorities.columns.issuerCn")}
                    />
                    <FunctionField
                        source="not_before"
                        label={t("certificateAuthorities.columns.notBefore")}
                        render={(ca: any) => formatDate(ca.not_before)}
                    />
                    <FunctionField
                        source="not_after"
                        label={t("certificateAuthorities.columns.notAfter")}
                        render={(ca: any) => {
                            const status = getExpiryStatus(ca.not_after)
                            return (
                                <Box sx={{display: "flex", alignItems: "center", gap: 1}}>
                                    {formatDate(ca.not_after)}
                                    <Chip
                                        label={t(`certificateAuthorities.expiry.${status}`)}
                                        color={expiryChipColor(status)}
                                        size="small"
                                    />
                                </Box>
                            )
                        }}
                    />
                    <FunctionField
                        source="fingerprint_sha256"
                        label={t("certificateAuthorities.columns.fingerprint")}
                        render={(ca: any) =>
                            ca.fingerprint_sha256 ? (
                                <Tooltip title={ca.fingerprint_sha256} placement="top">
                                    <Typography
                                        variant="body2"
                                        sx={{fontFamily: "monospace", fontSize: "0.75rem"}}
                                    >
                                        {ca.fingerprint_sha256.slice(
                                            0,
                                            FINGERPRINT_TRUNCATE_LENGTH
                                        )}
                                        …
                                    </Typography>
                                </Tooltip>
                            ) : (
                                "—"
                            )
                        }
                    />
                    <WrapperField source="actions" label="Actions">
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </DatagridConfigurable>
            </List>

            {/* View Drawer */}
            <Drawer
                anchor="right"
                open={!!viewId}
                onClose={() => setViewId(undefined)}
                PaperProps={{sx: {width: "40%"}}}
            >
                {viewId && <ViewCAContent id={viewId} onClose={() => setViewId(undefined)} />}
            </Drawer>

            {/* Import Drawer */}
            <Drawer
                anchor="right"
                open={importDrawerOpen}
                onClose={handleImportDrawerClose}
                PaperProps={{sx: {width: "30%"}}}
            >
                <Box sx={{padding: "16px"}}>
                    <ElectionHeader
                        title={t("certificateAuthorities.importDialog.title")}
                        subtitle={t("certificateAuthorities.importDialog.subtitle")}
                    />
                    <DrawerStyles.Wrapper>
                        <DrawerStyles.SubTitle>
                            {t("certificateAuthorities.importDialog.description")}
                        </DrawerStyles.SubTitle>
                        <Box sx={{mt: 2}}>
                            <DropFile handleFiles={handleDropFiles} />
                            {pemContent && (
                                <Typography variant="caption" sx={{mt: 1, display: "block"}}>
                                    {t("certificateAuthorities.importDialog.fileLoaded", {
                                        bytes: pemContent.length,
                                    })}
                                </Typography>
                            )}
                            {fileError && (
                                <Alert severity="error" sx={{mt: 1}}>
                                    {fileError}
                                </Alert>
                            )}
                        </Box>
                        <Box sx={{mt: 3, display: "flex", gap: 1}}>
                            <Button onClick={handleImportDrawerClose}>
                                {t("common.label.cancel")}
                            </Button>
                            <Button
                                onClick={handleImport}
                                variant="contained"
                                disabled={!pemContent || importing}
                            >
                                {importing ? (
                                    <CircularProgress size={20} />
                                ) : (
                                    t("certificateAuthorities.importDialog.importButton")
                                )}
                            </Button>
                        </Box>
                    </DrawerStyles.Wrapper>
                </Box>
            </Drawer>

            {/* Delete Confirmation Dialog */}
            <Dialog
                variant="warning"
                open={openDeleteModal}
                ok={String(t("common.label.delete"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.warning"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmDeleteAction()
                    }
                    setOpenDeleteModal(false)
                }}
            >
                {t("common.message.delete")}
            </Dialog>

            {/* Bulk Delete Confirmation Dialog */}
            <Dialog
                variant="warning"
                open={openBulkDeleteModal}
                ok={String(t("common.label.delete"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.warning"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmBulkDeleteAction()
                    }
                    setOpenBulkDeleteModal(false)
                }}
            >
                {t("certificateAuthorities.deleteDialog.description", {
                    count: bulkDeleteIds.length,
                })}
            </Dialog>

            {/* Export Confirmation Dialog */}
            <Dialog
                variant="info"
                open={openExportModal}
                ok={String(t("common.label.export"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("certificateAuthorities.exportDialog.title"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmExportAction()
                    } else {
                        setOpenExportModal(false)
                    }
                }}
            >
                {t("certificateAuthorities.exportDialog.description", {
                    amount:
                        exportIds.length === 0
                            ? t("certificateAuthorities.exportDialog.all")
                            : exportIds.length,
                })}
            </Dialog>
        </>
    )
}

export default EditElectionEventCAs
