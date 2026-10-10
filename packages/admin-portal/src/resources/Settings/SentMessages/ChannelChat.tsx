// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Avatar,
    Box,
    Button,
    IconButton,
    List,
    ListItem,
    ListItemAvatar,
    ListItemButton,
    ListItemText,
    Paper,
    TextField,
    Typography,
} from "@mui/material"
import {blue, deepPurple, green, grey, lightGreen} from "@mui/material/colors"
import ArrowBackIcon from "@mui/icons-material/ArrowBack"
import ContentCopyIcon from "@mui/icons-material/ContentCopy"
import PersonOutlineIcon from "@mui/icons-material/PersonOutline"
import {ChannelIcon} from "@sequentech/ui-essentials"
import {EMessageChannel} from "@/types/messaging"
import {ReportResult} from "./ReportResult"
import {
    IConversation,
    contentPreview,
    conversations,
    fakeReasonText,
    formatTime,
    parameterText,
    templateLabel,
} from "./messageContent"
import {IFakeGetStartedAnswer, IFakeMessageContent, IFakeProviderMessage} from "./types"

/** The channels shown as a messaging app on a phone. */
export type ChatChannel = Exclude<EMessageChannel, EMessageChannel.EMAIL>

interface IChatTheme {
    header: string
    headerText: string
    screen: string
    bubble: string
    bubbleText: string
}

// Our own colours, after each app's look; never the apps' logos.
const THEMES: Record<ChatChannel, IChatTheme> = {
    [EMessageChannel.WHATSAPP]: {
        header: green[800],
        headerText: "#fff",
        screen: grey[100],
        bubble: lightGreen[100],
        bubbleText: grey[900],
    },
    [EMessageChannel.VIBER]: {
        header: deepPurple[600],
        headerText: "#fff",
        screen: deepPurple[50],
        bubble: deepPurple[100],
        bubbleText: grey[900],
    },
    [EMessageChannel.MESSENGER]: {
        header: blue[700],
        headerText: "#fff",
        screen: "#fff",
        bubble: blue[800],
        bubbleText: "#fff",
    },
    [EMessageChannel.SMS]: {
        header: grey[200],
        headerText: grey[900],
        screen: "#fff",
        bubble: grey[200],
        bubbleText: grey[900],
    },
}

const PhoneFrame: React.FC<React.PropsWithChildren<{label: string; theme: IChatTheme}>> = ({
    label,
    theme,
    children,
}) => (
    <Box
        component="section"
        aria-label={label}
        sx={{
            // Full width on phones; a phone of its own on larger screens.
            width: {xs: "100%", sm: 380},
            height: {xs: "auto", sm: 680},
            minHeight: {xs: 480, sm: 0},
            flexShrink: 0,
            border: {xs: `1px solid ${grey[300]}`, sm: `10px solid ${grey[900]}`},
            borderRadius: {xs: 1, sm: "36px"},
            overflow: "hidden",
            display: "flex",
            flexDirection: "column",
            bgcolor: theme.screen,
        }}
    >
        {children}
    </Box>
)

const CopyCodeButton: React.FC<{code: string}> = ({code}) => {
    const {t} = useTranslation()
    const [copied, setCopied] = useState<"IDLE" | "COPIED" | "FAILED">("IDLE")
    const copy = async () => {
        try {
            await navigator.clipboard.writeText(code)
            setCopied("COPIED")
        } catch {
            setCopied("FAILED")
        }
    }
    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 0.5}}>
            <Button
                size="small"
                variant="outlined"
                color="inherit"
                startIcon={<ContentCopyIcon />}
                onClick={copy}
                sx={{bgcolor: "#fff", textTransform: "none"}}
            >
                {t("sentMessages.content.copyCode")}
            </Button>
            {copied === "IDLE" ? null : (
                <Typography variant="caption" role="status">
                    {copied === "COPIED"
                        ? t("sentMessages.content.copied")
                        : t("sentMessages.content.copyFailed")}
                </Typography>
            )}
        </Box>
    )
}

const TemplateBody: React.FC<{content: IFakeMessageContent; channel: ChatChannel}> = ({
    content,
    channel,
}) => {
    const {t} = useTranslation()
    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 0.5}}>
            <Typography variant="caption" sx={{opacity: 0.8}}>
                {templateLabel(content, t)}
            </Typography>
            {content.code ? (
                <>
                    <Typography variant="caption">{t("sentMessages.content.code")}</Typography>
                    <Typography
                        sx={{fontFamily: "monospace", fontSize: "1.5rem", letterSpacing: "0.15em"}}
                    >
                        {content.code}
                    </Typography>
                    {channel === EMessageChannel.WHATSAPP ? (
                        <CopyCodeButton code={content.code} />
                    ) : null}
                </>
            ) : (
                (content.parameters ?? []).map((parameter, index) => (
                    <Typography key={index} variant="body2" sx={{whiteSpace: "pre-wrap"}}>
                        {parameterText(parameter)}
                    </Typography>
                ))
            )}
        </Box>
    )
}

