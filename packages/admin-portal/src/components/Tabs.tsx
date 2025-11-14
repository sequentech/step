// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {SyntheticEvent} from "react"
import {styled} from "@mui/material/styles"

import {Tabs as MuiTabs, Tab as MuiTab, Box} from "@mui/material"
import {useTranslation} from "react-i18next"

const TabStyles = {
    Wrapper: styled("div")`
        display: flex;
        flex-direction: column;
        align-items: left;
        border-bottom: 1px solid rgba(0, 0, 0, 0.12);

        .MuiTabs-scroller {
            margin-bottom: 10px;
        }
    `,
    Content: styled("div")`
        width: 100%;
        padding: 2rem var(--2, 16px);
    `,
}

export const Tabs: React.FC<{
    elements: Array<{
        label: string
        component: React.ComponentType<any>
        action?: (index: number) => void
    }>
}> = ({elements, ...props}) => {
    const {t} = useTranslation()
    const baseUrl = new URL(window.location.href)
    const [selectedTab, setSelectedTab] = React.useState(
        Number.parseInt(baseUrl?.searchParams?.get("tabIndex") ?? "0")
    )

    const handleChange = (event: SyntheticEvent<Element, Event>, newValue: number) => {
        setSelectedTab(newValue)
    }

    const SelectedComponent = elements[selectedTab]?.component

    return (
        <TabStyles.Wrapper>
            <Box
                sx={{
                    bgcolor: "background.paper",
                    borderBottom: 1,
                    borderColor: "divider",
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
                    {elements.map(
                        (tab: {label: string; action?: (index: number) => void}, index: number) => (
                            <MuiTab
                                key={tab.label}
                                label={tab.label}
                                onClick={() => tab?.action?.(index) ?? null}
                            />
                        )
                    )}
                </MuiTabs>
            </Box>
            <TabStyles.Content>
                <React.Suspense fallback={<div>{t("loading")}</div>}>
                    {SelectedComponent ? <SelectedComponent {...props} /> : null}
                </React.Suspense>
            </TabStyles.Content>
        </TabStyles.Wrapper>
    )
}
