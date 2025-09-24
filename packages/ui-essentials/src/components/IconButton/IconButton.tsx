// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
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
    extends Omit<FontAwesomeIconProps, "onClick">,
        Pick<IconButtonProps, "onClick"> {
    variant?: "inherit" | "primary" | "info" | "warning" | "error" | "success"
    sx?: SxProps<Theme>
}

const ColorMap = {
    primary: theme.palette.black,
    info: theme.palette.blue?.main,
    warning: theme.palette.yellow?.main,
    error: theme.palette.red?.main,
    success: theme.palette.green?.main,
    inherit: "inherit",
}

const IconButton: React.FC<IIconButtonProps> = ({variant, sx, onClick, ...props}) => (
    <StyledButton
        aria-label={props.title || (props as any)["aria-label"] || "icon button"}
        sx={{color: ColorMap[variant || "inherit"], ...sx}}
        onClick={onClick}
    >
        <FontAwesomeIcon {...props} />
    </StyledButton>
)

export default IconButton
