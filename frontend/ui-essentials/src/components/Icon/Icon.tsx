// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {FontAwesomeIcon, FontAwesomeIconProps} from "@fortawesome/react-fontawesome"
import {theme} from "../../services/theme"

export interface IconProps extends FontAwesomeIconProps {
    variant?: "inherit" | "primary" | "info" | "warning" | "error" | "success" | "form"
}

const ColorMap = {
    primary: theme.palette.black,
    info: theme.palette.brandColor,
    warning: theme.palette.yellow?.main,
    error: theme.palette.errorColor,
    success: theme.palette.green?.main,
    form: theme.palette.customGrey?.dark,
    inherit: undefined,
}

const Icon: React.FC<IconProps> = ({variant, ...props}) => (
    <FontAwesomeIcon color={ColorMap[variant || "inherit"]} {...props} />
)

export default Icon
