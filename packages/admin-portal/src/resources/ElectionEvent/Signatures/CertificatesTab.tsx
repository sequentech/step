// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useId, useState} from "react"
import {useTranslation} from "react-i18next"
import {useEventZonedFormat} from "@/hooks/useZonedFormat"
import {useNotify} from "react-admin"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Alert,
    Box,
    Button,
    Chip,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    FormControl,
    FormControlLabel,
    FormLabel,
    IconButton,
    InputAdornment,
    InputLabel,
    MenuItem,
    Radio,
    RadioGroup,
    Select,
    Switch,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    TextField,
    Typography,
} from "@mui/material"
import BlockIcon from "@mui/icons-material/Block"
import DeleteIcon from "@mui/icons-material/Delete"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import SearchIcon from "@mui/icons-material/Search"
import WarningAmberIcon from "@mui/icons-material/WarningAmber"
import {visuallyHidden} from "@mui/utils"
import {
    CertificatePostBinding,
    CertificateRegistration,
    CrlUnavailablePolicy,
    CrlStatus,
    RevocationCheck,
    StaffCertificateRegistration,
    type IImportSigningIssuersOutput,
    type ISigningChecks,
    type IStaffCrl,
    type IStaffCertificate,
    type IStaffIssuer,
} from "@/lib/signing/types"
import {
    CertificateDisplayStatus,
    DEFAULT_CHECKS,
    certificateStatus,
    commonName,
    filterCertificates,
    issuerUpload,
    personName,
    shortFingerprint,
} from "./signingSettings"
import {
    useDeleteIssuer,
    useImportIssuers,
    usePutChecks,
    useRevokeCertificate,
    useScopeNames,
    useSigningCertificates,
    useSigningEventInfo,
    useWriteError,
} from "./useSigningSettings"
import {CERTIFICATE_FILES, FileButton, RegisterCertificateDialog} from "./RegisterCertificateDialog"
import type {ISignaturesSubTabProps} from "./ProtectedActionsTab"

const mono = {fontFamily: "monospace", fontSize: "0.75rem"}

/**
 * Dates, and times with the zone's name, in the event's time zone, as the
 * signing panel shows them; and the signers' titles.
 */
const useEventFormat = (electionEventId: string) => {
    const {titles} = useSigningEventInfo(electionEventId)
    const zoned = useEventZonedFormat(electionEventId)
    return {date: zoned.format, time: zoned.format, titles}
}

/** Under a person's name: their username when the name isn't it, and their title. */
const PersonDetail: React.FC<{username: string | null; title: string | null}> = ({
    username,
    title,
}) =>
    username || title ? (
        <Typography variant="body2" color="text.secondary">
            {username ? <span>{username}</span> : null}
            {title ? <span>{username ? ` · ${title}` : title}</span> : null}
        </Typography>
    ) : null

/** A collapsible card; its region is named by its title. Without its write permission it's read-only. */
const Card: React.FC<React.PropsWithChildren<{title: string; readOnly: boolean}>> = ({
    title,
    readOnly,
    children,
}) => {
    const {t} = useTranslation()
    const id = useId()
    return (
        <Accordion defaultExpanded disableGutters variant="outlined">
            <AccordionSummary id={id} expandIcon={<ExpandMoreIcon />}>
                <Typography variant="subtitle1" component="span" sx={{fontWeight: 600, flex: 1}}>
                    {title}
                </Typography>
                {readOnly && (
                    <Chip
                        size="small"
                        variant="outlined"
                        label={t("signing.readOnly.chip")}
                        sx={{mr: 1}}
                    />
                )}
            </AccordionSummary>
            <AccordionDetails>{children}</AccordionDetails>
        </Accordion>
    )
}

interface ICardProps extends ISignaturesSubTabProps {
    /** Reads the tab again after a change; it reports its own failure. */
    refetch: () => Promise<void>
}

