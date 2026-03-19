// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useRef, useState} from "react"
import {useRecordContext, useNotify} from "react-admin"
import {useQuery, useMutation} from "@apollo/client"
import {
    Alert,
    Box,
    Button,
    Chip,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogContentText,
    DialogTitle,
    Paper,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    Tooltip,
    Typography,
} from "@mui/material"
import DeleteIcon from "@mui/icons-material/Delete"
import UploadFileIcon from "@mui/icons-material/UploadFile"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {GET_CERTIFICATE_AUTHORITIES} from "@/queries/GetCertificateAuthorities"
import {IMPORT_CERTIFICATE_AUTHORITY} from "@/queries/ImportCertificateAuthority"
import {DELETE_CERTIFICATE_AUTHORITY} from "@/queries/DeleteCertificateAuthority"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"

interface CertificateAuthority {
    id: string
    common_name: string
    issuer_common_name: string
    subject: string
    issuer: string
    not_before: string
    not_after: string
    fingerprint_sha256: string
    serial_number: string
    created_at: string
}

const FINGERPRINT_TRUNCATE_LENGTH = 23

const getExpiryStatus = (notAfter: string): "expired" | "expiringSoon" | "valid" => {
    const expiry = new Date(notAfter)
    const now = new Date()
    if (expiry < now) {
        return "expired"
    }
    const thirtyDaysMs = 30 * 24 * 3600 * 1000
    if (expiry.getTime() - now.getTime() < thirtyDaysMs) {
        return "expiringSoon"
    }
    return "valid"
}

const expiryChipColor = (status: "expired" | "expiringSoon" | "valid") => {
    if (status === "expired") return "error"
    if (status === "expiringSoon") return "warning"
    return "success"
}

const getCertType = (subject: string, issuer: string, t: (key: string) => string) =>
    subject === issuer
        ? t("certificateAuthorities.type.root")
        : t("certificateAuthorities.type.intermediate")

const formatDate = (iso: string) => new Date(iso).toLocaleDateString()

