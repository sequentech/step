// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren} from "react"
import DialogTitle from "@mui/material/DialogTitle"
import MaterialDialog from "@mui/material/Dialog"
import {Backdrop, Box, Button} from "@mui/material"
import DialogContent from "@mui/material/DialogContent"
import DialogActions from "@mui/material/DialogActions"
import {
    faTimesCircle,
    faInfoCircle,
    faExclamationTriangle,
    faBullseye,
} from "@fortawesome/free-solid-svg-icons"
import styledEmotion from "@emotion/styled"
import Icon from "../Icon/Icon"
import IconButton from "../IconButton/IconButton"

const StyledBackdrop = styledEmotion(Backdrop)`
    opacity: 0.5 !important;
`

export interface DialogProps extends PropsWithChildren {
    handleClose: (value: boolean) => void
    open: boolean
    title: string
    cancel?: string
    ok: string
    okEnabled?: () => boolean
    variant?: "warning" | "info" | "action"
    fullWidth?: boolean
}

const Dialog: React.FC<DialogProps> = ({
    children,
    handleClose,
    open,
    title,
    cancel,
    ok,
    okEnabled,
    variant,
    fullWidth = false,
}) => {
    const okVariant = "info" === variant ? "primary" : "solidWarning"
    const faIcon = "info" === variant ? faInfoCircle : faExclamationTriangle
    const infoVariant = "action" === variant ? "error" : variant
    const cancelVariant = "cancel"
    const closeDialog = () => handleClose(false)
    const clickOk = () => handleClose(true)

    return (
        <MaterialDialog
            onClose={closeDialog}
            open={open}
            slots={{backdrop: StyledBackdrop}}
            fullWidth={fullWidth}
        >
            <DialogTitle>
                <Icon variant={infoVariant} icon={faIcon} fontSize="24px" />
                <Box component="span" flexGrow={2} pt="3px" fontWeight="bold">
                    {title}
                </Box>
                <IconButton icon={faTimesCircle} variant="primary" onClick={closeDialog} />
            </DialogTitle>
            <DialogContent>{children}</DialogContent>
            <DialogActions>
                {cancel ? (
                    <Button
                        variant={cancelVariant}
                        onClick={closeDialog}
                        sx={{minWidth: "unset", flexGrow: 2}}
                    >
                        {cancel}
                    </Button>
                ) : undefined}
                <Button
                    disabled={okEnabled ? !okEnabled() : undefined}
                    variant={okVariant as any}
                    onClick={clickOk}
                    sx={{minWidth: "unset", flexGrow: 2}}
                >
                    {ok}
                </Button>
            </DialogActions>
        </MaterialDialog>
    )
}

export default Dialog
