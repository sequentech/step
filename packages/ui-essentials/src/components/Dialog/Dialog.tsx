// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren, useEffect, useRef} from "react"
import DialogTitle from "@mui/material/DialogTitle"
import MaterialDialog from "@mui/material/Dialog"
import {Backdrop, Box, Button, Breakpoint} from "@mui/material"
import DialogContent from "@mui/material/DialogContent"
import DialogActions from "@mui/material/DialogActions"
import {faTimesCircle, faInfoCircle, faExclamationTriangle} from "@fortawesome/free-solid-svg-icons"
import styledEmotion from "@emotion/styled"
import Icon from "../Icon/Icon"
import IconButton from "../IconButton/IconButton"
import {styled as muiStyled} from "@mui/material/styles"

const StyledBackdrop = styledEmotion(Backdrop)`
    opacity: 0.5 !important;
`

const StyledDialogActions = muiStyled(DialogActions)`
    @media (max-width: 600px) {
        &.has-middle.MuiDialogActions-root {
            flex-direction: column !important;
            gap: 5px !important;
        &.has-middle button.MuiButtonBase-root {
            width: 100% !important;
            margin: 0 !important;
        }
    }
`

const StyledDialogErrorContent = muiStyled(DialogContent)(({theme}) => ({
    color: theme.palette.errorColor,
}))

export interface DialogProps extends PropsWithChildren {
    handleClose: (value: boolean) => void
    open: boolean
    title: string
    cancel?: string
    middleActions?: React.ReactElement[]
    ok: string
    okEnabled?: () => boolean
    variant?: "warning" | "info" | "action" | "softwarning"
    fullWidth?: boolean
    maxWidth?: Breakpoint | false
    errorMessage?: string
    hasCloseButton?: boolean
}

const Dialog: React.FC<DialogProps> = ({
    children,
    handleClose,
    open,
    title,
    cancel,
    middleActions,
    ok,
    okEnabled,
    variant,
    fullWidth = false,
    maxWidth = "xs",
    errorMessage,
    hasCloseButton,
}) => {
    const okVariant =
        "info" === variant ? "primary" : "softwarning" === variant ? "softWarning" : "solidWarning"
    const faIcon = "info" === variant ? faInfoCircle : faExclamationTriangle
    const infoVariant =
        "action" === variant ? "error" : "softwarning" === variant ? "warning" : variant
    const cancelVariant = "cancel"
    const closeDialog = () => handleClose(false)
    const clickOk = () => {
        okButtonRef.current = true
        handleClose(true)
    }

    const okButtonRef = useRef<boolean>(false)

    useEffect(() => {
        okButtonRef.current = false
    }, [open])

    return (
        <MaterialDialog
            onClose={closeDialog}
            open={open}
            slots={{backdrop: StyledBackdrop}}
            fullWidth={fullWidth}
            maxWidth={maxWidth}
            className="dialog"
        >
            <DialogTitle className="dialog-title">
                <Icon
                    variant={infoVariant}
                    icon={faIcon}
                    fontSize="24px"
                    className="dialog-icon-info"
                />
                <Box
                    component="span"
                    flexGrow={2}
                    pt="3px"
                    fontWeight="bold"
                    className="dialog-title-text"
                >
                    {title}
                </Box>
                {hasCloseButton ? (
                    <IconButton
                        icon={faTimesCircle}
                        variant="primary"
                        onClick={closeDialog}
                        className="dialog-icon-close"
                    />
                ) : null}
            </DialogTitle>
            <DialogContent className="dialog-content"> {children} </DialogContent>
            <StyledDialogErrorContent className="dialog-content">
                {errorMessage}
            </StyledDialogErrorContent>
            <StyledDialogActions className={middleActions ? "has-middle" : "no-middle"}>
                {cancel ? (
                    <Button
                        className="cancel-button"
                        variant={cancelVariant}
                        onClick={closeDialog}
                        sx={{minWidth: "unset", flexGrow: 2}}
                    >
                        {cancel}
                    </Button>
                ) : undefined}
                {middleActions &&
                    middleActions.map((action, index) => (
                        <React.Fragment key={index}>{action}</React.Fragment>
                    ))}
                <Button
                    className="ok-button"
                    disabled={okButtonRef.current || (okEnabled ? !okEnabled() : undefined)}
                    variant={okVariant as any}
                    onClick={clickOk}
                    sx={{minWidth: "unset", flexGrow: 2}}
                >
                    {ok}
                </Button>
            </StyledDialogActions>
        </MaterialDialog>
    )
}

export default Dialog
