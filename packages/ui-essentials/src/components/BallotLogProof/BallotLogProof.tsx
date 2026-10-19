// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo} from "react"
import {Alert, Box, Button, Chip, Stack, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {verifyBallotLogProof} from "@sequentech/ui-core"

export type BallotLogTrust = Parameters<typeof verifyBallotLogProof>[0]
export type BallotLogEvidence = Parameters<typeof verifyBallotLogProof>[1]

export interface BallotLogProofProps {
    trust: BallotLogTrust
    proof: BallotLogEvidence
}

/** Both portals run the same WASM verifier over the signed bytes with published keys. */
export const BallotLogProof: React.FC<BallotLogProofProps> = ({trust, proof}) => {
    const {t} = useTranslation()
    const checked = useMemo(() => {
        try {
            return verifyBallotLogProof(trust, proof)
        } catch {
            return null
        }
    }, [trust, proof])
    const checks = checked?.checks
    const statuses = (["binding", "received", "ballotId", "cast"] as const).map(
        (key) => checks?.[key] ?? "failed"
    )
    const failed = statuses.some((status) => status === "failed")
    const pending = !failed && statuses.some((status) => status === "pending")
    const state = failed ? "failed" : pending ? "pending" : "passed"
    const download = () => {
        const blob = new Blob([JSON.stringify({trust, proof}, null, 2)], {type: "application/json"})
        const url = URL.createObjectURL(blob)
        const link = document.createElement("a")
        link.href = url
        link.download = `ballot-${proof.ballot_id.replace(/[^A-Z0-9-]/gi, "")}-signed-entries.json`
        link.click()
        setTimeout(() => URL.revokeObjectURL(url), 0)
    }
    return (
        <Stack spacing={2} sx={{minWidth: 0}}>
            <Alert severity={failed ? "error" : pending ? "info" : "success"}>
                <Typography fontWeight={700}>{t(`ballotPrivacy.${state}`)}</Typography>
                {t(`ballotPrivacy.${failed ? "failedHelp" : pending ? "pendingHelp" : "verified"}`)}
            </Alert>
            <Typography component="div">
                {t("ballotPrivacy.pseudonym")}: <Chip size="small" label={proof.pseudonym} />
            </Typography>
            <Box component="ol" sx={{pl: 3, m: 0}}>
                {(["binding", "received", "ballotId", "cast"] as const).map((key) => {
                    const status = checks?.[key] ?? "failed"
                    return (
                        <Box component="li" key={key} sx={{mb: 2}}>
                            <Typography fontWeight={600}>
                                {t(`ballotPrivacy.checkLabels.${key}`)}
                            </Typography>
                            <Typography variant="body2">
                                {t(`ballotPrivacy.checkDescriptions.${key}`)}
                            </Typography>
                            <Typography
                                variant="body2"
                                color={
                                    status === "failed"
                                        ? "error"
                                        : status === "passed"
                                          ? "success.main"
                                          : "text.secondary"
                                }
                            >
                                {t(`ballotPrivacy.states.${status}`)}
                            </Typography>
                        </Box>
                    )
                })}
            </Box>
            <Box component="dl" sx={{m: 0, overflowWrap: "anywhere"}}>
                <Typography component="dt" variant="body2">
                    {t("ballotPrivacy.systemKey")}
                </Typography>
                <Typography component="dd" variant="caption" sx={{m: 0, fontFamily: "monospace"}}>
                    {trust.system_public_key}
                </Typography>
                <Typography component="dt" variant="body2" sx={{mt: 1}}>
                    {t("ballotPrivacy.boxKey")}
                </Typography>
                <Typography component="dd" variant="caption" sx={{m: 0, fontFamily: "monospace"}}>
                    {trust.ballot_box_key.key_id}
                </Typography>
            </Box>
            <Typography variant="body2" color="text.secondary">
                {t("ballotPrivacy.neverPublished")}
            </Typography>
            <Button onClick={download}>{t("ballotPrivacy.download")}</Button>
        </Stack>
    )
}
