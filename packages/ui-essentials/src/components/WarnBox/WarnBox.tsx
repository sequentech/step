// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
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
}

const WarnBox: React.FC<PropsWithChildren<WarnBoxProps>> = ({onClose, variant, children}) => (
    <WarnContainer variant={variant}>
        <Icon icon={faWarning} size="lg" />
        <Box flexGrow={2}>{children}</Box>
        {onClose ? <IconButton icon={faTimes} onClick={onClose} /> : undefined}
    </WarnContainer>
)

export default WarnBox
