// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Link as RouterLink, LinkProps as RouterLinkProps} from "react-router-dom"

const LinkBehavior = React.forwardRef<
    HTMLAnchorElement,
    Omit<RouterLinkProps, "to"> & {href: RouterLinkProps["to"]}
>((props, ref) => {
    const {href, ...other} = props
    // Map href (MUI) -> to (react-router)
    return <RouterLink ref={ref} to={href} {...other} />
})
LinkBehavior.displayName = "LinkBehavior"

export default LinkBehavior