const ChatBubble: React.FC<{
    message: IFakeProviderMessage
    channel: ChatChannel
    theme: IChatTheme
}> = ({message, channel, theme}) => {
    const {t, i18n} = useTranslation()
    const {content} = message
    return (
        <Box
            component="li"
            sx={{display: "flex", flexDirection: "column", alignItems: "flex-end", gap: 0.25}}
        >
            <Box
                sx={{
                    maxWidth: "85%",
                    bgcolor: theme.bubble,
                    color: theme.bubbleText,
                    borderRadius: channel === EMessageChannel.MESSENGER ? "18px" : "10px",
                    px: 1.5,
                    py: 1,
                    overflowWrap: "anywhere",
                }}
            >
                {content.template_name ? (
                    <TemplateBody content={content} channel={channel} />
                ) : (
                    <Typography variant="body2" sx={{whiteSpace: "pre-wrap"}}>
                        {contentPreview(message, t)}
                    </Typography>
                )}
            </Box>
            {content.truncated ? (
                <Typography variant="caption" color="text.secondary">
                    {t("sentMessages.content.truncated")}
                </Typography>
            ) : null}
            <Box sx={{display: "flex", gap: 1, alignItems: "center"}}>
                <Typography variant="caption" color="text.secondary">
                    {formatTime(message.received_at, i18n.language)}
                </Typography>
                <ReportResult report={message.report} />
            </Box>
        </Box>
    )
}

const ChatHeader: React.FC<React.PropsWithChildren<{theme: IChatTheme; onBack?: () => void}>> = ({
    theme,
    onBack,
    children,
}) => {
    const {t} = useTranslation()
    return (
        <Box
            sx={{
                display: "flex",
                alignItems: "center",
                gap: 1,
                px: 1.5,
                py: 1,
                minHeight: 56,
                bgcolor: theme.header,
                color: theme.headerText,
            }}
        >
            {onBack ? (
                <IconButton
                    aria-label={t("sentMessages.chat.back")}
                    onClick={onBack}
                    sx={{color: "inherit"}}
                    size="small"
                >
                    <ArrowBackIcon />
                </IconButton>
            ) : null}
            {children}
        </Box>
    )
}

const ConversationList: React.FC<{
    chats: IConversation[]
    onOpen: (key: string) => void
}> = ({chats, onOpen}) => {
    const {t, i18n} = useTranslation()
    return (
        <List disablePadding sx={{overflowY: "auto", flex: 1, bgcolor: "#fff"}}>
            {chats.map((chat) => (
                <ListItem key={chat.key} disablePadding divider>
                    <ListItemButton
                        aria-label={t("sentMessages.chat.open", {
                            recipient: chat.recipient,
                            sender: chat.sender,
                        })}
                        onClick={() => onOpen(chat.key)}
                    >
                        <ListItemAvatar>
                            <Avatar>
                                <PersonOutlineIcon />
                            </Avatar>
                        </ListItemAvatar>
                        <ListItemText
                            primary={chat.recipient}
                            secondary={
                                <>
                                    <Box component="span" sx={{display: "block"}}>
                                        {t("sentMessages.chat.from", {sender: chat.sender})}
                                    </Box>
                                    {contentPreview(chat.latest, t)}
                                </>
                            }
                            slotProps={{secondary: {noWrap: true}}}
                        />
                        <Typography
                            variant="caption"
                            color="text.secondary"
                            sx={{ml: 1, alignSelf: "flex-start", whiteSpace: "nowrap"}}
                        >
                            {formatTime(chat.latest.received_at, i18n.language)}
                        </Typography>
                    </ListItemButton>
                </ListItem>
            ))}
        </List>
    )
}

