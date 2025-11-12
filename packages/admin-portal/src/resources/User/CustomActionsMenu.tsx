// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {ListControllerResult, useListContext, Menu} from "react-admin"

export const CustomActionsMenu = ({
    anchorEl,
    handleCloseCustomMenu,
    customFiltersList,
    open,
    doContext,
}: {
    anchorEl: HTMLElement | null
    handleCloseCustomMenu: () => void
    customFiltersList: Array<React.ReactNode>
    open: boolean
    doContext: (ctx: ListControllerResult) => void
}) => {
    const listContext = useListContext()

    useEffect(() => {
        doContext(listContext)
    }, [])

    return (
        <Menu
            id="basic-menu"
            anchorEl={anchorEl}
            open={open}
            onClose={handleCloseCustomMenu}
            MenuListProps={{
                "aria-labelledby": "basic-button",
            }}
        >
            {customFiltersList}
        </Menu>
    )
}
