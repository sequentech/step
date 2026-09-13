// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren, useEffect, useId, useLayoutEffect, useRef} from "react"
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

const inertBackgrounds = new WeakMap<Element, {count: number; wasInert: boolean}>()

const makeBackgroundInert = (modal: HTMLElement) => {
    const siblings = Array.from(modal.parentElement?.children ?? []).filter(
        (element) =>
            element !== modal &&
            !element.matches(".MuiModal-hidden, .MuiModal-root:not([aria-hidden='true'])")
    )
    siblings.forEach((element) => {
        const state = inertBackgrounds.get(element) ?? {
            count: 0,
            wasInert: element.hasAttribute("inert"),
        }
        state.count += 1
        inertBackgrounds.set(element, state)
        element.setAttribute("inert", "")
    })
    return () => {
        siblings.forEach((element) => {
            const state = inertBackgrounds.get(element)
            if (!state || --state.count > 0) {
                return
            }
            if (!state.wasInert) {
                element.removeAttribute("inert")
            }
            inertBackgrounds.delete(element)
        })
    }
}

const getDialogTabStops = (root: HTMLElement): HTMLElement[] => {
    const candidates = Array.from(
        root.querySelectorAll<HTMLElement>(
            'a[href], button, input, select, textarea, [tabindex], [contenteditable="true"], audio[controls], video[controls], summary'
        )
    ).filter(
        (element) =>
            element.tabIndex >= 0 &&
            !element.matches(":disabled") &&
            !element.closest("[inert]") &&
            element.getClientRects().length > 0 &&
            getComputedStyle(element).visibility === "visible"
    )

    return candidates
        .filter((element) => {
            if (
                !(element instanceof HTMLInputElement) ||
                element.type !== "radio" ||
                !element.name
            ) {
                return true
            }
            const group = candidates.filter(
                (candidate): candidate is HTMLInputElement =>
                    candidate instanceof HTMLInputElement &&
                    candidate.type === "radio" &&
                    candidate.name === element.name &&
                    candidate.form === element.form
            )
            return element === (group.find((radio) => radio.checked) ?? group[0])
        })
        .sort(
            (left, right) =>
                (left.tabIndex || Number.MAX_SAFE_INTEGER) -
                (right.tabIndex || Number.MAX_SAFE_INTEGER)
        )
}

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
    const [isFullScreen, setIsFullScreen] = React.useState<boolean>(false)
    const {t} = useTranslation()
    // Ties the modal to its visible title (and to the error text, when shown) so
    // screen readers announce what the dialog is about when it opens.
    const generatedId = useId()
    const titleId = `${generatedId}-title`
    const errorId = `${generatedId}-error`
    const paperRef = useRef<HTMLDivElement>(null)
    const [modalRoot, setModalRoot] = React.useState<HTMLDivElement | null>(null)

    // aria-hidden alone does not prevent focus. Release inert before MUI's
    // passive focus-restoration effect, including when dialogs overlap.
    useLayoutEffect(() => {
        if (open && modalRoot) {
            return makeBackgroundInert(modalRoot)
        }
    }, [open, modalRoot])

    const handleTabKey = (event: React.KeyboardEvent<HTMLDivElement>) => {
        const paper = paperRef.current
        if (
            event.key !== "Tab" ||
            event.defaultPrevented ||
            !paper ||
            !event.currentTarget.contains(event.target as Node)
        ) {
            return
        }

        const tabStops = getDialogTabStops(paper)
        const first = tabStops[0]
        const last = tabStops[tabStops.length - 1]
        const active = paper.ownerDocument.activeElement
        if (!first) {
            event.preventDefault()
            paper.focus()
        } else if (
            active === paper ||
            active === event.currentTarget ||
            (event.shiftKey ? active === first : active === last)
        ) {
            event.preventDefault()
            const target = event.shiftKey ? last : first
            target.focus()
        }
    }

    useEffect(() => {
        okButtonRef.current = false
        setIsFullScreen(false)
    }, [open])

    let fullClass = className ? `${className} dialog` : "dialog"

    return (
        <MaterialDialog
            ref={setModalRoot}
            onClose={closeDialog}
            open={open}
            slots={{backdrop: StyledBackdrop}}
            slotProps={{
                backdrop: {className: "dialog-backdrop"},
                paper: {className: "dialog-paper", ref: paperRef, tabIndex: -1},
                container: {onKeyDown: handleTabKey},
            }}
            // Keyboard wrapping above replaces MUI's empty tabbable focus guards.
            sx={{
                '& > [data-testid="sentinelStart"], & > [data-testid="sentinelEnd"]': {
                    display: "none",
                },
            }}
            classes={{container: "dialog-container"}}
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
                        buttonClassName="dialog-expand-button"
                    />
                ) : null}
                {hasCloseButton ? (
                    <IconButton
                        icon={faTimesCircle}
                        variant="primary"
                        onClick={closeDialog}
                        className="dialog-icon-close"
                        buttonClassName="dialog-close-button"
                        ariaLabel={t("a11y.closeDialog")}
                    />
                ) : null}
            </DialogTitle>
            <DialogContent className="dialog-content"> {children} </DialogContent>
            <StyledDialogErrorContent
                className="dialog-content dialog-error"
                id={errorId}
                role="alert"
            >
                {errorMessage}
            </StyledDialogErrorContent>
            <StyledDialogActions
                className={`dialog-actions ${middleActions ? "has-middle" : "no-middle"}`}
            >
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
