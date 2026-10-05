// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import type {TFunction} from "i18next"
import {
    Alert,
    Box,
    Button,
    CircularProgress,
    Paper,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableRow,
    Typography,
} from "@mui/material"
import DescriptionOutlinedIcon from "@mui/icons-material/DescriptionOutlined"
import OpenInNewIcon from "@mui/icons-material/OpenInNew"
import type {ISigningApi, ISigningPanelData} from "@/lib/signing/api"
import {
    PayloadMismatchError,
    checkDocument,
    documentKindOf,
    type ISignedView,
} from "@/lib/signing/request"
import {DocumentKind, SigningAction} from "@/lib/signing/types"
import {actionObject, documentTypeLabel, shortHash} from "./format"
import {problemMessage, useSignedView} from "./useSignedView"
import {ConfigurationAuthorizes, LIFECYCLE_SUBJECT_KEYS} from "./ConfigurationAuthorizes"

export enum SigningSubjectVariant {
    Panel = "panel",
    Dialog = "dialog",
}

const MIME_TYPE: Record<DocumentKind, string> = {
    [DocumentKind.Pdf]: "application/pdf",
    [DocumentKind.Eml]: "application/xml",
    [DocumentKind.NoDocument]: "application/octet-stream",
}

/** A blob URL outlives its tab only briefly: long enough for the viewer to load it. */
const BLOB_URL_LIFETIME_MS = 60_000

/**
 * "Open the document": downloads it, checks it against the hash the signed
 * payload names, and shows those verified bytes; a different document is refused.
 */
const DocumentCard: React.FC<{data: ISigningPanelData; view: ISignedView; api: ISigningApi}> = ({
    data,
    view,
    api,
}) => {
    const {t} = useTranslation()
    const kind = documentKindOf(data.request)
    const hash = view.documentSha256 ?? ""
    const [opening, setOpening] = useState(false)
    const [error, setError] = useState<string | null>(null)

    const openDocument = async () => {
        if (!data.document_url) return
        setOpening(true)
        setError(null)
        try {
            const bytes = await api.fetchDocument(data.document_url)
            await checkDocument(bytes, view)
            const url = URL.createObjectURL(new Blob([bytes], {type: MIME_TYPE[kind]}))
            window.open(url, "_blank", "noopener")
            setTimeout(() => URL.revokeObjectURL(url), BLOB_URL_LIFETIME_MS)
        } catch (failure) {
            setError(
                failure instanceof PayloadMismatchError
                    ? "signing.widget.documentMismatch"
                    : "signing.widget.documentError"
            )
        } finally {
            setOpening(false)
        }
    }

    return (
        <Paper
            variant="outlined"
            sx={{
                p: 2,
                display: "grid",
                gridTemplateColumns: "auto minmax(0, 1fr)",
                columnGap: 2,
                rowGap: 1,
                alignItems: "center",
            }}
            data-testid="signing-document"
        >
            <DescriptionOutlinedIcon color="action" aria-hidden />
            <Box sx={{minWidth: 0}}>
                <Typography sx={{fontWeight: 600, overflowWrap: "anywhere", m: 0}}>
                    {data.document_name ?? actionObject(t, data.request.action)}
                </Typography>
                <Typography
                    variant="body2"
                    color="text.secondary"
                    title={hash}
                    sx={{m: 0, mt: 0.5}}
                >
                    {data.document_pages
                        ? t("signing.widget.documentPages", {
                              type: documentTypeLabel(kind),
                              pages: data.document_pages,
                              hash: shortHash(hash),
                          })
                        : t("signing.widget.document", {
                              type: documentTypeLabel(kind),
                              hash: shortHash(hash),
                          })}
                </Typography>
            </Box>
            {data.document_url ? (
                <Button
                    variant="secondary"
                    size="small"
                    sx={{
                        px: 1.5,
                        py: 0.5,
                        gridColumn: {xs: "1 / -1", sm: 2},
                        justifySelf: "start",
                        minHeight: {xs: 44, sm: 36},
                        fontSize: "0.875rem",
                        fontWeight: 600,
                    }}
                    endIcon={<OpenInNewIcon fontSize="small" />}
                    disabled={opening}
                    startIcon={opening ? <CircularProgress size={14} aria-hidden /> : undefined}
                    onClick={() => {
                        openDocument().catch(() => undefined)
                    }}
                >
                    {t("signing.panel.openDocument")}
                </Button>
            ) : null}
            {error ? (
                <Alert severity="error" sx={{gridColumn: "1 / -1"}}>
                    {t(error)}
                </Alert>
            ) : null}
        </Paper>
    )
}

