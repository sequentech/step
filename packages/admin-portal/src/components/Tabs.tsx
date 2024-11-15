// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {SyntheticEvent} from "react"
import styled from "@emotion/styled"

import {Tabs as MuiTabs, Tab as MuiTab, Box} from "@mui/material"

const TabStyles = {
    Wrapper: styled.div`
        display: flex;
        flex-direction: column;
        align-items: left;

        .MuiTabs-scroller {
            border-bottom: 1px solid rgba(0, 0, 0, 0.12);
            margin-bottom: 10px;
        }
    `,
    Content: styled.div`
        padding: 2rem var(--2, 16px);
    `,
}

export const Tabs: React.FC<{
    elements: {label: string; component: React.FC; action?: () => void}[]
}> = ({elements, ...props}) => {
    const baseUrl = new URL(window.location.href)
    const [selectedTab, setSelectedTab] = React.useState(
        Number.parseInt(baseUrl?.searchParams?.get("tabIndex") ?? "0")
    )

    const handleChange = (event: SyntheticEvent<Element, Event>, newValue: number) => {
        setSelectedTab(newValue)
    }

    return (
        <TabStyles.Wrapper>
            <Box
                sx={{
                    maxWidth: {xs: 360, sm: 420, m: 680, lg: 900, xl: "100%"},
                    bgcolor: "background.paper",
                }}
            >
                <MuiTabs
                    variant="scrollable"
                    allowScrollButtonsMobile
                    scrollButtons="auto"
                    value={selectedTab}
                    onChange={handleChange}
                    indicatorColor="primary"
                    textColor="primary"
                    aria-label="disabled tabs example"
                >
                    {elements.map((tab: {label: string; action?: () => void}) => (
                        <MuiTab key={tab.label} label={tab.label} onClick={tab?.action} />
                    ))}
                </MuiTabs>
            </Box>

            <TabStyles.Content>{elements[selectedTab]?.component(props)}</TabStyles.Content>
        </TabStyles.Wrapper>
    )
}
