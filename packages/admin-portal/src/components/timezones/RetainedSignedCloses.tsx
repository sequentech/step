// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {Alert, Stack, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {IRetainedSignedClose} from "@/types/lifecycle"
import {useTimeZoneService} from "./timeZoneService"

/** Display the signed instant, never the editable row's replacement wall time. */
export const RetainedSignedCloseNotice: React.FC<{
    close: IRetainedSignedClose
    zone: string
    electionName: string
    now?: number
}> = ({close, zone, electionName, now = Date.now()}) => {
    const {t} = useTranslation()
    const service = useTimeZoneService()
    const deadline = service.formatDateTimeZone(close.scheduled_at, zone, service.text)
    const processed = !!close.fired_at
    const reached = Date.parse(close.scheduled_at) <= now
    return (
        <Alert
            severity={processed ? "info" : "warning"}
            data-testid={`retained-close-${close.scheduled_event_id}-${close.election_id}`}
        >
            <Typography component="h3" variant="body2" sx={{fontWeight: 600}}>
                {close.fired_at
                    ? t("lifecycle.signedClose.processed", {
                          time: service.formatDateTimeZone(close.fired_at, zone, service.text),
                      })
                    : t("lifecycle.signedClose.title")}
            </Typography>
            <Typography variant="body2">
                {t("lifecycle.signedClose.deadline", {
                    election: electionName,
                    time: deadline,
                    code: close.authorized_by.code,
                })}
            </Typography>
            <Typography variant="body2">
                {t(
                    processed ? "lifecycle.signedClose.result" : "lifecycle.signedClose.explanation"
                )}
            </Typography>
            {!processed && reached ? (
                <Typography variant="body2">{t("lifecycle.signedClose.reached")}</Typography>
            ) : null}
            {!processed ? (
                <Typography variant="body2">
                    {t("lifecycle.signedClose.channels", {
                        channels: close.channels
                            .map((channel) => t(`common.channel.${channel.toLowerCase()}`))
                            .join(", "),
                    })}
                </Typography>
            ) : null}
            {close.authorized_by.signers.length ? (
                <Typography variant="body2">
                    {t("lifecycle.publish.authorizedBy")}: {close.authorized_by.signers.join(", ")}
                </Typography>
            ) : null}
        </Alert>
    )
}

export const RetainedSignedCloses: React.FC<{
    closes: ReadonlyArray<IRetainedSignedClose>
    zoneOf: (electionId: string) => string
    nameOf: (electionId: string) => string
    unavailable?: boolean
}> = ({closes, zoneOf, nameOf, unavailable = false}) => {
    const {t} = useTranslation()
    const [now, setNow] = useState(Date.now)
    useEffect(() => {
        if (!closes.some((close) => !close.fired_at)) return
        const timer = setInterval(() => setNow(Date.now()), 15_000)
        return () => clearInterval(timer)
    }, [closes])
    if (!closes.length && !unavailable) return null
    return (
        <Stack spacing={1} data-testid="retained-signed-closes">
            {unavailable ? (
                <Alert severity="warning">{t("lifecycle.signedClose.unavailable")}</Alert>
            ) : null}
            {[...closes]
                .sort((a, b) => Date.parse(a.scheduled_at) - Date.parse(b.scheduled_at))
                .map((close) => (
                    <RetainedSignedCloseNotice
                        key={`${close.scheduled_event_id}:${close.election_id}:${close.fingerprint}`}
                        close={close}
                        zone={zoneOf(close.election_id)}
                        electionName={nameOf(close.election_id)}
                        now={now}
                    />
                ))}
        </Stack>
    )
}