/** The subject field of open and close voting with each channel's status before. */
const FROM_KEY = "from"
/** The subject field of a configuration version with the signing rules it changes. */
const SIGNING_RULES_KEY = "signing_rules"
/** A rule's value in a `signing_rules` entry when it needs no signatures. */
const RULE_OFF = "off"

/**
 * A `signing_rules` entry, `ACTION=BEFORE>AFTER` (or `ACTION=AFTER` when what
 * it was is unknown; `ACTION` alone in requests started before entries carried
 * numbers), each value `off` or the signatures needed: "Close voting: needs 2
 * (was 1)".
 */
const ruleChange = (t: TFunction, code: string): string => {
    const separator = code.indexOf("=")
    const action = separator > 0 ? code.slice(0, separator) : code
    const name = t(`signing.values.signing_rules.${action}`, {defaultValue: action})
    if (separator <= 0) return name
    const [before, after] = code.slice(separator + 1).split(">", 2)
    const off = (value: string) => (value === RULE_OFF ? t("signing.values.ruleOff") : null)
    const current = after ?? before
    const now = off(current) ?? t("signing.values.ruleNeeds", {n: current})
    return after !== undefined && after !== before
        ? t("signing.values.ruleChangeFrom", {action: name, rule: now, was: off(before) ?? before})
        : t("signing.values.ruleChange", {action: name, rule: now})
}

/**
 * A signed value in the organization's words: each code under
 * `signing.values.<key>.<code>`, else the code as signed. A `from` entry
 * reads `CHANNEL=STATUS`: "Online: Open".
 */
export const worded = (t: TFunction, key: string, raw: unknown, shown: string): string => {
    const word = (code: unknown) => {
        if (typeof code !== "string") return String(code)
        if (key === SIGNING_RULES_KEY) return ruleChange(t, code)
        const separator = code.indexOf("=")
        if (key === FROM_KEY && separator > 0) {
            const channel = code.slice(0, separator)
            const status = code.slice(separator + 1)
            return t("signing.values.channelStatus", {
                channel: t(`signing.values.channels.${channel}`, {defaultValue: channel}),
                status: t(`signing.values.statuses.${status}`, {defaultValue: status}),
            })
        }
        return t(`signing.values.${key}.${code}`, {defaultValue: code})
    }
    if (Array.isArray(raw)) return raw.map(word).join(", ")
    return typeof raw === "string" ? word(raw) : shown
}

/** A row that explains an action beside what it signs; it is not signed. */
export interface ISubjectNote {
    label: string
    value: string
}

/** The keys of each action's notes, under `signing.notes`. */
const NOTES: Partial<Record<SigningAction, Array<[label: string, value: string]>>> = {
    [SigningAction.ApproveVoter]: [["afterApproval", "afterApprovalValue"]],
    [SigningAction.ConfirmKeyShare]: [
        ["keyShare", "keyShareChecked"],
        ["recordedIn", "recordedInCeremony"],
    ],
    [SigningAction.ContributeKeyShare]: [
        ["keyShare", "keyShareChecked"],
        ["recordedIn", "recordedInTally"],
    ],
}

/** What the details table adds after the signed rows of `action` (drafts Other staff 1 and 3). */
export const subjectNotes = (t: TFunction, action: SigningAction): ISubjectNote[] =>
    (NOTES[action] ?? []).map(([label, value]) => ({
        label: t(`signing.notes.${label}`),
        value: t(`signing.notes.${value}`),
    }))

