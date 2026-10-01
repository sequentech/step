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
import type {ISigningApi, ISigningPanelData} from "@/lib/signing/api"
import {
    PayloadMismatchError,
    checkDocument,
    documentKindOf,
    type ISignedView,
} from "@/lib/signing/request"
import {DocumentKind} from "@/lib/signing/types"
import {actionObject, documentTypeLabel, shortHash} from "./format"
import {problemMessage, useSignedView} from "./useSignedView"

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
            sx={{p: 2, display: "flex", gap: 2, alignItems: "center"}}
            data-testid="signing-document"
        >
            <DescriptionOutlinedIcon color="action" aria-hidden />
            <Box sx={{minWidth: 0}}>
                <Typography sx={{fontWeight: 600, overflowWrap: "anywhere"}}>
                    {data.document_name ?? actionObject(t, data.request.action)}
                </Typography>
                <Typography variant="body2" color="text.secondary" title={hash}>
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
                {data.document_url ? (
                    <Button
                        size="small"
                        sx={{px: 0, fontWeight: 600}}
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
                    <Alert severity="error" sx={{mt: 1}}>
                        {t(error)}
                    </Alert>
                ) : null}
            </Box>
        </Paper>
    )
}

/** The subject field of open and close voting with each channel's status before. */
const FROM_KEY = "from"

/**
 * A signed value in the organization's words: each code under
 * `signing.values.<key>.<code>`, else the code as signed. A `from` entry
 * reads `CHANNEL=STATUS`: "Online: Open".
 */
export const worded = (t: TFunction, key: string, raw: unknown, shown: string): string => {
    const word = (code: unknown) => {
        if (typeof code !== "string") return String(code)
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

    return (
        <Stack spacing={2}>
            {problem ? (
                <Alert severity="error" data-testid="signing-mismatch">
                    {t(problemMessage(problem))}
                </Alert>
            ) : null}
            {view && kind !== DocumentKind.NoDocument ? (
                <DocumentCard data={data} view={view} api={api} />
            ) : view && view.rows.length ? (
                <Table size="small" aria-label={t("signing.widget.panel.details")}>
                    <TableBody>
                        {view.rows.map((row) => (
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
                    </TableBody>
                </Table>
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
