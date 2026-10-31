// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    CircularProgress,
    List,
    ListItem,
    ListItemButton,
    ListItemText,
    Paper,
    ToggleButton,
    ToggleButtonGroup,
    Typography,
} from "@mui/material"
import {ReportResult} from "./ReportResult"
import {emailSrcDoc} from "./emailFrame"
import {formatTime, hasHtml, newestFirst} from "./messageContent"
import {useFakeEmailHtml} from "./useSentMessages"
import {IFakeProviderMessage} from "./types"

type EBodyView = "HTML" | "TEXT"

export const EmailReadingPane: React.FC<{message: IFakeProviderMessage}> = ({message}) => {
    const {t, i18n} = useTranslation()
    const {content} = message
    const withHtml = hasHtml(message)
    // The HTML isn't in the list: it is read for this email only (R6-03).
    const {html, loading, failed} = useFakeEmailHtml(message.id, withHtml)
    const [view, setView] = useState<EBodyView>(withHtml ? "HTML" : "TEXT")
    const shown: EBodyView = view === "HTML" && withHtml ? "HTML" : "TEXT"
    const srcDoc = useMemo(() => (html ? emailSrcDoc(html) : null), [html])
    return (
        <Box
            component="article"
            aria-label={t("sentMessages.email.reading")}
            sx={{display: "flex", flexDirection: "column", gap: 1, p: 2, minWidth: 0}}
        >
            <Typography variant="h6" component="h3" sx={{overflowWrap: "anywhere"}}>
                {content.subject || t("sentMessages.email.noSubject")}
            </Typography>
            <Typography variant="body2">
                {t("sentMessages.email.from")}: {message.sender}
            </Typography>
            <Typography variant="body2">
                {t("sentMessages.email.to")}: {message.recipient}
            </Typography>
            <Box sx={{display: "flex", gap: 1, alignItems: "center", flexWrap: "wrap"}}>
                <Typography variant="caption" color="text.secondary">
                    {formatTime(message.received_at, i18n.language)}
                </Typography>
                <ReportResult report={message.report} />
            </Box>
            {content.truncated ? (
                <Typography variant="caption" color="text.secondary">
                    {t("sentMessages.content.truncated")}
                </Typography>
            ) : null}
            {withHtml && content.text ? (
                <ToggleButtonGroup
                    size="small"
                    exclusive
                    value={shown}
                    onChange={(_event, value: EBodyView | null) => value && setView(value)}
                    aria-label={t("sentMessages.email.view")}
                >
                    <ToggleButton value="HTML">{t("sentMessages.email.html")}</ToggleButton>
                    <ToggleButton value="TEXT">{t("sentMessages.email.text")}</ToggleButton>
                </ToggleButtonGroup>
            ) : null}
            {shown === "HTML" ? (
                loading ? (
                    <CircularProgress size={24} aria-label={t("sentMessages.email.loading")} />
                ) : failed || !srcDoc ? (
                    <Alert severity="warning">{t("sentMessages.email.loadError")}</Alert>
                ) : (
                    // sandbox="": no scripts, forms, popups or same-origin access;
                    // the CSP meta first in srcDoc: nothing loads from outside (R6-06).
                    <Box
                        component="iframe"
                        sandbox=""
                        srcDoc={srcDoc}
                        title={t("sentMessages.email.htmlFrame")}
                        referrerPolicy="no-referrer"
                        sx={{
                            width: "100%",
                            height: 480,
                            border: 1,
                            borderColor: "divider",
                            bgcolor: "#fff",
                        }}
                    />
                )
            ) : content.text ? (
                <Box
                    component="pre"
                    sx={{
                        whiteSpace: "pre-wrap",
                        overflowWrap: "anywhere",
                        fontFamily: "inherit",
                        m: 0,
                    }}
                >
                    {content.text}
                </Box>
            ) : (
                <Typography color="text.secondary">{t("sentMessages.email.noContent")}</Typography>
            )}
        </Box>
    )
}

/** The emails as a mail client shows them: an inbox list and a reading pane. */
export const EmailInbox: React.FC<{messages: IFakeProviderMessage[]}> = ({messages}) => {
    const {t, i18n} = useTranslation()
    const inbox = newestFirst(messages)
    const [selected, setSelected] = useState<string | null>(null)
    // The newest email is open until another is chosen.
    const open = inbox.find((message) => message.id === selected) ?? inbox[0]
    if (!open) {
        return (
            <Typography color="text.secondary">
                {t("sentMessages.emptyChannel", {channel: t("messaging.channel.EMAIL")})}
            </Typography>
        )
    }
    return (
        <Paper
            variant="outlined"
            sx={{
                display: "grid",
                gridTemplateColumns: {xs: "1fr", md: "minmax(240px, 1fr) 2fr"},
                minHeight: 480,
            }}
        >
            <List
                aria-label={t("sentMessages.email.inbox")}
                disablePadding
                sx={{
                    borderRight: {md: 1},
                    borderBottom: {xs: 1, md: 0},
                    borderColor: {xs: "divider", md: "divider"},
                    maxHeight: {xs: 280, md: 640},
                    overflowY: "auto",
                }}
            >
                {inbox.map((message) => (
                    <ListItem key={message.id} disablePadding divider>
                        <ListItemButton
                            selected={message.id === open.id}
                            onClick={() => setSelected(message.id)}
                        >
                            <ListItemText
                                primary={
                                    message.content.subject || t("sentMessages.email.noSubject")
                                }
                                secondary={`${message.recipient} · ${formatTime(
                                    message.received_at,
                                    i18n.language
                                )}`}
                                slotProps={{primary: {noWrap: true}, secondary: {noWrap: true}}}
                            />
                        </ListItemButton>
                    </ListItem>
                ))}
            </List>
            <EmailReadingPane key={open.id} message={open} />
        </Paper>
    )
}
