// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {EScheduledOutcomeKind} from "@sequentech/ui-core"
import {ScheduledOutcome} from "@/components/timezones/ScheduledOutcome"
import type {IFiredPost} from "@/types/lifecycle"

export enum ETransition {
    OPENED = "opened",
    CLOSED = "closed",
}

/**
 * The Publish tab's card of a scheduled opening or closing that fired at the
 * election (design §5a): authorized by the signed configuration (its signers
 * under "Authorized by", not as closing signatures); a close without
 * signatures as a neutral card naming the close requests it cancelled; or a
 * refusal with its "Why?".
 */
export const ScheduledTransitionCard: React.FC<{
    transition: ETransition
    /** When it fired, as the card words it (in the election's zone). */
    time: string
    post: IFiredPost
    /** The election's zone, for times in the explanation. */
    zone: string
}> = ({transition, time, post, zone}) => {
    const {t, i18n} = useTranslation()
    const authorized = post.authorized_by
    const refused = post.outcome === EScheduledOutcomeKind.REFUSED
    const names = (list: Array<string>) =>
        new Intl.ListFormat(i18n.language, {type: "conjunction"}).format(list)
    const title = refused
        ? t(`lifecycle.publish.${transition}Refused`, {time})
        : post.nothing_to_change
          ? t(`lifecycle.publish.${transition}NothingToChange`, {time})
          : authorized
            ? t(`lifecycle.publish.${transition}Authorized`, {
                  time,
                  code: authorized.code,
                  names: names(authorized.signers),
              })
            : post.unsigned
              ? t("lifecycle.publish.closedUnsigned", {time})
              : t(`lifecycle.publish.${transition}NoSignaturesNeeded`, {time})
    return (
        <Box
            data-testid={`${transition}-on-schedule`}
            sx={{
                width: "100%",
                border: 1,
                borderColor: refused ? "warning.light" : authorized ? "success.light" : "divider",
                borderRadius: 1,
                bgcolor: authorized ? "rgba(46, 125, 50, 0.06)" : "action.hover",
                p: 2,
            }}
        >
            <Typography component="h3" sx={{fontWeight: 600, mb: 1}}>
                {title}
            </Typography>
            {authorized?.signers.length && !refused ? (
                <>
                    <Typography component="h4" variant="body2" sx={{fontWeight: 600}}>
                        {t("lifecycle.publish.authorizedBy")}
                    </Typography>
                    <Box component="ul" sx={{m: 0, pl: 2}}>
                        {authorized.signers.map((signer) => (
                            <li key={signer}>
                                <Typography variant="body2">{signer}</Typography>
                            </li>
                        ))}
                    </Box>
                </>
            ) : null}
            {(post.cancelled ?? []).map((request) => (
                <Typography
                    key={request.request_id}
                    variant="body2"
                    color="text.secondary"
                    sx={{mt: 1}}
                >
                    {t("lifecycle.publish.cancelledRequest", {
                        code: request.code,
                        n: request.signatures,
                        k: request.required,
                    })}
                </Typography>
            ))}
            {refused && post.explanation ? (
                <Box sx={{mt: 1}}>
                    <ScheduledOutcome explanation={post.explanation} zone={zone} />
                </Box>
            ) : null}
        </Box>
    )
}
