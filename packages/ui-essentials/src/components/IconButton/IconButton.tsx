// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {FontAwesomeIcon, FontAwesomeIconProps} from "@fortawesome/react-fontawesome"
import {theme} from "../../services/theme"
import {IconButton as MuiIconButton, SxProps, Theme, styled} from "@mui/material"
import {IconButtonProps} from "@mui/material/IconButton"

const StyledButton = styled(MuiIconButton)`
    &:hover {
        background-color: unset;
    }
    &:active {
        border: none;
    }
`

export interface IIconButtonProps
    extends
        Omit<FontAwesomeIconProps, "onClick" | "aria-label" | "aria-labelledby">,
        Pick<IconButtonProps, "onClick" | "disabled"> {
    variant?: "inherit" | "primary" | "info" | "warning" | "error" | "success"
    sx?: SxProps<Theme>
    // An icon conveys no text to assistive technology, so every icon button
    // needs an explicit name: either `ariaLabel` (or the `title` tooltip, which
    // is used as a fallback) or `ariaLabelledby` pointing at visible text. These
    // are separate props because everything else on this interface is spread
    // onto the icon, where an accessible name would not reach the button.
    ariaLabel?: string
    ariaLabelledby?: string
}

// Callers outside the voting portal have not been given names yet, so this
// placeholder is kept to avoid leaving those buttons with no name at all. It is
// not an acceptable accessible name: pass ariaLabel or ariaLabelledby instead.
const UNNAMED_FALLBACK = "icon button"

const ColorMap = {
    primary: theme.palette.black,
    info: theme.palette.blue?.main,
    warning: theme.palette.yellow?.main,
    error: theme.palette.red?.main,
    success: theme.palette.green?.main,
    inherit: "inherit",
}

const IconButton: React.FC<IIconButtonProps> = ({
    variant,
    sx,
    onClick,
    disabled,
    ariaLabel,
    ariaLabelledby,
    ...iconProps
}) => (
    <StyledButton
        aria-label={ariaLabelledby ? undefined : (ariaLabel ?? iconProps.title ?? UNNAMED_FALLBACK)}
        aria-labelledby={ariaLabelledby}
        disabled={disabled}
        sx={{color: ColorMap[variant || "inherit"], ...sx}}
        onClick={onClick}
    >
        <FontAwesomeIcon {...iconProps} />
    </StyledButton>
)

export default IconButton
