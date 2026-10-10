// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useTranslation} from "react-i18next"
import {Box, Tooltip, Typography} from "@mui/material"
import {fakeReasonText} from "./messageContent"
import {EFakeReportState, IFakeReport} from "./types"

const HTTP_OK = 200

/**
 * The delivery report the fake posted for a message and what the webhook
 * component answered: ✓✓ for 200, otherwise the status or why it wasn't
 * posted, so a refused report is visible rather than silent.
 */
export const ReportResult: React.FC<{report?: IFakeReport | null}> = ({report}) => {
    const {t} = useTranslation()
    if (!report || report.state === EFakeReportState.NO_RECEIPTS) {
        return (
            <Typography variant="caption" color="text.secondary">
                {t("sentMessages.report.unavailable")}
            </Typography>
        )
    }
    if (report.state === EFakeReportState.ANSWERED && report.status === HTTP_OK) {
        const label = t("sentMessages.report.accepted")
        return (
            <Tooltip title={label}>
                <Box
                    component="span"
                    role="img"
                    aria-label={label}
                    sx={{
                        color: "info.main",
                        fontWeight: 700,
                        letterSpacing: "-0.2em",
                        fontSize: "0.85rem",
                    }}
                >
                    ✓✓
                </Box>
            </Tooltip>
        )
    }
    const pending = report.state === EFakeReportState.PENDING
    const text = pending
        ? t("sentMessages.report.pending")
        : report.state === EFakeReportState.ANSWERED
          ? t("sentMessages.report.refused", {status: report.status ?? "?"})
          : report.state === EFakeReportState.NOT_SENT
            ? t("sentMessages.report.notSent", {reason: fakeReasonText(t, report.reason)})
            : t("sentMessages.report.noAnswer", {reason: fakeReasonText(t, report.reason)})
    return (
        <Typography
            variant="caption"
            sx={{color: pending ? "text.secondary" : "error.dark", fontWeight: pending ? 400 : 600}}
        >
            {text}
        </Typography>
    )
}