const GetStartedBox: React.FC<{onGetStarted: (link: string) => Promise<IFakeGetStartedAnswer>}> = ({
    onGetStarted,
}) => {
    const {t} = useTranslation()
    const [link, setLink] = useState("")
    const [busy, setBusy] = useState(false)
    const [answer, setAnswer] = useState<IFakeGetStartedAnswer | null>(null)
    const submit = async (event: React.FormEvent) => {
        event.preventDefault()
        setBusy(true)
        setAnswer(null)
        try {
            setAnswer(await onGetStarted(link.trim()))
        } finally {
            setBusy(false)
        }
    }
    const reason = fakeReasonText(t, answer?.reason)
    return (
        <Paper
            variant="outlined"
            component="form"
            aria-labelledby="sent-messages-get-started"
            onSubmit={submit}
            sx={{
                p: 2,
                display: "flex",
                flexDirection: "column",
                gap: 1.5,
                flex: 1,
                minWidth: 260,
                maxWidth: 480,
            }}
        >
            <Typography id="sent-messages-get-started" variant="subtitle1" component="h3">
                {t("sentMessages.getStarted.title")}
            </Typography>
            <Typography variant="body2" color="text.secondary">
                {t("sentMessages.getStarted.help")}
            </Typography>
            <TextField
                size="small"
                label={t("sentMessages.getStarted.link")}
                placeholder="https://m.me/…?ref=…"
                value={link}
                onChange={(event) => setLink(event.target.value)}
            />
            <Button
                type="submit"
                variant="contained"
                disabled={busy || !link.trim()}
                sx={{alignSelf: "flex-start"}}
            >
                {t("sentMessages.getStarted.submit")}
            </Button>
            {answer ? (
                <Alert
                    role="status"
                    severity={
                        answer.status === 200
                            ? "success"
                            : answer.status != null
                              ? "warning"
                              : "error"
                    }
                >
                    {answer.status === 200
                        ? t("sentMessages.getStarted.accepted")
                        : answer.status != null
                          ? t("sentMessages.getStarted.answered", {status: answer.status})
                          : t("sentMessages.getStarted.failed", {reason})}
                    {answer.status != null && answer.status !== 200 && answer.reason
                        ? ` ${reason}`
                        : null}
                </Alert>
            ) : null}
        </Paper>
    )
}

export interface IChannelChatProps {
    channel: ChatChannel
    /** This channel's messages. */
    messages: IFakeProviderMessage[]
    /** Messenger only: the Get Started box sends the pasted link here. */
    onGetStarted?: (link: string) => Promise<IFakeGetStartedAnswer>
}

/**
 * A channel's messages as its app shows them on a phone: the chats, one per
 * sender and recipient, and a chat's messages with each one's delivery report.
 */
export const ChannelChat: React.FC<IChannelChatProps> = ({channel, messages, onGetStarted}) => {
    const {t} = useTranslation()
    const theme = THEMES[channel]
    const channelName = t(`messaging.channel.${channel}`)
    const chats = conversations(messages)
    const [selected, setSelected] = useState<string | null>(null)
    // A chat that is gone (the fake's list restarted) closes.
    const open = chats.find((chat) => chat.key === selected)
    return (
        <Box sx={{display: "flex", flexWrap: "wrap", gap: 2, alignItems: "flex-start"}}>
            <PhoneFrame label={t("sentMessages.chat.phone", {channel: channelName})} theme={theme}>
                {open ? (
                    <>
                        <ChatHeader theme={theme} onBack={() => setSelected(null)}>
                            <Box sx={{minWidth: 0}}>
                                <Typography variant="subtitle2" component="h3" noWrap>
                                    {open.recipient}
                                </Typography>
                                <Typography variant="caption" noWrap component="p">
                                    {t("sentMessages.chat.from", {sender: open.sender})}
                                </Typography>
                            </Box>
                        </ChatHeader>
                        <Box
                            component="ol"
                            aria-label={t("sentMessages.chat.messages", {
                                recipient: open.recipient,
                            })}
                            sx={{
                                listStyle: "none",
                                m: 0,
                                p: 1.5,
                                display: "flex",
                                flexDirection: "column",
                                gap: 1.5,
                                overflowY: "auto",
                                flex: 1,
                            }}
                        >
                            {open.messages.map((message) => (
                                <ChatBubble
                                    key={message.id}
                                    message={message}
                                    channel={channel}
                                    theme={theme}
                                />
                            ))}
                        </Box>
                    </>
                ) : (
                    <>
                        <ChatHeader theme={theme}>
                            <ChannelIcon channel={channel} />
                            <Typography variant="subtitle1" component="h3">
                                {t("sentMessages.chat.chats", {channel: channelName})}
                            </Typography>
                        </ChatHeader>
                        {chats.length === 0 ? (
                            <Typography color="text.secondary" sx={{p: 2}}>
                                {t("sentMessages.emptyChannel", {channel: channelName})}
                            </Typography>
                        ) : (
                            <ConversationList chats={chats} onOpen={setSelected} />
                        )}
                    </>
                )}
            </PhoneFrame>
            {channel === EMessageChannel.MESSENGER && onGetStarted ? (
                <GetStartedBox onGetStarted={onGetStarted} />
            ) : null}
        </Box>
    )
}