export const EditElectionEventCAs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const notify = useNotify()

    const [importDialogOpen, setImportDialogOpen] = useState(false)
    const [deleteDialogId, setDeleteDialogId] = useState<string | null>(null)
    const [pemContent, setPemContent] = useState<string>("")
    const [fileError, setFileError] = useState<string | null>(null)
    const fileInputRef = useRef<HTMLInputElement>(null)

    const canWrite = authContext.isAuthorized(true, tenantId, IPermissions.CA_WRITE)

    const {data, loading, refetch} = useQuery(GET_CERTIFICATE_AUTHORITIES, {
        variables: {electionEventId: record?.id},
        skip: !record?.id,
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
            if (errors?.length > 0) {
                notify(t("certificateAuthorities.notify.importError", {error: errors.join("; ")}), {
                    type: "error",
                })
            } else {
                notify(
                    t("certificateAuthorities.notify.importSuccess", {
                        inserted: inserted_count,
                        skipped: skipped_count,
                    }),
                    {type: "success"}
                )
            }
            setImportDialogOpen(false)
            setPemContent("")
            setFileError(null)
            refetch()
        },
        onError: (err) => {
            notify(t("certificateAuthorities.notify.importError", {error: err.message}), {
                type: "error",
            })
        },
    })

    const [deleteCA, {loading: deleting}] = useMutation(DELETE_CERTIFICATE_AUTHORITY, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.CA_WRITE,
            },
        },
        onCompleted: () => {
            notify(t("certificateAuthorities.notify.deleteSuccess"), {type: "success"})
            setDeleteDialogId(null)
            refetch()
        },
        onError: (err) => {
            notify(t("certificateAuthorities.notify.deleteError", {error: err.message}), {
                type: "error",
            })
        },
    })

    const handleFileChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0]
        if (!file) {
            return
        }
        const reader = new FileReader()
        reader.onload = (e) => {
            setPemContent((e.target?.result as string) ?? "")
            setFileError(null)
        }
        reader.onerror = () => {
            setFileError(t("certificateAuthorities.fileReadError"))
        }
        reader.readAsText(file)
        // Reset input so the same file can be re-selected if needed
        event.target.value = ""
    }

    const handleImport = () => {
        if (!pemContent || !record?.id) {
            return
        }
        importCA({variables: {electionEventId: record.id, pemContent}})
    }

    const handleDeleteConfirm = () => {
        if (!deleteDialogId) {
            return
        }
        deleteCA({variables: {id: deleteDialogId}})
    }

    const handleImportDialogClose = () => {
        setImportDialogOpen(false)
        setPemContent("")
        setFileError(null)
    }

    const cas: CertificateAuthority[] =
        data?.sequent_backend_certificate_authority ?? []

    if (loading) {
        return (
            <Box sx={{display: "flex", justifyContent: "center", p: 4}}>
                <CircularProgress />
            </Box>
        )
    }

    return (
        <Box sx={{p: 2}}>
            {canWrite && (
                <Box sx={{mb: 2}}>
                    <Button
                        variant="contained"
                        startIcon={<UploadFileIcon />}
                        onClick={() => setImportDialogOpen(true)}
                    >
                        {t("certificateAuthorities.importButton")}
                    </Button>
                </Box>
            )}

            {cas.length === 0 ? (
                <ResourceListStyles.EmptyBox>
                    <Typography variant="h6" paragraph>
                        {t("certificateAuthorities.emptyHeader")}
                    </Typography>
                </ResourceListStyles.EmptyBox>
            ) : (
                <TableContainer component={Paper}>
                    <Table size="small">
                        <TableHead>
                            <TableRow>
                                <TableCell>
                                    {t("certificateAuthorities.columns.commonName")}
                                </TableCell>
                                <TableCell>{t("certificateAuthorities.columns.type")}</TableCell>
                                <TableCell>
                                    {t("certificateAuthorities.columns.issuerCn")}
                                </TableCell>
                                <TableCell>
                                    {t("certificateAuthorities.columns.notBefore")}
                                </TableCell>
                                <TableCell>
                                    {t("certificateAuthorities.columns.notAfter")}
                                </TableCell>
                                <TableCell>
                                    {t("certificateAuthorities.columns.fingerprint")}
                                </TableCell>
                                {canWrite && <TableCell />}
                            </TableRow>
                        </TableHead>
                        <TableBody>
                            {cas.map((ca) => {
                                const expiryStatus = getExpiryStatus(ca.not_after)
                                return (
                                    <TableRow key={ca.id}>
                                        <TableCell>{ca.common_name}</TableCell>
                                        <TableCell>
                                            <Chip
                                                label={getCertType(ca.subject, ca.issuer, t)}
                                                size="small"
                                                variant="outlined"
                                            />
                                        </TableCell>
                                        <TableCell>{ca.issuer_common_name}</TableCell>
                                        <TableCell>{formatDate(ca.not_before)}</TableCell>
                                        <TableCell>
                                            <Box
                                                sx={{
                                                    display: "flex",
                                                    alignItems: "center",
                                                    gap: 1,
                                                }}
                                            >
                                                {formatDate(ca.not_after)}
                                                <Chip
                                                    label={t(
                                                        `certificateAuthorities.expiry.${expiryStatus}`
                                                    )}
                                                    color={expiryChipColor(expiryStatus)}
                                                    size="small"
                                                />
                                            </Box>
                                        </TableCell>
                                        <TableCell>
                                            <Tooltip
                                                title={ca.fingerprint_sha256}
                                                placement="top"
                                            >
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
                                        </TableCell>
                                        {canWrite && (
                                            <TableCell>
                                                <Button
                                                    size="small"
                                                    color="error"
                                                    startIcon={<DeleteIcon />}
                                                    onClick={() => setDeleteDialogId(ca.id)}
                                                    disabled={deleting}
                                                >
                                                    {t("common.delete")}
                                                </Button>
                                            </TableCell>
                                        )}
                                    </TableRow>
                                )
                            })}
                        </TableBody>
                    </Table>
                </TableContainer>
            )}

            {/* Import Dialog */}
            <Dialog
                open={importDialogOpen}
                onClose={handleImportDialogClose}
                maxWidth="sm"
                fullWidth
            >
                <DialogTitle>{t("certificateAuthorities.importDialog.title")}</DialogTitle>
                <DialogContent>
                    <DialogContentText sx={{mb: 2}}>
                        {t("certificateAuthorities.importDialog.description")}
                    </DialogContentText>
                    <input
                        ref={fileInputRef}
                        type="file"
                        accept=".pem,.cer,.crt"
                        style={{display: "none"}}
                        onChange={handleFileChange}
                    />
                    <Button
                        variant="outlined"
                        startIcon={<UploadFileIcon />}
                        onClick={() => fileInputRef.current?.click()}
                    >
                        {t("certificateAuthorities.importDialog.selectFile")}
                    </Button>
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
                </DialogContent>
                <DialogActions>
                    <Button onClick={handleImportDialogClose}>
                        {t("common.cancel")}
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
                </DialogActions>
            </Dialog>

            {/* Delete Confirmation Dialog */}
            <Dialog open={!!deleteDialogId} onClose={() => setDeleteDialogId(null)}>
                <DialogTitle>{t("common.confirmDelete")}</DialogTitle>
                <DialogContent>
                    <DialogContentText>
                        {t("common.confirmDeleteDescription")}
                    </DialogContentText>
                </DialogContent>
                <DialogActions>
                    <Button onClick={() => setDeleteDialogId(null)}>{t("common.cancel")}</Button>
                    <Button
                        onClick={handleDeleteConfirm}
                        color="error"
                        variant="contained"
                        disabled={deleting}
                    >
                        {deleting ? <CircularProgress size={20} /> : t("common.delete")}
                    </Button>
                </DialogActions>
            </Dialog>
        </Box>
    )
}

export default EditElectionEventCAs