const TrustedIssuersCard: React.FC<ICardProps & {issuers: IStaffIssuer[]}> = ({
    electionEventId,
    access,
    issuers,
    refetch,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const {date} = useEventFormat(electionEventId)
    const [importing, setImporting] = useState(false)
    const [file, setFile] = useState<({name: string} & ReturnType<typeof issuerUpload>) | null>(
        null
    )
    const [deleting, setDeleting] = useState<IStaffIssuer | null>(null)
    const [deleteOpen, setDeleteOpen] = useState(false)
    const writeError = useWriteError()
    const [importIssuers, {loading: saving}] = useImportIssuers()
    const [deleteIssuer] = useDeleteIssuer()

    const closeImport = () => {
        setImporting(false)
        setFile(null)
    }
    const runImport = async () => {
        if (!file) return
        let result: IImportSigningIssuersOutput | undefined
        try {
            const {data} = await importIssuers({
                variables: {
                    election_event_id: electionEventId,
                    pem: file.pem,
                    der_base64: file.der_base64,
                },
            })
            result = data?.signingImportIssuers
        } catch (error) {
            writeError(error, "signing.certificates.importError")
            return
        }
        const counts = {imported: result?.imported ?? 0, skipped: result?.skipped ?? 0}
        if (result?.errors.length) {
            notify(
                t("signing.certificates.importedWithErrors", {
                    ...counts,
                    errors: result.errors.join("; "),
                }),
                {type: "error"}
            )
        } else {
            notify(t("signing.certificates.imported", counts), {type: "success"})
        }
        closeImport()
        await refetch()
    }
    const runDelete = async (issuer: IStaffIssuer) => {
        setDeleteOpen(false)
        try {
            await deleteIssuer({
                variables: {election_event_id: electionEventId, issuer_id: issuer.id},
            })
        } catch (error) {
            writeError(error, "signing.certificates.deleteError")
            return
        }
        await refetch()
    }

    return (
        <Card title={t("signing.certificates.issuers")} readOnly={!access.issuersWrite}>
            <Box sx={{display: "flex", alignItems: "center", gap: 2, mb: 2}}>
                <Typography variant="body2" color="text.secondary" sx={{flex: 1}}>
                    {t("signing.certificates.issuersIntro")}
                </Typography>
                {access.issuersWrite && (
                    <Button variant="contained" onClick={() => setImporting(true)}>
                        {t("signing.certificates.import")}
                    </Button>
                )}
            </Box>
            <Table size="small" aria-label={t("signing.certificates.issuers")}>
                <TableHead>
                    <TableRow>
                        <TableCell>{t("signing.certificates.columns.issuer")}</TableCell>
                        <TableCell>{t("signing.certificates.columns.type")}</TableCell>
                        <TableCell>{t("signing.certificates.columns.issuedBy")}</TableCell>
                        <TableCell>{t("signing.certificates.columns.validUntil")}</TableCell>
                        <TableCell>{t("signing.certificates.columns.sha256")}</TableCell>
                        {access.issuersWrite && (
                            <TableCell>
                                <Box component="span" sx={visuallyHidden}>
                                    {t("common.label.actions")}
                                </Box>
                            </TableCell>
                        )}
                    </TableRow>
                </TableHead>
                <TableBody>
                    {issuers.length === 0 && (
                        <TableRow>
                            <TableCell colSpan={6}>{t("signing.certificates.noIssuers")}</TableCell>
                        </TableRow>
                    )}
                    {issuers.map((issuer) => {
                        const name = issuer.common_name ?? commonName(issuer.subject)
                        return (
                            <TableRow key={issuer.id}>
                                <TableCell sx={{fontWeight: 600}}>{name}</TableCell>
                                <TableCell>
                                    {t(
                                        issuer.subject === issuer.issuer
                                            ? "signing.certificates.root"
                                            : "signing.certificates.intermediate"
                                    )}
                                </TableCell>
                                <TableCell>
                                    {issuer.issuer_common_name ?? commonName(issuer.issuer)}
                                </TableCell>
                                <TableCell>{date(issuer.not_after)}</TableCell>
                                <TableCell sx={mono}>
                                    {shortFingerprint(issuer.fingerprint_sha256)}
                                </TableCell>
                                {access.issuersWrite && (
                                    <TableCell align="right">
                                        <IconButton
                                            size="small"
                                            aria-label={t("signing.certificates.deleteIssuer", {
                                                name,
                                            })}
                                            onClick={() => {
                                                setDeleting(issuer)
                                                setDeleteOpen(true)
                                            }}
                                        >
                                            <DeleteIcon fontSize="small" />
                                        </IconButton>
                                    </TableCell>
                                )}
                            </TableRow>
                        )
                    })}
                </TableBody>
            </Table>

            <Dialog open={importing} onClose={closeImport} fullWidth maxWidth="sm">
                <DialogTitle>{t("signing.certificates.import")}</DialogTitle>
                <DialogContent sx={{display: "grid", gap: 2}}>
                    <Typography variant="body2">{t("signing.certificates.importHelp")}</Typography>
                    <Box>
                        <FileButton
                            label={t("signing.certificates.chooseFile")}
                            accept={CERTIFICATE_FILES}
                            onFile={(chosen) =>
                                chosen.arrayBuffer().then(
                                    (bytes) =>
                                        setFile({
                                            name: chosen.name,
                                            ...issuerUpload(new Uint8Array(bytes)),
                                        }),
                                    () =>
                                        notify(t("signing.certificates.fileError"), {type: "error"})
                                )
                            }
                        />
                        {file && (
                            <Typography variant="body2" sx={{mt: 1}}>
                                {file.name}
                            </Typography>
                        )}
                    </Box>
                </DialogContent>
                <DialogActions>
                    <Button onClick={closeImport}>{t("common.label.cancel")}</Button>
                    <Button variant="contained" disabled={!file || saving} onClick={runImport}>
                        {t("common.label.import")}
                    </Button>
                </DialogActions>
            </Dialog>

            <Dialog open={deleteOpen} onClose={() => setDeleteOpen(false)}>
                <DialogTitle>{t("common.label.warning")}</DialogTitle>
                <DialogContent>
                    {deleting &&
                        t("signing.certificates.deleteIssuerConfirm", {
                            name: deleting.common_name ?? commonName(deleting.subject),
                        })}
                </DialogContent>
                <DialogActions>
                    <Button onClick={() => setDeleteOpen(false)}>{t("common.label.cancel")}</Button>
                    <Button
                        color="error"
                        variant="contained"
                        onClick={() => deleting && runDelete(deleting)}
                    >
                        {t("common.label.delete")}
                    </Button>
                </DialogActions>
            </Dialog>
        </Card>
    )
}

const ChecksCard: React.FC<ICardProps & {checks: ISigningChecks; crls: IStaffCrl[]}> = ({
    electionEventId,
    access,
    checks,
    crls,
    refetch,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const writeError = useWriteError()
    const {time} = useEventFormat(electionEventId)
    const [putChecks] = usePutChecks()
    // What a save in flight shows until the refetch returns the stored checks.
    const [saving, setSaving] = useState<ISigningChecks | null>(null)
    const shown = saving ?? checks
    const disabled = !access.checksWrite || !!saving

    const save = async (change: Partial<ISigningChecks>) => {
        const next = {...checks, ...change}
        setSaving(next)
        try {
            await putChecks({
                variables: {
                    election_event_id: electionEventId,
                    revocation_check: next.revocation_check,
                    crl_unavailable: next.crl_unavailable,
                    registration: next.registration,
                    post_binding: next.post_binding,
                    expected_revision: checks.revision,
                },
            })
            await refetch()
            notify(t("signing.certificates.checksSaved"), {type: "success"})
        } catch (error) {
            writeError(error, "signing.certificates.checksError")
        } finally {
            setSaving(null)
        }
    }

    return (
        <Card title={t("signing.certificates.checks")} readOnly={!access.checksWrite}>
            <Box sx={{display: "grid", gridTemplateColumns: {md: "1fr 1fr"}, gap: 3}}>
                <Box sx={{display: "grid", gap: 2, alignContent: "start"}}>
                    <Box>
                        <FormControlLabel
                            control={
                                <Switch
                                    checked={shown.revocation_check === RevocationCheck.Check}
                                    disabled={disabled}
                                    onChange={(event) =>
                                        save({
                                            revocation_check: event.target.checked
                                                ? RevocationCheck.Check
                                                : RevocationCheck.DontCheck,
                                        })
                                    }
                                />
                            }
                            label={t("signing.certificates.checkRevocation")}
                        />
                        <Box sx={{ml: 6}}>
                            <Typography variant="body2" color="text.secondary">
                                {t("signing.certificates.crlSchedule")}
                            </Typography>
                            {crls.map((crl) => (
                                <Typography
                                    key={crl.id}
                                    variant="body2"
                                    color={crl.status === CrlStatus.Ok ? "text.secondary" : "error"}
                                    sx={{wordBreak: "break-all"}}
                                >
                                    {t(
                                        crl.status === CrlStatus.Ok
                                            ? "signing.certificates.crlUpdated"
                                            : "signing.certificates.crlFailed",
                                        {
                                            url: crl.url,
                                            time: crl.fetched_at ? time(crl.fetched_at) : "–",
                                        }
                                    )}
                                </Typography>
                            ))}
                        </Box>
                    </Box>
                    <FormControl sx={{maxWidth: 380}} disabled={disabled}>
                        <InputLabel id="signing-crl-unavailable">
                            {t("signing.certificates.crlUnavailable.label")}
                        </InputLabel>
                        <Select
                            labelId="signing-crl-unavailable"
                            label={t("signing.certificates.crlUnavailable.label")}
                            value={shown.crl_unavailable}
                            onChange={(event) =>
                                save({crl_unavailable: event.target.value as CrlUnavailablePolicy})
                            }
                        >
                            {Object.values(CrlUnavailablePolicy).map((policy) => (
                                <MenuItem key={policy} value={policy}>
                                    {t(`signing.certificates.crlUnavailable.${policy}`)}
                                </MenuItem>
                            ))}
                        </Select>
                    </FormControl>
                </Box>
                <Box sx={{display: "grid", gap: 2, alignContent: "start"}}>
                    <FormControl disabled={disabled}>
                        <FormLabel
                            id="signing-registration"
                            sx={{"&.Mui-disabled": {color: "text.secondary"}}}
                        >
                            {t("signing.certificates.registration.label")}
                        </FormLabel>
                        <RadioGroup
                            aria-labelledby="signing-registration"
                            value={shown.registration}
                            onChange={(event) =>
                                save({registration: event.target.value as CertificateRegistration})
                            }
                        >
                            {Object.values(CertificateRegistration).map((registration) => (
                                <FormControlLabel
                                    key={registration}
                                    value={registration}
                                    control={<Radio />}
                                    label={t(`signing.certificates.registration.${registration}`)}
                                />
                            ))}
                        </RadioGroup>
                    </FormControl>
                    <FormControlLabel
                        control={
                            <Switch
                                checked={shown.post_binding === CertificatePostBinding.OnePost}
                                disabled={disabled}
                                onChange={(event) =>
                                    save({
                                        post_binding: event.target.checked
                                            ? CertificatePostBinding.OnePost
                                            : CertificatePostBinding.AnyPost,
                                    })
                                }
                            />
                        }
                        label={t("signing.certificates.onePost")}
                    />
                </Box>
            </Box>
        </Card>
    )
}

// "Expires soon" is marked by its icon and border: warning-colored text is too faint to read.
const STATUS_COLORS: Record<CertificateDisplayStatus, "success" | "error" | "default"> = {
    [CertificateDisplayStatus.Active]: "success",
    [CertificateDisplayStatus.ExpiresSoon]: "default",
    [CertificateDisplayStatus.Expired]: "error",
    [CertificateDisplayStatus.Revoked]: "default",
}

const RegisteredCertificatesCard: React.FC<ICardProps & {certificates: IStaffCertificate[]}> = ({
    electionEventId,
    access,
    certificates,
    refetch,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const {date, titles} = useEventFormat(electionEventId)
    const {postName} = useScopeNames(electionEventId)
    const [search, setSearch] = useState("")
    const [status, setStatus] = useState<CertificateDisplayStatus | null>(null)
    const [registering, setRegistering] = useState(false)
    // The certificate stays while the dialog closes, so its title doesn't empty mid-transition.
    const [revoking, setRevoking] = useState<IStaffCertificate | null>(null)
    const [revokeOpen, setRevokeOpen] = useState(false)
    const [reason, setReason] = useState("")
    const writeError = useWriteError()
    const [revoke, {loading: revokingSaving}] = useRevokeCertificate()
    const now = new Date()
    const postOf = (electionId: string | null) =>
        electionId ? postName(electionId) : t("signing.certificates.allPosts")
    const shown = filterCertificates(certificates, {search, status}, now, postOf)

    const closeRevoke = () => {
        setRevokeOpen(false)
        setReason("")
    }
    const runRevoke = async () => {
        if (!revoking) return
        try {
            await revoke({
                variables: {
                    election_event_id: electionEventId,
                    certificate_id: revoking.id,
                    reason: reason.trim(),
                },
            })
        } catch (error) {
            writeError(error, "signing.certificates.revokeError")
            return
        }
        notify(t("signing.certificates.revokeDone"), {type: "success"})
        closeRevoke()
        await refetch()
    }

    return (
        <Card
            title={t("signing.certificates.registeredTitle")}
            readOnly={!access.certificatesRegister && !access.certificatesRevoke}
        >
            <Box sx={{display: "flex", flexWrap: "wrap", alignItems: "center", gap: 2, mb: 2}}>
                <TextField
                    size="small"
                    label={t("signing.certificates.search")}
                    value={search}
                    onChange={(event) => setSearch(event.target.value)}
                    slotProps={{
                        input: {
                            startAdornment: (
                                <InputAdornment position="start">
                                    <SearchIcon />
                                </InputAdornment>
                            ),
                        },
                    }}
                    sx={{minWidth: 320}}
                />
                <FormControl size="small" sx={{minWidth: 200}}>
                    <InputLabel id="signing-certificate-status">
                        {t("signing.certificates.status")}
                    </InputLabel>
                    <Select
                        labelId="signing-certificate-status"
                        label={t("signing.certificates.status")}
                        value={status ?? "all"}
                        onChange={(event) =>
                            setStatus(
                                event.target.value === "all"
                                    ? null
                                    : (event.target.value as CertificateDisplayStatus)
                            )
                        }
                    >
                        <MenuItem value="all">{t("signing.certificates.statusAll")}</MenuItem>
                        {Object.values(CertificateDisplayStatus).map((value) => (
                            <MenuItem key={value} value={value}>
                                {t(`signing.certificates.statuses.${value}`)}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
                <Box sx={{flex: 1}} />
                {access.certificatesRegister && (
                    <Button variant="contained" onClick={() => setRegistering(true)}>
                        {t("signing.certificates.register")}
                    </Button>
                )}
            </Box>
            <Table size="small" aria-label={t("signing.certificates.registeredTitle")}>
                <TableHead>
                    <TableRow>
                        <TableCell>{t("signing.certificates.columns.person")}</TableCell>
                        <TableCell>{t("signing.certificates.columns.post")}</TableCell>
                        <TableCell>{t("signing.certificates.columns.certificate")}</TableCell>
                        <TableCell>{t("signing.certificates.columns.issuer")}</TableCell>
                        <TableCell>{t("signing.certificates.columns.validUntil")}</TableCell>
                        <TableCell>{t("signing.certificates.columns.registered")}</TableCell>
                        <TableCell>{t("signing.certificates.columns.status")}</TableCell>
                        {access.certificatesRevoke && (
                            <TableCell>
                                <Box component="span" sx={visuallyHidden}>
                                    {t("common.label.actions")}
                                </Box>
                            </TableCell>
                        )}
                    </TableRow>
                </TableHead>
                <TableBody>
                    {shown.length === 0 && (
                        <TableRow>
                            <TableCell colSpan={8}>
                                {t("signing.certificates.noCertificates")}
                            </TableCell>
                        </TableRow>
                    )}
                    {shown.map((certificate) => {
                        const display = certificateStatus(certificate, now)
                        const soon = display === CertificateDisplayStatus.ExpiresSoon
                        return (
                            <TableRow key={certificate.id}>
                                <TableCell>
                                    <Box sx={{fontWeight: 600}}>
                                        {personName(
                                            certificate.user_display_name,
                                            certificate.username
                                        )}
                                    </Box>
                                    {/* "username · title or role" (draft Settings 4). */}
                                    <PersonDetail
                                        username={
                                            certificate.user_display_name
                                                ? certificate.username
                                                : null
                                        }
                                        title={titles[certificate.user_id] ?? null}
                                    />
                                </TableCell>
                                <TableCell>{postOf(certificate.election_id)}</TableCell>
                                <TableCell>
                                    <div>{commonName(certificate.subject)}</div>
                                    <Box sx={mono}>
                                        {shortFingerprint(certificate.fingerprint_sha256)}
                                    </Box>
                                </TableCell>
                                <TableCell>{commonName(certificate.issuer)}</TableCell>
                                <TableCell sx={soon ? {fontWeight: 700} : undefined}>
                                    {date(certificate.not_after)}
                                </TableCell>
                                <TableCell>
                                    <div>{date(certificate.registered_at)}</div>
                                    <Typography variant="body2" color="text.secondary">
                                        {certificate.registration ===
                                            StaffCertificateRegistration.SecurityOfficer &&
                                        certificate.registered_by_name
                                            ? t("signing.certificates.registeredBy", {
                                                  name: certificate.registered_by_name,
                                              })
                                            : t(
                                                  `signing.certificates.registeredHow.${certificate.registration}`
                                              )}
                                    </Typography>
                                </TableCell>
                                <TableCell>
                                    <Chip
                                        size="small"
                                        variant={
                                            display === CertificateDisplayStatus.Revoked
                                                ? "filled"
                                                : "outlined"
                                        }
                                        color={STATUS_COLORS[display]}
                                        icon={
                                            soon ? (
                                                <WarningAmberIcon
                                                    color="warning"
                                                    fontSize="small"
                                                />
                                            ) : undefined
                                        }
                                        sx={soon ? {borderColor: "warning.main"} : undefined}
                                        label={
                                            display === CertificateDisplayStatus.Revoked &&
                                            certificate.revoked_at
                                                ? t("signing.certificates.revokedOn", {
                                                      date: date(certificate.revoked_at),
                                                  })
                                                : t(`signing.certificates.statuses.${display}`)
                                        }
                                    />
                                </TableCell>
                                {access.certificatesRevoke && (
                                    <TableCell align="right">
                                        {display !== CertificateDisplayStatus.Revoked && (
                                            <IconButton
                                                size="small"
                                                aria-label={t("signing.certificates.revokeOf", {
                                                    name: personName(
                                                        certificate.user_display_name,
                                                        certificate.username
                                                    ),
                                                })}
                                                onClick={() => {
                                                    setRevoking(certificate)
                                                    setRevokeOpen(true)
                                                }}
                                            >
                                                <BlockIcon fontSize="small" />
                                            </IconButton>
                                        )}
                                    </TableCell>
                                )}
                            </TableRow>
                        )
                    })}
                </TableBody>
            </Table>

            <RegisterCertificateDialog
                open={registering}
                electionEventId={electionEventId}
                certificates={certificates}
                onClose={() => setRegistering(false)}
                onRegistered={() => void refetch()}
            />

            <Dialog open={revokeOpen} onClose={closeRevoke} fullWidth maxWidth="sm">
                <DialogTitle>
                    {revoking &&
                        t("signing.certificates.revokeTitle", {
                            name: personName(revoking.user_display_name, revoking.username),
                        })}
                </DialogTitle>
                <DialogContent sx={{display: "grid", gap: 2, pt: "8px !important"}}>
                    <Typography variant="body2">{t("signing.certificates.revokeHelp")}</Typography>
                    <TextField
                        required
                        label={t("signing.certificates.revokeReason")}
                        value={reason}
                        onChange={(event) => setReason(event.target.value)}
                    />
                </DialogContent>
                <DialogActions>
                    <Button onClick={closeRevoke}>{t("common.label.cancel")}</Button>
                    <Button
                        color="error"
                        variant="contained"
                        disabled={!reason.trim() || revokingSaving}
                        onClick={runRevoke}
                    >
                        {t("signing.certificates.revoke")}
                    </Button>
                </DialogActions>
            </Dialog>
        </Card>
    )
}

/** Staff certificates: trusted issuers, the checks, and who has which certificate. */
export const CertificatesTab: React.FC<ISignaturesSubTabProps> = ({electionEventId, access}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const {data, loading, error, refetch} = useSigningCertificates(electionEventId)
    const reload = async () => {
        try {
            await refetch()
        } catch {
            notify(t("signing.loadError"), {type: "error"})
        }
    }

    if (error) {
        return <Alert severity="error">{t("signing.loadError")}</Alert>
    }
    if (loading || !data) {
        return <CircularProgress aria-label={t("common.label.loadingData")} />
    }
    const props = {electionEventId, access, refetch: reload}

    return (
        <Box sx={{display: "grid", gap: 2}}>
            <TrustedIssuersCard {...props} issuers={data.issuers} />
            <ChecksCard {...props} checks={data.checks[0] ?? DEFAULT_CHECKS} crls={data.crls} />
            <RegisteredCertificatesCard {...props} certificates={data.certificates} />
        </Box>
    )
}
