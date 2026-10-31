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

export enum EBallotStatusTrigger {
    /** The 34 px check mark next to a Tracking ID. */
    ICON = "icon",
    /** The "Ballot cast" label of the ballot list. */
    LABEL = "label",
}

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
const useTrackerLink = (
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

/** "Ballot cast" with the check mark: the whole label opens the dialog. */
export const BallotCastButton: React.FC<ITriggerProps> = ({status, onClick, surface}) => {
    const {t} = useTranslation()
    const {tooltip, state} = useStateLabels(status)
    return (
        <Tooltip
            className="ballot-status-tooltip"
            title={tooltip}
            placement="top-start"
            slotProps={{
                popper: {modifiers: [{name: "offset", options: {offset: [0, -TOOLTIP_GAP_PX]}}]},
            }}
        >
            <ButtonBase
                className="ballot-status-button ballot-cast-button"
                data-status={status}
                aria-label={t("vitrine.status.ariaCast", {state})}
                aria-haspopup="dialog"
                onClick={onClick}
                sx={[
                    {
                        "gap": "6px",
                        "m": "-5px 0 -5px -5px",
                        "p": "5px 8px 5px 5px",
                        "borderRadius": "4px",
                        "fontSize": "14px",
                        "lineHeight": "16px",
                        "color": "#047857",
                        "&:hover": {
                            "bgcolor": "rgba(0, 0, 0, 0.04)",
                            "& .ballot-cast-label": {
                                textDecoration: "underline",
                                textUnderlineOffset: "2px",
                            },
                        },
                    },
                    focusSx,
                ]}
            >
                <BallotStatusIcon status={status} size={CHECK_SIZES.LIST} surface={surface} />
                <span className="ballot-cast-label">{t("vitrine.status.ballotCast")}</span>
            </ButtonBase>
        </Tooltip>
    )
}

export interface IBallotStatusProps {
    trigger: EBallotStatusTrigger
    /** Given when the ballot isn't cast yet; otherwise it comes from the receipt. */
    status?: EBallotStatus
    view?: IReceiptView
    /** The dialog's tag, when the status doesn't tell it. */
    tag?: EBallotTag
    ballotId: string
    electionName?: string
    castAt?: string
    /** Whether this device keeps the receipt and checks it on each visit. */
    kept?: boolean
    trackerUrl?: string
    /** The background behind the icon, for the ring around a badge. */
    surface?: string
    strokeWidth?: number
    sx?: SxProps<Theme>
}

/** The status check mark of a ballot, and its dialog. */
export const BallotStatus: React.FC<IBallotStatusProps> = ({
    trigger,
    status: given,
    view,
    tag,
    ballotId,
    electionName,
    castAt,
    kept,
    trackerUrl,
    surface,
    strokeWidth,
    sx,
}) => {
    const [open, setOpen] = useState(false)
    const status = given ?? ballotStatusOf(view)
    const times = useStepTimes(statusSteps(status, view?.result))
    const tracker = useTrackerLink(trackerUrl)
    const Trigger = trigger === EBallotStatusTrigger.ICON ? BallotStatusButton : BallotCastButton
    return (
        <>
            <Trigger
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
                kept={kept}
                tracker={tracker}
            />
        </>
    )
}