/**
 * What is signed, read from the canonical payload the approval signs: the
 * document card (name, type, pages, SHA-256), or the details table for an
 * action without a document; then the signing code everyone who signs
 * compares. A payload that doesn't describe the request is refused here.
 */
export const SigningSubject: React.FC<{
    data: ISigningPanelData
    api: ISigningApi
    variant?: SigningSubjectVariant
}> = ({data, api, variant = SigningSubjectVariant.Panel}) => {
    const {t} = useTranslation()
    const {view, problem} = useSignedView(data)
    const kind = documentKindOf(data.request)
    const notes = subjectNotes(t, data.request.action)
    // A configuration approval shows its lifecycle snapshot in words, not as rows.
    const configuration = data.request.action === SigningAction.ApproveConfiguration
    const rows = (view?.rows ?? []).filter(
        (row) => !configuration || !LIFECYCLE_SUBJECT_KEYS.includes(row.key)
    )

    return (
        <Stack spacing={2}>
            {problem ? (
                <Alert severity="error" data-testid="signing-mismatch">
                    {t(problemMessage(problem))}
                </Alert>
            ) : null}
            {view && kind !== DocumentKind.NoDocument ? (
                <DocumentCard data={data} view={view} api={api} />
            ) : view && (rows.length || notes.length) ? (
                <Box>
                    {data.request.action === SigningAction.ApproveConfiguration ? (
                        <Typography variant="subtitle2" component="h3" sx={{mb: 1}}>
                            {t("signing.panel.configurationChanges")}
                        </Typography>
                    ) : null}
                    <Table size="small" aria-label={t("signing.widget.panel.details")}>
                        <TableBody>
                            {rows.map((row) => (
                                <TableRow key={row.key}>
                                    <TableCell
                                        component="th"
                                        scope="row"
                                        sx={{color: "text.secondary", width: "40%"}}
                                    >
                                        {t(`signing.details.${row.key}`, {
                                            defaultValue: row.label ?? row.key,
                                        })}
                                    </TableCell>
                                    <TableCell sx={{overflowWrap: "anywhere"}}>
                                        {row.name ? (
                                            <>
                                                {row.name}
                                                <Typography
                                                    variant="body2"
                                                    color="text.secondary"
                                                    component="div"
                                                >
                                                    {row.value}
                                                </Typography>
                                            </>
                                        ) : (
                                            worded(t, row.key, view.subject[row.key], row.value)
                                        )}
                                    </TableCell>
                                </TableRow>
                            ))}
                            {notes.map((note) => (
                                <TableRow key={note.label} data-testid="signing-note">
                                    <TableCell
                                        component="th"
                                        scope="row"
                                        sx={{color: "text.secondary", width: "40%"}}
                                    >
                                        {note.label}
                                    </TableCell>
                                    <TableCell>{note.value}</TableCell>
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                </Box>
            ) : null}
            {view && configuration ? (
                <ConfigurationAuthorizes
                    subject={view.subject}
                    electionEventId={data.request.election_event_id}
                    requestId={data.request.id}
                />
            ) : null}
            <Stack
                direction={{xs: "column", sm: "row"}}
                spacing={1}
                sx={{alignItems: {sm: "baseline"}, justifyContent: "space-between"}}
            >
                <Typography component="div">
                    {t("signing.panel.signingCode")}{" "}
                    <Box
                        component="span"
                        data-testid="signing-code"
                        sx={{fontFamily: "monospace", fontSize: "1.2rem", fontWeight: 700, ml: 1}}
                    >
                        {data.request.code}
                    </Box>
                </Typography>
                {variant === SigningSubjectVariant.Dialog ? (
                    <Typography variant="body2" color="text.secondary">
                        {t("signing.dialog.check.sameCode")}
                    </Typography>
                ) : null}
            </Stack>
        </Stack>
    )
}
