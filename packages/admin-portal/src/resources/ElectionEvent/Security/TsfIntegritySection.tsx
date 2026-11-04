// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    Typography,
    type AlertColor,
} from "@mui/material"
import {CopyableFingerprint} from "@/components/CopyableFingerprint"
import {shortHash} from "@/components/signing/format"

interface TsfIntegrityPart {
    part: string
    accepted_digest?: string | null
    running: number
    outcome: string
    detail?: string | null
}
export interface TsfIntegrityCheck {
    id: string
    requested_by: string
    started_at: string
    finished_at?: string | null
    outcome?: string | null
    result: {
        reason?: string | null
        toe_reference?: string | null
        digest_list_sha256?: string | null
        parts?: TsfIntegrityPart[]
    }
}
interface ITsfIntegritySectionProps {
    /** The event's latest check; null when none has been requested. */
    check?: TsfIntegrityCheck | null
    loaded: boolean
    disabled: boolean
    onVerify: () => void
    format: (value: string) => string
}

const SEVERITY: Record<string, AlertColor> = {
    verified: "success",
    different: "error",
    unverifiable: "warning",
}

const Fact: React.FC<{label: string; children: React.ReactNode}> = ({label, children}) => (
    <>
        <Typography component="dt" variant="body2" sx={{fontWeight: 600}}>
            {label}
        </Typography>
        <Typography component="dd" variant="body2" sx={{m: 0, overflowWrap: "anywhere"}}>
            {children}
        </Typography>
    </>
)

/** The on-demand comparison of the running TOE software with the accepted delivery. */
export const TsfIntegritySection: React.FC<ITsfIntegritySectionProps> = ({
    check,
    loaded,
    disabled,
    onVerify,
    format,
}) => {
    const {t} = useTranslation()
    const running = !!check && !check.finished_at
    const result = check?.result
    return (
        <Box>
            <Typography variant="h6" sx={{mb: 1}}>
                {t("tsfIntegrity.title")}
            </Typography>
            <Typography sx={{mb: 2}}>{t("tsfIntegrity.description")}</Typography>
            <Button
                variant="contained"
                disabled={disabled || running}
                onClick={onVerify}
                sx={{mb: 2}}
            >
                {t(running ? "tsfIntegrity.running" : "tsfIntegrity.run")}
            </Button>
            {loaded && !check && <Alert severity="info">{t("tsfIntegrity.none")}</Alert>}
            {check && (
                <Stack spacing={2}>
                    <Alert severity={running ? "info" : SEVERITY[check.outcome ?? "unverifiable"]}>
                        <Typography sx={{fontWeight: 600}}>
                            {t(
                                running
                                    ? "tsfIntegrity.running"
                                    : `tsfIntegrity.outcome.${check.outcome}`
                            )}
                        </Typography>
                        {result?.reason && <Typography>{result.reason}</Typography>}
                    </Alert>
                    <Box
                        component="dl"
                        sx={{
                            display: "grid",
                            gridTemplateColumns: {xs: "1fr", sm: "max-content 1fr"},
                            columnGap: 2,
                            rowGap: 0.5,
                        }}
                    >
                        <Fact label={t("tsfIntegrity.started")}>{format(check.started_at)}</Fact>
                        <Fact label={t("tsfIntegrity.requestedBy")}>{check.requested_by}</Fact>
                        {result?.toe_reference && (
                            <Fact label={t("tsfIntegrity.toeReference")}>
                                {result.toe_reference}
                            </Fact>
                        )}
                        {result?.digest_list_sha256 && (
                            <Fact label={t("tsfIntegrity.digestList")}>
                                <Box component="span" sx={{fontFamily: "monospace"}}>
                                    <CopyableFingerprint
                                        value={result.digest_list_sha256}
                                        short={result.digest_list_sha256}
                                    />
                                </Box>
                                <Typography variant="caption" component="div">
                                    {t("tsfIntegrity.digestListHelp")}
                                </Typography>
                            </Fact>
                        )}
                    </Box>
                    {!!result?.parts?.length && (
                        <Box sx={{overflowX: "auto"}}>
                            <Table size="small" aria-label={t("tsfIntegrity.parts")}>
                                <TableHead>
                                    <TableRow>
                                        <TableCell>{t("tsfIntegrity.part")}</TableCell>
                                        <TableCell>{t("tsfIntegrity.acceptedDigest")}</TableCell>
                                        <TableCell>{t("tsfIntegrity.runningContainers")}</TableCell>
                                        <TableCell>{t("tsfIntegrity.result")}</TableCell>
                                    </TableRow>
                                </TableHead>
                                <TableBody>
                                    {result.parts.map((part) => (
                                        <TableRow key={part.part}>
                                            <TableCell>{part.part}</TableCell>
                                            <TableCell sx={{fontFamily: "monospace"}}>
                                                {part.accepted_digest ? (
                                                    <CopyableFingerprint
                                                        value={part.accepted_digest}
                                                        short={shortHash(
                                                            part.accepted_digest.replace(
                                                                "sha256:",
                                                                ""
                                                            )
                                                        )}
                                                    />
                                                ) : (
                                                    "—"
                                                )}
                                            </TableCell>
                                            <TableCell>{part.running}</TableCell>
                                            <TableCell>
                                                {t(`tsfIntegrity.partOutcome.${part.outcome}`)}
                                                {part.detail && (
                                                    <Typography variant="body2">
                                                        {part.detail}
                                                    </Typography>
                                                )}
                                            </TableCell>
                                        </TableRow>
                                    ))}
                                </TableBody>
                            </Table>
                        </Box>
                    )}
                </Stack>
            )}
        </Box>
    )
}
