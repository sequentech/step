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

const WarnContainer = styled(Paper)`
    padding: 17px;
    display: flex;
    flex-direction: row;
    gap: 8px;
    border-radius: 4px;
    line-height: 19px;
    align-items: center;
`

interface WarnBoxProps {
    onClose?: () => void
    variant?: "error" | "success" | "warning" | "info"
    className?: string
    id?: string
    warnId?: string
    warnType?: string
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
    children,
}) => (
    <WarnContainer
        variant={variant}
        id={id}
        className={
            [className, warnId ? warnIdToClassName(warnId) : undefined].filter(Boolean).join(" ") ||
            undefined
        }
        data-warn-id={warnId}
        data-warn-type={warnType}
    >
        <Icon icon={faWarning} size="lg" />
        <Box flexGrow={2}>{children}</Box>
        {onClose ? <IconButton icon={faTimes} onClick={onClose} /> : undefined}
    </WarnContainer>
)

export default WarnBox
