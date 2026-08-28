// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren} from "react"
import {styled} from "@mui/material/styles"
import Paper from "@mui/material/Paper"
import Box from "@mui/material/Box"
import {faWarning, faTimes} from "@fortawesome/free-solid-svg-icons"
import IconButton from "../IconButton/IconButton"
import Icon from "../Icon/Icon"
import VisuallyHidden from "../VisuallyHidden/VisuallyHidden"
import {useTranslation} from "react-i18next"

const WarnContainer = styled(Paper)`
    padding: 17px;
    display: flex;
    flex-direction: row;
    gap: 8px;
    border-radius: 4px;
    line-height: 19px;
    align-items: center;
`

// How a WarnBox is announced when it appears. Messages on the ballot show up
// and disappear as the voter changes their selections, so without a live region
// a screen reader user is never told why they cannot continue.
export enum EWarnBoxAnnouncement {
    // Interrupts the screen reader — for messages that block progress.
    ASSERTIVE = "assertive",
    // Announced when the screen reader next pauses — for informational messages.
    POLITE = "polite",
    // Not announced at all; for a box that is already announced some other way —
    // an enclosing live region, an aria-describedby reference, or simply being
    // static content that is read in document order.
    SILENT = "silent",
}

const ANNOUNCEMENT_ROLE: Record<EWarnBoxAnnouncement, string | undefined> = {
    [EWarnBoxAnnouncement.ASSERTIVE]: "alert",
    [EWarnBoxAnnouncement.POLITE]: "status",
    [EWarnBoxAnnouncement.SILENT]: undefined,
}

// Polite by default: a screen full of contests mounts one message list per
// contest, and assertive regions interrupt each other so all but the last would
// be lost. Callers opt into ASSERTIVE for a single blocking message.
const DEFAULT_ANNOUNCEMENT = EWarnBoxAnnouncement.POLITE

// The variant is otherwise conveyed only by colour and by an icon that is the
// same for every severity, so the severity is also stated in text for anyone who
// cannot see the styling.
const SEVERITY_KEY: Record<NonNullable<WarnBoxProps["variant"]>, string> = {
    error: "a11y.severity.error",
    warning: "a11y.severity.warning",
    success: "a11y.severity.success",
    info: "a11y.severity.info",
}

interface WarnBoxProps {
    onClose?: () => void
    variant?: "error" | "success" | "warning" | "info"
    className?: string
    id?: string
    warnId?: string
    warnType?: string
    announcement?: EWarnBoxAnnouncement
}

// Derives a CSS class from a warning id (e.g. "errors.implicit.underVote" ->
// "warn--errors-implicit-underVote") so it can be targeted from custom CSS
// without escaping dots
export const warnIdToClassName = (warnId: string): string =>
    `warn--${warnId.replace(/[^a-zA-Z0-9_-]/g, "-")}`

const WarnBox: React.FC<PropsWithChildren<WarnBoxProps>> = ({
    onClose,
    variant,
    className,
    id,
    warnId,
    warnType,
    announcement,
    children,
}) => {
    const {t} = useTranslation()
    const role = ANNOUNCEMENT_ROLE[announcement ?? DEFAULT_ANNOUNCEMENT]

    return (
        <WarnContainer
            variant={variant}
            id={id}
            role={role}
            className={
                [className, warnId ? warnIdToClassName(warnId) : undefined]
                    .filter(Boolean)
                    .join(" ") || undefined
            }
            data-warn-id={warnId}
            data-warn-type={warnType}
        >
            <Icon icon={faWarning} size="lg" aria-hidden="true" />
            <Box flexGrow={2}>
                {variant ? (
                    <VisuallyHidden>{`${t(SEVERITY_KEY[variant])}: `}</VisuallyHidden>
                ) : null}
                {children}
            </Box>
            {onClose ? (
                <IconButton icon={faTimes} onClick={onClose} ariaLabel={t("a11y.dismissMessage")} />
            ) : undefined}
        </WarnContainer>
    )
}

export default WarnBox
