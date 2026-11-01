// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The check mark that shows a ballot's status, and the dialog it opens: next
// to a Tracking ID on the review and confirmation screens, and as the "Ballot
// cast" label in the ballot list (design boards Review, Confirmation and
// BallotList).

import React, {useCallback, useContext, useEffect, useState} from "react"
import {ButtonBase, Tooltip} from "@mui/material"
import type {SxProps, Theme} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {useNavigate} from "react-router-dom"
import {AuthContext} from "../../providers/AuthContextProvider"
import {
    ballotStatusOf,
    EBallotStatus,
    EBallotTag,
    EStatusStep,
    EStepState,
    STATUS_STEPS,
    statusSteps,
} from "../../services/vitrine/ballotStatus"
import type {IReceiptView} from "../../services/vitrine/castReceipt"
import {BallotStatusDialog} from "./BallotStatusDialog"
import {BallotStatusIcon, CHECK_SIZES} from "./BallotStatusIcon"

const NAVY = "#0F054C"
const TOOLTIP_GAP_PX = 8

type TTimes = Partial<Record<EStatusStep, number>>

/** When this page saw each step done. */
const useStepTimes = (steps: Record<EStatusStep, EStepState>): TTimes => {
    const [times, setTimes] = useState<TTimes>({})
    const done = STATUS_STEPS.filter((step) => steps[step] === EStepState.DONE).join(",")
    useEffect(() => {
        const seen = done ? (done.split(",") as EStatusStep[]) : []
        const now = Date.now()
        setTimes((current) => {
            const fresh = seen.filter((step) => current[step] === undefined)
            return fresh.length === 0
                ? current
                : {...current, ...Object.fromEntries(fresh.map((step) => [step, now]))}
        })
    }, [done])
    return times
}

/** The ballot tracker link: a new tab, or in this one on a kiosk, which keeps
 * its session in memory. */
export const useTrackerLink = (
    url: string | undefined
): React.AnchorHTMLAttributes<HTMLAnchorElement> | undefined => {
    const {isKiosk} = useContext(AuthContext)
    const navigate = useNavigate()
    const kiosk = isKiosk()
    const onClick = useCallback(
        (event: React.MouseEvent<HTMLAnchorElement>) => {
            if (!kiosk || !url) return
            event.preventDefault()
            const {pathname, search} = new URL(url, window.location.origin)
            navigate({pathname, search})
        },
        [kiosk, url, navigate]
    )
    if (!url) return undefined
    return kiosk ? {href: url, onClick} : {href: url, target: "_blank", rel: "noopener noreferrer"}
}

const focusSx = {
    "&.Mui-focusVisible": {outline: `2px solid ${NAVY}`, outlineOffset: "1px", boxShadow: "none"},
} as const

interface ITriggerProps {
    status: EBallotStatus
    onClick: () => void
    surface?: string
    strokeWidth?: number
    sx?: SxProps<Theme>
}

const useStateLabels = (status: EBallotStatus) => {
    const {t} = useTranslation()
    const state = t(`vitrine.status.state.${status}`)
    return {
        tooltip: t("vitrine.status.tooltip", {state}),
        state: state.replace("…", "").toLowerCase(),
    }
}

/** The check mark that opens the dialog: a 34 px target, with the state in its
 * tooltip and its accessible name. */
export const BallotStatusButton: React.FC<ITriggerProps> = ({
    status,
    onClick,
    surface,
    strokeWidth,
    sx,
}) => {
    const {t} = useTranslation()
    const {tooltip, state} = useStateLabels(status)
    return (
        <Tooltip
            className="ballot-status-tooltip"
            title={tooltip}
            placement="top"
            slotProps={{
                popper: {modifiers: [{name: "offset", options: {offset: [0, -TOOLTIP_GAP_PX]}}]},
            }}
        >
            <ButtonBase
                className="ballot-status-button"
                data-status={status}
                aria-label={t("vitrine.status.aria", {state})}
                aria-haspopup="dialog"
                onClick={onClick}
                sx={[
                    {
                        "width": 34,
                        "height": 34,
                        "borderRadius": "50%",
                        "flex": "none",
                        "&:hover": {bgcolor: "rgba(0, 0, 0, 0.04)"},
                    },
                    focusSx,
                    ...(Array.isArray(sx) ? sx : [sx]),
                ]}
            >
                <BallotStatusIcon
                    status={status}
                    size={CHECK_SIZES.ID}
                    surface={surface}
                    strokeWidth={strokeWidth}
                />
            </ButtonBase>
        </Tooltip>
    )
}

export interface IBallotStatusProps {
    /** Given when the ballot isn't cast yet; otherwise it comes from the receipt. */
    status?: EBallotStatus
    view?: IReceiptView
    /** The dialog's tag, when the status doesn't tell it. */
    tag?: EBallotTag
    ballotId: string
    electionName?: string
    castAt?: string
    trackerUrl?: string
    /** Audits the ballot, at review: the dialog closes and this runs. */
    onAudit?: () => void
    /** The background behind the icon, for the ring around a badge. */
    surface?: string
    strokeWidth?: number
    sx?: SxProps<Theme>
}

/** The status check mark of a ballot, and its dialog. */
export const BallotStatus: React.FC<IBallotStatusProps> = ({
    status: given,
    view,
    tag,
    ballotId,
    electionName,
    castAt,
    trackerUrl,
    onAudit,
    surface,
    strokeWidth,
    sx,
}) => {
    const [open, setOpen] = useState(false)
    const status = given ?? ballotStatusOf(view)
    const times = useStepTimes(statusSteps(status, view?.result))
    const tracker = useTrackerLink(trackerUrl)
    return (
        <>
            <BallotStatusButton
                status={status}
                onClick={() => setOpen(true)}
                surface={surface}
                strokeWidth={strokeWidth}
                sx={sx}
            />
            <BallotStatusDialog
                open={open}
                onClose={() => setOpen(false)}
                status={status}
                view={view}
                tag={tag}
                ballotId={ballotId}
                electionName={electionName}
                castAt={castAt}
                times={times}
                tracker={tracker}
                onAudit={
                    onAudit
                        ? () => {
                              setOpen(false)
                              onAudit()
                          }
                        : undefined
                }
            />
        </>
    )
}
