// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactNode} from "react"
import {Dialog, DialogTitle, DialogContent, Box, InputLabel} from "@mui/material"
import {faTimesCircle} from "@fortawesome/free-solid-svg-icons"
import {styled} from "@mui/system"
import {IconButton} from "@sequentech/ui-essentials"

interface GenericDialogProps {
    open: boolean
    onClose: () => void
    title: ReactNode
    children: ReactNode
    className?: string
    actions?: ReactNode
}

const DialogStyle = styled(Dialog)`
    & .MuiPaper-root {
        width: 650px;
        max-width: unset;
        padding-bottom: 12px;
    }
    & .MuiDialogContent-root {
        @media (max-width: ${({theme}) => theme.breakpoints.values.sm}px) {
            padding: 16px 24px 0 24px !important;
        }
    }
`

const FormDialog: React.FC<GenericDialogProps> = ({open, onClose, title, children}) => {
    return (
        <DialogStyle open={open} onClose={onClose} className="dialog">
            <DialogTitle className="dialog-title">
                <Box
                    component="span"
                    flexGrow={2}
                    pt="3px"
                    fontWeight="bold"
                    className="dialog-title-text"
                >
                    {title}
                </Box>
                <IconButton
                    icon={faTimesCircle as any}
                    variant="primary"
                    onClick={() => onClose()}
                    className="dialog-icon-close"
                />
            </DialogTitle>
            <DialogContent className="dialog-content">{children}</DialogContent>
        </DialogStyle>
    )
}

export default FormDialog
