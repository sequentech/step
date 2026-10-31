// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo} from "react"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Alert,
    Box,
    Button,
    ButtonBase,
    Stack,
    Typography,
} from "@mui/material"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"
import {
    faCheck,
    faChevronDown,
    faClock,
    faDownload,
    faLock,
    faShieldHalved,
    faXmark,
} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import {verifyBallotLogProof} from "@sequentech/ui-core"
import {PseudonymChip} from "../PseudonymChip"

/** A voter pseudonym is shown with a colon (FTBE:MHRX), never as a Tracking ID. */
const shownPseudonym = (code: string): string =>
    /^[0-9A-Za-z]{4}-[0-9A-Za-z]{4}$/.test(code) ? `${code.slice(0, 4)}:${code.slice(5)}` : code

export type BallotLogTrust = Parameters<typeof verifyBallotLogProof>[0]
export type BallotLogEvidence = Parameters<typeof verifyBallotLogProof>[1]

export interface BallotLogProofProps {
    trust: BallotLogTrust
    proof: BallotLogEvidence
    onRetry?: () => void
}

/** Both portals run the same WASM verifier over the signed bytes with published keys. */
export const BallotLogProof: React.FC<BallotLogProofProps> = ({trust, proof, onRetry}) => {
    const {t} = useTranslation()
    const checked = useMemo(() => {
        try {
            return verifyBallotLogProof(trust, proof)
        } catch {
            return null
        }
    }, [trust, proof])
    const checks = checked?.checks
    const steps = ["binding", "received", "ballotId", "cast"] as const
    const statuses = steps.map((key) => checks?.[key] ?? "failed")
    const failed = statuses.some((status) => status === "failed")
    // A ballot that is received or audited has no cast entry, and none is promised:
    // the cast step is open, not late.
    const stage = proof.stage === "received" || proof.stage === "audited" ? proof.stage : null
    const earlier = statuses.slice(0, 3).every((status) => status === "passed")
    const open = !failed && stage !== null && earlier && checks?.cast === "pending"
    const pending = !failed && !open && statuses.some((status) => status === "pending")
    const state = failed ? "failed" : open ? stage : pending ? "pending" : "passed"
    const entries = [
        {label: "received", bytes: proof.received_entry},
        {label: "cast", bytes: proof.cast_entry},
    ].filter((entry) => !!entry.bytes)
    const download = () => {
        const blob = new Blob([JSON.stringify({trust, proof}, null, 2)], {type: "application/json"})
        const url = URL.createObjectURL(blob)
        const link = document.createElement("a")
        link.href = url
        link.download = `ballot-${(proof.tracking_id ?? proof.ballot_id).replace(/[^A-Z0-9-]/gi, "")}-signed-entries.json`
        link.click()
        setTimeout(() => URL.revokeObjectURL(url), 0)
    }
    return (
        <Stack spacing={2.5} sx={{minWidth: 0, fontFamily: "Roboto, Helvetica, Arial, sans-serif"}}>
            <Alert
                severity={failed ? "error" : pending || open ? "info" : "success"}
                variant="outlined"
                icon={<FontAwesomeIcon icon={pending || open ? faClock : faShieldHalved} />}
                sx={{
                    borderRadius: "8px",
                    bgcolor: failed ? "#FEF2F2" : pending || open ? "#EFF6FF" : "#ECFDF5",
                }}
            >
                <Typography fontWeight={700} sx={{m: 0}}>
                    {open ? t(`ballotPrivacy.stages.${stage}.title`) : t(`ballotPrivacy.${state}`)}
                </Typography>
                <Typography variant="body2" sx={{m: 0}}>
                    {open
                        ? t(`ballotPrivacy.stages.${stage}.help`)
                        : t(
                              `ballotPrivacy.${failed ? "failedHelp" : pending ? "pendingHelp" : "verified"}`
                          )}
                </Typography>
            </Alert>
            {proof.tracking_id ? (
                <Box>
                    <Typography variant="body2" color="text.secondary" sx={{m: 0}}>
                        {t("ballotPrivacy.trackingId")}
                    </Typography>
                    <Typography
                        className="ballot-proof-tracking-id"
                        sx={{
                            m: 0,
                            fontFamily: "monospace",
                            fontSize: 15,
                            fontWeight: 700,
                            overflowWrap: "anywhere",
                        }}
                    >
                        {proof.tracking_id}
                    </Typography>
                    <Typography variant="caption" color="text.secondary" sx={{m: 0}}>
                        {t("ballotPrivacy.trackingIdHelp")}
                    </Typography>
                </Box>
            ) : null}
            <Stack direction="row" gap={1} alignItems="center">
                <Typography variant="body2" color="text.secondary" sx={{m: 0}}>
                    {t("ballotPrivacy.pseudonym")}
                </Typography>
                <PseudonymChip>{shownPseudonym(proof.pseudonym)}</PseudonymChip>
            </Stack>
            <Box component="ol" sx={{listStyle: "none", p: 0, m: 0}}>
                {steps.map((key, index) => {
                    const status = checks?.[key] ?? "failed"
                    const color =
                        status === "failed"
                            ? "error.main"
                            : status === "passed"
                              ? "#047857"
                              : "#64748B"
                    return (
                        <Box
                            component="li"
                            key={key}
                            sx={{
                                display: "grid",
                                gridTemplateColumns: "28px minmax(0, 1fr)",
                                columnGap: 1.75,
                            }}
                        >
                            <Box
                                sx={{
                                    display: "flex",
                                    flexDirection: "column",
                                    alignItems: "center",
                                }}
                            >
                                <Box
                                    sx={{
                                        width: 28,
                                        height: 28,
                                        display: "grid",
                                        placeItems: "center",
                                        borderRadius: "50%",
                                        bgcolor: color,
                                        color: "white",
                                        flexShrink: 0,
                                    }}
                                >
                                    <FontAwesomeIcon
                                        icon={
                                            status === "passed"
                                                ? faCheck
                                                : status === "pending"
                                                  ? faClock
                                                  : faXmark
                                        }
                                        style={{fontSize: 14}}
                                    />
                                </Box>
                                {index < steps.length - 1 && (
                                    <Box
                                        sx={{
                                            width: "2px",
                                            flex: 1,
                                            minHeight: 24,
                                            bgcolor: status === "passed" ? "#A7D9CC" : "divider",
                                            my: 0.25,
                                        }}
                                    />
                                )}
                            </Box>
                            <Box sx={{pb: index < steps.length - 1 ? 3 : 0, pt: 0.25}}>
                                <Stack
                                    direction="row"
                                    useFlexGap
                                    flexWrap="wrap"
                                    justifyContent="space-between"
                                    gap={1}
                                >
                                    <Typography sx={{m: 0, fontSize: 15, fontWeight: 700}}>
                                        {t(`ballotPrivacy.checkLabels.${key}`)}
                                    </Typography>
                                    <Typography variant="caption" sx={{m: 0, color}}>
                                        {key === "cast" && open
                                            ? t(`ballotPrivacy.stages.${stage}.castState`)
                                            : t(`ballotPrivacy.states.${status}`)}
                                    </Typography>
                                </Stack>
                                <Typography variant="body2" sx={{m: 0, mt: 0.75}}>
                                    {t(`ballotPrivacy.checkDescriptions.${key}`)}
                                </Typography>
                            </Box>
                        </Box>
                    )
                })}
            </Box>
            <Box
                component="dl"
                sx={{
                    m: 0,
                    overflowWrap: "anywhere",
                    display: "grid",
                    gridTemplateColumns: {xs: "1fr", sm: "auto minmax(0, 1fr)"},
                    columnGap: 1.5,
                    rowGap: 0.5,
                    color: "text.secondary",
                }}
            >
                <Typography component="dt" variant="caption" sx={{m: 0}}>
                    {t("ballotPrivacy.systemKey")}
                </Typography>
                <Typography component="dd" variant="caption" sx={{m: 0, fontFamily: "monospace"}}>
                    {trust.system_public_key}
                </Typography>
                <Typography component="dt" variant="caption" sx={{m: 0}}>
                    {t("ballotPrivacy.boxKey")}
                </Typography>
                <Typography component="dd" variant="caption" sx={{m: 0, fontFamily: "monospace"}}>
                    {trust.ballot_box_key.key_id}
                </Typography>
            </Box>
            <Stack
                direction="row"
                alignItems="center"
                gap={1.5}
                sx={{p: 1.5, bgcolor: "#F7F9FC", border: "1px solid #DFE6EF", borderRadius: "8px"}}
            >
                <FontAwesomeIcon icon={faLock} style={{fontSize: 13, color: "#64748B"}} />
                <Typography variant="body2" sx={{m: 0}}>
                    {t("ballotPrivacy.neverPublished")}
                </Typography>
            </Stack>
            <Box sx={{position: "relative", borderTop: "1px solid", borderColor: "divider"}}>
                <ButtonBase
                    onClick={download}
                    aria-label={t("ballotPrivacy.download")}
                    sx={{
                        position: "absolute",
                        top: 0,
                        right: 0,
                        zIndex: 1,
                        gap: 1,
                        px: 1,
                        py: 1.5,
                        color: "#0F054C",
                        fontSize: 14,
                    }}
                >
                    <FontAwesomeIcon icon={faDownload} />
                    {t("ballotPrivacy.downloadAction")}
                </ButtonBase>
                <Accordion
                    disableGutters
                    elevation={0}
                    sx={{
                        "bgcolor": "transparent",
                        "&:before": {display: "none"},
                    }}
                >
                    <AccordionSummary
                        expandIcon={<FontAwesomeIcon icon={faChevronDown} style={{fontSize: 12}} />}
                        sx={{pl: 0, pr: 15, minHeight: 48, flexDirection: "row-reverse", gap: 1}}
                    >
                        <Typography variant="body2" sx={{m: 0}}>
                            {t("ballotPrivacy.signedEntries", {count: entries.length})}
                        </Typography>
                    </AccordionSummary>
                    <AccordionDetails sx={{px: 0}}>
                        <Stack spacing={2}>
                            {entries.map(({label, bytes}) => (
                                <Box key={label}>
                                    <Typography variant="body2" fontWeight={600} sx={{m: 0}}>
                                        {t(`ballotPrivacy.checkLabels.${label}`)}
                                    </Typography>
                                    <Box
                                        component="pre"
                                        sx={{
                                            m: 0,
                                            mt: 1,
                                            p: 1.5,
                                            bgcolor: "#F7F9FC",
                                            border: "1px solid #DFE6EF",
                                            borderRadius: "4px",
                                            fontSize: 11,
                                            whiteSpace: "pre-wrap",
                                            overflowWrap: "anywhere",
                                        }}
                                    >
                                        {bytes}
                                    </Box>
                                </Box>
                            ))}
                        </Stack>
                    </AccordionDetails>
                </Accordion>
            </Box>
            {pending && onRetry ? (
                <Button className="ballot-proof-retry-button" onClick={onRetry}>
                    {t("ballotPrivacy.retry")}
                </Button>
            ) : null}
        </Stack>
    )
}
