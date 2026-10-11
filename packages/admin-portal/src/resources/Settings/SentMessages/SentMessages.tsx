// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useId, useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    Link,
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
import {EmailInbox, EmailReadingPane} from "./EmailInbox"
import {ReportResult} from "./ReportResult"
import {
    contentPreview,
    formatTime,
    newestFirst,
    parameterText,
    templateLabel,
} from "./messageContent"
import {
    IFakeGetStartedAnswer,
    IFakeProviderMessage,
    SENT_MESSAGES_MODES,
    SentMessagesMode,
} from "./types"

const MessageDetails: React.FC<{message: IFakeProviderMessage}> = ({message}) => {
    const {t} = useTranslation()
    const {content} = message
    if (message.channel === EMessageChannel.EMAIL) {
        return <EmailReadingPane message={message} />
    }
    return (
        <Box
            sx={{
                display: "flex",
                flexDirection: "column",
                gap: 1,
                p: 2,
                overflowWrap: "anywhere",
                whiteSpace: "pre-wrap",
            }}
        >
            {content.template_name ? (
                <Typography variant="subtitle2" component="p">
                    {templateLabel(content, t)}
                </Typography>
            ) : null}
            {content.text ? (
                <Typography variant="body2" sx={{whiteSpace: "pre-wrap"}}>
                    {content.text}
                </Typography>
            ) : null}
            {(content.parameters ?? []).map((parameter, index) => (
                <Typography key={index} variant="body2">
                    {parameterText(parameter)}
                </Typography>
            ))}
            {content.code ? (
                <Typography variant="body2">
                    {t("sentMessages.content.code")}: {content.code}
                </Typography>
            ) : null}
            {!content.text && !content.template_name && !content.code ? (
                <Typography variant="body2">{t("sentMessages.content.noText")}</Typography>
            ) : null}
            {content.truncated ? (
                <Typography variant="caption" color="text.secondary">
                    {t("sentMessages.content.truncated")}
                </Typography>
            ) : null}
        </Box>
    )
}

const MessageRow: React.FC<{message: IFakeProviderMessage}> = ({message}) => {
    const {t, i18n} = useTranslation()
    const [expanded, setExpanded] = useState(false)
    const detailsId = useId()
    const detailsRef = useRef<HTMLDivElement>(null)
    const toggleRef = useRef<HTMLButtonElement>(null)
    useEffect(() => {
        if (expanded) detailsRef.current?.scrollIntoView?.({block: "nearest", inline: "start"})
    }, [expanded])
    const collapse = () => {
        setExpanded(false)
        toggleRef.current?.focus()
    }
    return (
        <>
            <TableRow>
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
                <TableCell sx={{overflowWrap: "anywhere"}}>{message.recipient}</TableCell>
                <TableCell
                    sx={{
                        maxWidth: 420,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                    }}
                >
                    <Box sx={{overflow: "hidden", textOverflow: "ellipsis"}}>
                        {contentPreview(message, t)}
                    </Box>
                    <Link
                        component="button"
                        type="button"
                        underline="hover"
                        ref={toggleRef}
                        onClick={() => (expanded ? collapse() : setExpanded(true))}
                        aria-expanded={expanded}
                        aria-controls={detailsId}
                        sx={{display: "block", mt: 0.5}}
                    >
                        {t(expanded ? "sentMessages.showLess" : "sentMessages.showMore")}
                    </Link>
                </TableCell>
                <TableCell>
                    <ReportResult report={message.report} />
                </TableCell>
            </TableRow>
            {expanded ? (
                <TableRow>
                    <TableCell colSpan={6} sx={{p: 0}}>
                        <Box
                            id={detailsId}
                            ref={detailsRef}
                            role="group"
                            aria-label={t("sentMessages.column.message")}
                            sx={{maxWidth: "calc(100vw - 64px)", minWidth: 0}}
                        >
                            <Link
                                component="button"
                                type="button"
                                underline="hover"
                                onClick={collapse}
                                sx={{m: 2, mb: 0}}
                            >
                                {t("sentMessages.showLess")}
                            </Link>
                            <MessageDetails message={message} />
                        </Box>
                    </TableCell>
                </TableRow>
            ) : null}
        </>
    )
}

/** Every message, newest first: Time | Channel | From | To | Message, and its report. */
export const AllMessagesTable: React.FC<{messages: IFakeProviderMessage[]}> = ({messages}) => {
    const {t} = useTranslation()
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
                        <MessageRow key={message.id} message={message} />
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
