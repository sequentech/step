// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {SyntheticEvent} from "react"
import styled from "@emotion/styled"

import {Tabs as MuiTabs, Tab as MuiTab} from "@mui/material"

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

export const Tabs: React.FC<{elements: {label: string; component: React.FC}[]}> = ({
    elements,
    ...props
}) => {
    const baseUrl = new URL(window.location.href)
    const [selectedTab, setSelectedTab] = React.useState(
        Number.parseInt(baseUrl?.searchParams?.get("tabIndex") ?? "0")
    )

    const handleChange = (event: SyntheticEvent<Element, Event>, newValue: number) => {
        setSelectedTab(newValue)
    }

    return (
        <TabStyles.Wrapper>
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
                {elements.map((tab: {label: string}) => (
                    <MuiTab key={tab.label} label={tab.label} />
                ))}
            </MuiTabs>

            <TabStyles.Content>{elements[selectedTab]?.component(props)}</TabStyles.Content>
        </TabStyles.Wrapper>
    )
}
