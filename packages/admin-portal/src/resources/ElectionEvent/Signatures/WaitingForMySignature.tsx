// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useId, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Badge,
    Box,
    Button,
    Chip,
    CircularProgress,
    Drawer,
    IconButton,
    List,
    ListItem,
    ListItemButton,
    ListItemText,
    Stack,
    Typography,
} from "@mui/material"
import CloseIcon from "@mui/icons-material/Close"
import DrawOutlinedIcon from "@mui/icons-material/DrawOutlined"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useOptionalSigningRequest} from "@/hooks/useSignedAction"
import type {ISigningRequestContext} from "@/components/signing/SigningProvider"
import {useSigningFormat} from "@/components/signing/format"
import {SigningRequestStatus, type IWaitingSigningRequest} from "@/lib/signing/types"
import {requestStatusKey} from "@/lib/signing/status"
import {
    useListedSignPermissions,
    useScopeNames,
    useSigningEventInfo,
    useWaitingSigningRequests,
} from "./useSigningSettings"

const isOpen = (request: IWaitingSigningRequest, now: Date) =>
    !request.expires_at || Date.parse(request.expires_at) > now.getTime()

/**
 * "Waiting for my signature": the event's requests waiting for signatures of
 * the actions the signed-in user may sign, in their Posts, without the
 * Signatures tab. A row opens the signing request panel, with the signing
 * dialog when they haven't signed it yet. Nothing shows without a sign
 * permission whose requests Hasura lists (a trustee signs in their ceremony).
 */
export const WaitingForMySignature: React.FC<{electionEventId: string}> = ({electionEventId}) => {
    const roles = useListedSignPermissions()
    const signing = useOptionalSigningRequest()
    // Nothing is read for someone who signs nothing listed here.
    return roles.length && signing ? (
        <WaitingList electionEventId={electionEventId} signing={signing} />
    ) : null
}

const WaitingList: React.FC<{electionEventId: string; signing: ISigningRequestContext}> = ({
    electionEventId,
    signing,
}) => {
    const {t} = useTranslation()
    const {userId} = useContext(AuthContext)
    const {requests, error, reload} = useWaitingSigningRequests(electionEventId)
    const {timeZone} = useSigningEventInfo(electionEventId)
    const format = useSigningFormat(timeZone)
    const {postName, countryName} = useScopeNames(electionEventId, true)
    const [open, setOpen] = useState(false)
    const titleId = useId()

    const now = new Date()
    const shown = (requests ?? []).filter((request) => isOpen(request, now))
    const signedByMe = (request: IWaitingSigningRequest) =>
        request.approvals.some((approval) => approval.user_id === userId)
    const toSign = shown.filter((request) => !signedByMe(request)).length
    const title = (request: IWaitingSigningRequest) =>
        [
            t(`signing.actions.${request.action}.short`),
            request.election_id ? postName(request.election_id) : null,
            request.area_id ? countryName(request.area_id) : null,
        ]
            .filter(Boolean)
            .join(" · ")

    const openRequest = (request: IWaitingSigningRequest) => {
        setOpen(false)
        signing.open(request.id, {
            sign: !signedByMe(request),
            eventId: electionEventId,
            onChange: reload,
        })
    }

    return (
        <>
            <Button
                variant="outlined"
                size="small"
                onClick={() => {
                    reload()
                    setOpen(true)
                }}
                startIcon={
                    <Badge badgeContent={toSign} color="warning">
                        <DrawOutlinedIcon />
                    </Badge>
                }
                aria-label={t("signing.waiting.buttonCount", {count: toSign})}
            >
                {t("signing.waiting.title")}
            </Button>
            <Drawer
                anchor="right"
                open={open}
                onClose={() => setOpen(false)}
                slotProps={{
                    paper: {
                        "aria-labelledby": titleId,
                        "sx": {width: {xs: "100%", sm: 520}, p: {xs: 2, sm: 3}},
                    },
                }}
            >
                <Stack direction="row" sx={{alignItems: "flex-start", gap: 1}}>
                    <Box sx={{flex: 1, minWidth: 0}}>
                        <Typography id={titleId} variant="h5" component="h2">
                            {t("signing.waiting.title")}
                        </Typography>
                        <Typography color="text.secondary">{t("signing.waiting.intro")}</Typography>
                    </Box>
                    <IconButton
                        onClick={() => setOpen(false)}
                        aria-label={t("signing.waiting.close")}
                    >
                        <CloseIcon />
                    </IconButton>
                </Stack>
                <Box sx={{mt: 2}}>
                    {error ? (
                        <Alert severity="error">{t("signing.waiting.loadError")}</Alert>
                    ) : requests === null ? (
                        <CircularProgress aria-label={t("common.label.loadingData")} />
                    ) : shown.length === 0 ? (
                        <Typography>{t("signing.waiting.empty")}</Typography>
                    ) : (
                        <List aria-labelledby={titleId} disablePadding>
                            {shown.map((request) => (
                                <ListItem key={request.id} disablePadding>
                                    <ListItemButton
                                        divider
                                        onClick={() => openRequest(request)}
                                        data-request={request.id}
                                    >
                                        <ListItemText
                                            primary={title(request)}
                                            primaryTypographyProps={{sx: {fontWeight: 600}}}
                                            secondary={
                                                <>
                                                    <Box
                                                        component="span"
                                                        sx={{fontFamily: "monospace", mr: 1}}
                                                    >
                                                        {request.code}
                                                    </Box>
                                                    {t("signing.requests.statusCount", {
                                                        status: t(
                                                            requestStatusKey(
                                                                SigningRequestStatus.Waiting
                                                            )
                                                        ),
                                                        count: request.approvals.length,
                                                        total: request.required,
                                                    })}
                                                    {request.expires_at ? (
                                                        <Box
                                                            component="span"
                                                            sx={{display: "block"}}
                                                        >
                                                            {t("signing.requests.expires", {
                                                                time: format.dateTime(
                                                                    request.expires_at
                                                                ),
                                                            })}
                                                        </Box>
                                                    ) : null}
                                                </>
                                            }
                                        />
                                        {signedByMe(request) ? (
                                            <Chip
                                                size="small"
                                                color="success"
                                                variant="outlined"
                                                label={t("signing.waiting.signedByYou")}
                                            />
                                        ) : null}
                                    </ListItemButton>
                                </ListItem>
                            ))}
                        </List>
                    )}
                </Box>
            </Drawer>
        </>
    )
}
