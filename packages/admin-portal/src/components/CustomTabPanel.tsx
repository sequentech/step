// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"

interface TabPanelProps {
    children?: React.ReactNode
    index: number
    value: number
}

export const CustomTabPanel = (props: TabPanelProps) => {
    const {children, value, index, ...other} = props

    if (value !== index) {
        return null
    }

    return (
        <div
            role="tabpanel"
            hidden={value !== index}
            id={`panel-tabpanel-${index}`}
            aria-labelledby={`panel-tab-${index}`}
            {...other}
        >
            {children}
        </div>
    )
}
