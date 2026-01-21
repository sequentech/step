// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
