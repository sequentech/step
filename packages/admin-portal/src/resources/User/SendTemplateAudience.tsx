// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect} from "react"
import {useQuery} from "@apollo/client"
import {Table, TableBody, TableCell, TableHead, TableRow, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {ChannelLabel} from "@sequentech/ui-essentials"
import {
    ESendsFrom,
    GET_MESSAGING_AUDIENCE_PREVIEW,
    IAudienceChannel,
    IGetMessagingAudiencePreviewResult,
} from "@/queries/GetMessagingAudiencePreview"
import {roleContext, useMessagingReadRole} from "@/hooks/useMessagingReadRole"
import {RECOUNT_POLL_MS, useRecountMessagingVoters} from "@/hooks/useMessagingVoterCounts"
import {ERecountOutcome} from "@/queries/GetMessagingVoterCounts"
import {MessagingVoterCountStatus} from "@/components/MessagingVoterCountStatus"
import {useFormatCount} from "@/services/messagingFormat"
import {EMessageChannel} from "@/types/messaging"

export interface SendTemplateAudienceProps {
    electionEventId: string
    /** As the send names it: `ALL_USERS`, `VOTED`, `NOT_VOTED` or `SELECTED`. */
    audienceSelection: string
    voterIds?: Array<string | number>
    /** The event's notice channels, shown with their sender until the counts arrive. */
    channels: EMessageChannel[]
    /** Who sends `channel`'s notices, as the configuration says. */
    sendsFrom: (channel: EMessageChannel) => string
}

/**
 * Send Notification's "Each voter's channel" (Channel | Voters | Sends from):
 * per channel, how many of the send's voters it reaches and who it sends
 * from, counted on the server the way the send reads voters
 * (`messaging_audience_preview`). Voters whose Post doesn't offer their
 * channel for notices, and voters with no contact channel, are counted and
 * said not to get it: nothing is sent to them on another channel. A channel
 * none of whose voters may get it reads "Not sent". The whole event and the
 * voted filters show the event's last count, with when and Recount; selected
 * voters are counted when asked.
 */
export const SendTemplateAudience: React.FC<SendTemplateAudienceProps> = ({
    electionEventId,
    audienceSelection,
    voterIds,
    channels,
    sendsFrom,
}) => {
    const {t} = useTranslation()
    const formatCount = useFormatCount()
    const readRole = useMessagingReadRole()
    const ids = (voterIds ?? []).map(String)
    const selected = audienceSelection === "SELECTED"
    // Selected voters with none selected: the send itself is refused.
    const noneSelected = audienceSelection === "SELECTED" && ids.length === 0
    const {data, loading, error, refetch, startPolling, stopPolling} =
        useQuery<IGetMessagingAudiencePreviewResult>(GET_MESSAGING_AUDIENCE_PREVIEW, {
            variables: {
                electionEventId,
                audienceSelection,
                voterIds: selected ? ids : undefined,
            },
            skip: !readRole || noneSelected,
            context: roleContext(readRole),
        })
    const answer = data?.messaging_audience_preview
    // Counting the selected voters took too long: nothing counted to show.
    const selectedTimedOut =
        selected &&
        answer?.status?.outcome === ERecountOutcome.TIMED_OUT &&
        !answer.status.computed_at
    const preview = selectedTimedOut ? undefined : answer
    const rows: IAudienceChannel[] = preview?.channels ?? []
    const pending = !selected && !!preview?.status?.pending
    useEffect(() => {
        if (!pending) return undefined
        startPolling(RECOUNT_POLL_MS)
        return () => stopPolling()
    }, [pending, startPolling, stopPolling])
    const {recount, recounting} = useRecountMessagingVoters(electionEventId, refetch)
    const channelName = (channel: EMessageChannel) => t(`messaging.channel.${channel}`)
    // A channel none of whose voters may get the notice: nothing is sent on it.
    const notSent = (row: IAudienceChannel) => !row.channel || !row.sends_from
    const sender = (row: IAudienceChannel): string => {
        if (notSent(row)) return t("messaging.audience.notSent")
        switch (row.sends_from) {
            case ESendsFrom.ACCOUNT:
                return row.account_name ?? t("messaging.audience.deletedAccount")
            case ESendsFrom.DEPLOYMENT:
                return t("messaging.send.environment")
            case ESendsFrom.ACCOUNT_MISSING:
                return t("messaging.audience.deletedAccount")
            default:
                return t("messaging.send.noAccount")
        }
    }

    return (
        <>
            <Table size="small" aria-label={String(t("messaging.send.eachVoter"))}>
                <TableHead>
                    <TableRow>
                        <TableCell>{t("messaging.send.channelColumn")}</TableCell>
                        <TableCell align="right">{t("messaging.send.voters")}</TableCell>
                        <TableCell>{t("messaging.send.sendsFrom")}</TableCell>
                    </TableRow>
                </TableHead>
                <TableBody>
                    {preview
                        ? rows.map((row) => (
                              <TableRow
                                  key={row.channel ?? "NONE"}
                                  data-channel={row.channel ?? "NONE"}
                              >
                                  <TableCell>
                                      {row.channel ? (
                                          <ChannelLabel
                                              channel={row.channel}
                                              label={channelName(row.channel)}
                                          />
                                      ) : (
                                          t("messaging.reason.NO_CONTACT_CHANNEL")
                                      )}
                                  </TableCell>
                                  <TableCell align="right">
                                      {formatCount(row.voters)}
                                      {row.not_offered > 0 && !notSent(row) ? (
                                          <Typography
                                              variant="caption"
                                              component="div"
                                              color="text.secondary"
                                          >
                                              {t("messaging.audience.notOffered", {
                                                  notOffered: formatCount(row.not_offered),
                                                  reason: t(
                                                      "messaging.reason.CHANNEL_NOT_OFFERED_FOR_NOTICES"
                                                  ),
                                              })}
                                          </Typography>
                                      ) : null}
                                  </TableCell>
                                  <TableCell>
                                      {sender(row)}
                                      {row.channel &&
                                      (row.sends_from === ESendsFrom.NONE ||
                                          row.sends_from === ESendsFrom.ACCOUNT_MISSING) ? (
                                          <Typography
                                              variant="caption"
                                              component="div"
                                              color="text.secondary"
                                          >
                                              {t("messaging.audience.noAccountHelp", {
                                                  reason: t(
                                                      "messaging.reason.NO_ACCOUNT_FOR_CHANNEL"
                                                  ),
                                              })}
                                          </Typography>
                                      ) : null}
                                      {row.channel && notSent(row) ? (
                                          <Typography
                                              variant="caption"
                                              component="div"
                                              color="text.secondary"
                                          >
                                              {t("messaging.audience.notOfferedRow", {
                                                  reason: t(
                                                      "messaging.reason.CHANNEL_NOT_OFFERED_FOR_NOTICES"
                                                  ),
                                              })}
                                          </Typography>
                                      ) : null}
                                      {!row.channel ? (
                                          <Typography
                                              variant="caption"
                                              component="div"
                                              color="text.secondary"
                                          >
                                              {t("messaging.audience.noChannelHelp", {
                                                  reason: t("messaging.reason.NO_CONTACT_CHANNEL"),
                                              })}
                                          </Typography>
                                      ) : null}
                                  </TableCell>
                              </TableRow>
                          ))
                        : channels.map((channel) => (
                              <TableRow key={channel} data-channel={channel}>
                                  <TableCell>
                                      <ChannelLabel
                                          channel={channel}
                                          label={channelName(channel)}
                                      />
                                  </TableCell>
                                  <TableCell align="right">–</TableCell>
                                  <TableCell>{sendsFrom(channel)}</TableCell>
                              </TableRow>
                          ))}
                </TableBody>
            </Table>
            {preview ? (
                <>
                    <Typography variant="caption" color="text.secondary" component="p">
                        {t("messaging.audience.total", {total: formatCount(preview.total)})}
                    </Typography>
                    <Typography variant="body2" component="p">
                        {t("messaging.audience.willSend", {
                            willSend: formatCount(preview.will_send),
                        })}
                    </Typography>
                    {selected ? null : (
                        <MessagingVoterCountStatus
                            status={preview.status}
                            onRecount={recount}
                            recounting={recounting}
                        />
                    )}
                </>
            ) : null}
            {selectedTimedOut && !loading ? (
                <Typography variant="caption" role="status" color="text.secondary" component="p">
                    {t("messaging.audience.selectedTimedOut")}
                </Typography>
            ) : null}
            {error && !loading ? (
                <Typography variant="caption" role="status" color="text.secondary" component="p">
                    {t("messaging.audience.loadError")}
                </Typography>
            ) : null}
        </>
    )
}
