// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Button, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useZonedTime} from "@/hooks/useZonedTime"
import {ERecountOutcome, IVoterCountStatus} from "@/queries/GetMessagingVoterCounts"

export interface MessagingVoterCountStatusProps {
    /** `undefined` while the counts load. */
    status?: IVoterCountStatus | null
    /** Asks for a recount; absent where a recount can't be asked for. */
    onRecount?: () => void
    /** A recount is being asked for. */
    recounting?: boolean
}

/**
 * When an event's voter counts were counted, and why they may be older:
 * counting a realm takes time, so it runs in the background (after each
 * save of the event's messaging configuration, regularly, and on Recount),
 * never while the page loads. A recount that took too long or failed
 * keeps the previous counts, and says so.
 */
export const MessagingVoterCountStatus: React.FC<MessagingVoterCountStatusProps> = ({
    status,
    onRecount,
    recounting,
}) => {
    const {t} = useTranslation()
    const zonedTime = useZonedTime()
    if (!status) return null
    const lines = [
        status.computed_at
            ? t("messaging.voterCount.countedAt", {time: zonedTime(status.computed_at)})
            : t("messaging.voterCount.never"),
    ]
    if (status.outcome === ERecountOutcome.TIMED_OUT) {
        lines.push(t("messaging.voterCount.timedOut"))
    } else if (status.outcome === ERecountOutcome.FAILED) {
        lines.push(t("messaging.voterCount.failed"))
    }
    if (status.outcome && status.outcome !== ERecountOutcome.COUNTED && status.computed_at) {
        lines.push(t("messaging.voterCount.previous"))
    }
    if (status.pending) lines.push(t("messaging.voterCount.pending"))
    return (
        <Box sx={{display: "flex", alignItems: "center", gap: 1, flexWrap: "wrap"}}>
            <Typography variant="caption" role="status" sx={{color: "text.secondary"}}>
                {lines.join(" ")}
            </Typography>
            {onRecount ? (
                <Button
                    size="small"
                    variant="outlined"
                    onClick={onRecount}
                    disabled={status.pending || recounting}
                >
                    {t("messaging.voterCount.recount")}
                </Button>
            ) : null}
        </Box>
    )
}
