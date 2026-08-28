// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren, useEffect, useId, useRef} from "react"
import DialogTitle from "@mui/material/DialogTitle"
import MaterialDialog from "@mui/material/Dialog"
import {Backdrop, Box, Button, Breakpoint} from "@mui/material"
import DialogContent from "@mui/material/DialogContent"
import DialogActions from "@mui/material/DialogActions"
import {
    faTimesCircle,
    faInfoCircle,
    faExclamationTriangle,
    faExpand,
    faCompress,
} from "@fortawesome/free-solid-svg-icons"
import {styled} from "@mui/material/styles"
import Icon from "../Icon/Icon"
import IconButton from "../IconButton/IconButton"
import {useTranslation} from "react-i18next"

const StyledBackdrop = styled(Backdrop)`
    opacity: 0.5 !important;
`

const StyledDialogActions = styled(DialogActions)`
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

const StyledDialogErrorContent = styled(DialogContent)(({theme}) => ({
    color: theme.palette.errorColor,
}))

export interface DialogProps extends PropsWithChildren {
    handleClose: (value: boolean) => void
    open: boolean
    title: string
    cancel?: string
    middleActions?: React.ReactElement[]
    ok?: string
    okEnabled?: () => boolean
    variant?: "warning" | "info" | "action" | "softwarning"
    fullWidth?: boolean
    maxWidth?: Breakpoint | false
    errorMessage?: string
    hasCloseButton?: boolean
    expandable?: boolean
    className?: string
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
    expandable,
    className,
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
    const {t} = useTranslation()
    // Ties the modal to its visible title (and to the error text, when shown) so
    // screen readers announce what the dialog is about when it opens.
    const generatedId = useId()
    const titleId = `${generatedId}-title`
    const errorId = `${generatedId}-error`
    const [isFullScreen, setIsFullScreen] = React.useState<boolean>(false)

    useEffect(() => {
        okButtonRef.current = false
        setIsFullScreen(false)
    }, [open])

    let fullClass = className ? `${className} dialog` : "dialog"

    return (
        <MaterialDialog
            onClose={closeDialog}
            open={open}
            slots={{backdrop: StyledBackdrop}}
            fullWidth={fullWidth}
            maxWidth={maxWidth}
            fullScreen={isFullScreen}
            className={fullClass}
            aria-labelledby={titleId}
            aria-describedby={errorMessage ? errorId : undefined}
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
                    id={titleId}
                    flexGrow={2}
                    pt="3px"
                    fontWeight="bold"
                    className="dialog-title-text"
                >
                    {title}
                </Box>
                {expandable ? (
                    <IconButton
                        icon={isFullScreen ? faCompress : faExpand}
                        variant="primary"
                        onClick={() => setIsFullScreen((prev) => !prev)}
                        className="dialog-icon-expand"
                    />
                ) : null}
                {hasCloseButton ? (
                    <IconButton
                        icon={faTimesCircle}
                        variant="primary"
                        onClick={closeDialog}
                        className="dialog-icon-close"
                        ariaLabel={t("a11y.closeDialog")}
                    />
                ) : null}
            </DialogTitle>
            <DialogContent className="dialog-content"> {children} </DialogContent>
            <StyledDialogErrorContent className="dialog-content" id={errorId} role="alert">
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
                {ok ? (
                    <Button
                        className="ok-button"
                        disabled={okButtonRef.current || (okEnabled ? !okEnabled() : undefined)}
                        variant={okVariant as any}
                        onClick={clickOk}
                        sx={{minWidth: "unset", flexGrow: 2}}
                    >
                        {ok}
                    </Button>
                ) : null}
            </StyledDialogActions>
        </MaterialDialog>
    )
}

export default Dialog
