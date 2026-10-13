// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {PropsWithChildren, useId, useRef} from "react"
import {Box, Button} from "@mui/material"
import MaterialDialog from "@mui/material/Dialog"
import {styled} from "@mui/material/styles"
import {faXmark} from "@fortawesome/free-solid-svg-icons"
import {Icon, theme} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"

const PHONE = theme.breakpoints.down("sm")

/**
 * The dialogs of a ballot with slates: a plain white panel up to 550px wide,
 * 16px from the screen edges, with the title and a close button on top and the
 * answers at the bottom right; on a phone the answers take a row each, the
 * primary one first.
 */
const Panel = styled(MaterialDialog)`
    .MuiBackdrop-root {
        background: rgba(15, 5, 40, 0.38);
    }

    .slate-dialog-paper {
        box-sizing: border-box;
        width: min(550px, calc(100% - 32px));
        max-width: none;
        max-height: calc(100dvh - 32px);
        margin: 16px;
        border-radius: 12px;
        box-shadow: 0 20px 85px #08022133;
        border: none;
        overflow: auto;
    }

    .slate-dialog-inner {
        padding: 28px;
        overflow-wrap: anywhere;
        font-size: 15px;
        line-height: 1.55;
    }

    .slate-dialog-top {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 12px;
    }

    .slate-dialog-title {
        max-width: 390px;
        margin: 0 0 8px;
        font-size: 23px;
        font-weight: 700;
        line-height: 1.25;
        letter-spacing: -0.35px;
        color: #191d23;
    }

    .dialog-close-button {
        flex: none;
        width: 44px;
        min-width: 44px;
        height: 44px;
        min-height: 44px;
        margin: -10px -12px 0 0;
        padding: 10px;
        border: 1px solid transparent;
        border-radius: 4px;
        background: transparent;
        box-shadow: none;
        color: ${theme.palette.brandColor};
    }

    .slate-dialog-actions {
        display: flex;
        justify-content: flex-end;
        gap: 10px;
        margin-top: 23px;
    }

    .slate-dialog-actions .MuiButton-root {
        min-height: 44px;
        padding: 10px 17px;
        font-size: 15px;
    }

    ${PHONE} {
        .slate-dialog-inner {
            padding: 23px 20px;
            font-size: 14px;
        }

        .slate-dialog-title {
            font-size: 22px;
        }

        .slate-dialog-actions {
            flex-direction: column-reverse;
            gap: 9px;
            margin-top: 22px;
        }

        .slate-dialog-actions .MuiButton-root {
            width: 100%;
            margin: 0;
            font-size: 14px;
        }
    }
`

export interface SlateDialogProps extends PropsWithChildren {
    /** Stable class of this dialog, on its root. */
    className: string
    open: boolean
    title: string
    /** The primary answer; none when only closing is possible. */
    ok?: string
    cancel?: string
    /** True for the primary answer; false for the other answer, the X and Escape. */
    handleClose: (confirmed: boolean) => void
}

export const SlateDialog: React.FC<SlateDialogProps> = ({
    className,
    open,
    title,
    ok,
    cancel,
    handleClose,
    children,
}) => {
    const {t} = useTranslation()
    const titleId = `${useId()}-title`
    const closeRef = useRef<HTMLButtonElement>(null)

    return (
        <Panel
            className={`${className} slate-dialog`}
            open={open}
            onClose={() => handleClose(false)}
            aria-labelledby={titleId}
            slotProps={{
                paper: {className: "slate-dialog-paper"},
                // As the native dialog of the design, the first control takes the focus.
                transition: {onEntered: () => closeRef.current?.focus()},
            }}
        >
            <Box className="slate-dialog-inner">
                <Box className="slate-dialog-top">
                    <h2 className="slate-dialog-title" id={titleId}>
                        {title}
                    </h2>
                    <Button
                        className="dialog-close-button"
                        variant="secondary"
                        ref={closeRef}
                        aria-label={t("slates.dialog.close")}
                        onClick={() => handleClose(false)}
                    >
                        <Icon className="dialog-close-icon" icon={faXmark} aria-hidden="true" />
                    </Button>
                </Box>
                <Box className="slate-dialog-content">{children}</Box>
                <Box className="slate-dialog-actions">
                    {cancel ? (
                        <Button
                            className="cancel-button"
                            variant="secondary"
                            onClick={() => handleClose(false)}
                        >
                            {cancel}
                        </Button>
                    ) : null}
                    {ok ? (
                        <Button className="ok-button" onClick={() => handleClose(true)}>
                            {ok}
                        </Button>
                    ) : null}
                </Box>
            </Box>
        </Panel>
    )
}
