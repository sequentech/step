// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    ToggleButton,
    ToggleButtonGroup,
    Typography,
} from "@mui/material"
import RefreshIcon from "@mui/icons-material/Refresh"
import {ChannelIcon, ChannelLabel} from "@sequentech/ui-essentials"
import {EMessageChannel} from "@/types/messaging"
import {ChannelChat} from "./ChannelChat"
import {EmailInbox} from "./EmailInbox"
import {ReportResult} from "./ReportResult"
import {contentPreview, formatTime, newestFirst} from "./messageContent"
import {
    IFakeGetStartedAnswer,
    IFakeProviderMessage,
    SENT_MESSAGES_MODES,
    SentMessagesMode,
} from "./types"

/** Every message, newest first: Time | Channel | From | To | Message, and its report. */
export const AllMessagesTable: React.FC<{messages: IFakeProviderMessage[]}> = ({messages}) => {
    const {t, i18n} = useTranslation()
    if (messages.length === 0) {
        return <Typography color="text.secondary">{t("sentMessages.empty")}</Typography>
    }
    return (
        <TableContainer>
            <Table size="small" aria-label={t("sentMessages.tab")}>
                <TableHead>
                    <TableRow>
                        <TableCell>{t("sentMessages.column.time")}</TableCell>
                        <TableCell>{t("sentMessages.column.channel")}</TableCell>
                        <TableCell>{t("sentMessages.column.from")}</TableCell>
                        <TableCell>{t("sentMessages.column.to")}</TableCell>
                        <TableCell>{t("sentMessages.column.message")}</TableCell>
                        <TableCell>{t("sentMessages.column.report")}</TableCell>
                    </TableRow>
                </TableHead>
                <TableBody>
                    {newestFirst(messages).map((message) => (
                        <TableRow key={message.id}>
                            <TableCell sx={{whiteSpace: "nowrap"}}>
                                {formatTime(message.received_at, i18n.language)}
                            </TableCell>
                            <TableCell>
                                <ChannelLabel
                                    channel={message.channel}
                                    label={t(`messaging.channel.${message.channel}`)}
                                />
                            </TableCell>
                            <TableCell sx={{overflowWrap: "anywhere"}}>{message.sender}</TableCell>
                            <TableCell sx={{overflowWrap: "anywhere"}}>
                                {message.recipient}
                            </TableCell>
                            <TableCell
                                sx={{
                                    maxWidth: 420,
                                    overflow: "hidden",
                                    textOverflow: "ellipsis",
                                    whiteSpace: "nowrap",
                                }}
                            >
                                {contentPreview(message, t)}
                            </TableCell>
                            <TableCell>
                                <ReportResult report={message.report} />
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        </TableContainer>
    )
}

export interface ISentMessagesProps {
    messages: IFakeProviderMessage[]
    mode: SentMessagesMode
    onModeChange: (mode: SentMessagesMode) => void
    onRefresh: () => void
    /** A refresh the user asked for is running. */
    refreshing?: boolean
    /** The last refresh failed; the messages shown are the last list read. */
    refreshFailed?: boolean
    /** The Messenger view's Get Started box sends the pasted link here. */
    onGetStarted?: (link: string) => Promise<IFakeGetStartedAnswer>
}

/**
 * What the dev fake provider routes received: every message in a table, or
 * one channel as its app shows it.
 */
export const SentMessages: React.FC<ISentMessagesProps> = ({
    messages,
    mode,
    onModeChange,
    onRefresh,
    refreshing = false,
    refreshFailed = false,
    onGetStarted,
}) => {
    const {t} = useTranslation()
    const shown = mode === "ALL" ? messages : messages.filter(({channel}) => channel === mode)
    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            <Typography variant="body2" color="text.secondary">
                {t("sentMessages.description")}
            </Typography>
            <Box
                sx={{
                    display: "flex",
                    flexWrap: "wrap",
                    gap: 2,
                    alignItems: "center",
                    justifyContent: "space-between",
                }}
            >
                <Box sx={{overflowX: "auto", maxWidth: "100%"}}>
                    <ToggleButtonGroup
                        size="small"
                        exclusive
                        value={mode}
                        onChange={(_event, value: SentMessagesMode | null) =>
                            value && onModeChange(value)
                        }
                        aria-label={t("sentMessages.modeLabel")}
                    >
                        {SENT_MESSAGES_MODES.map((option) => (
                            <ToggleButton
                                key={option}
                                value={option}
                                sx={{gap: 1, textTransform: "none", whiteSpace: "nowrap"}}
                            >
                                {option === "ALL" ? null : (
                                    <ChannelIcon channel={option} fontSize="small" />
                                )}
                                {option === "ALL"
                                    ? t("sentMessages.all")
                                    : t(`messaging.channel.${option}`)}
                            </ToggleButton>
                        ))}
                    </ToggleButtonGroup>
                </Box>
                <Button
                    variant="outlined"
                    startIcon={<RefreshIcon />}
                    onClick={onRefresh}
                    disabled={refreshing}
                >
                    {refreshing ? t("sentMessages.refreshing") : t("sentMessages.refresh")}
                </Button>
            </Box>
            {refreshFailed ? (
                <Alert severity="warning">{t("sentMessages.refreshError")}</Alert>
            ) : null}
            {mode === "ALL" ? (
                <AllMessagesTable messages={shown} />
            ) : mode === EMessageChannel.EMAIL ? (
                <EmailInbox messages={shown} />
            ) : (
                <ChannelChat
                    key={mode}
                    channel={mode}
                    messages={shown}
                    onGetStarted={onGetStarted}
                />
            )}
        </Box>
    )
}
