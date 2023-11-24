import React from "react"
import styled from "@emotion/styled"

import {Tabs as MuiTabs, Tab as MuiTab, createTheme, ThemeProvider} from "@mui/material"

const TabStyles = {
    Wrapper: styled.div`
        display: flex;
        flex-direction: column;
        align-items: left;
    `,
    Content: styled.div`
        padding: 2rem var(--2, 16px);
    `,
}

const theme = createTheme({
    components: {
        MuiTabs: {
            styleOverrides: {
                indicator: {
                    backgroundColor: "#43E3A1",
                },
            },
        },
        MuiTab: {
            styleOverrides: {
                root: {
                    "textTransform": "uppercase",
                    "fontWeight": "500",
                    "fontSize": "14px",
                    "fontFamily": "Roboto",
                    "lineHeight": "24px",
                    "color": "#000",
                    "opacity": 0.4,
                    "letter": "0.4",
                    "cursor": "pointer",
                    "&:hover": {
                        opacity: 0.6,
                    },
                    "&.Mui-selected": {
                        color: "#0F054C",
                        opacity: 1,
                    },
                },
            },
        },
    },
})

export const Tabs: React.FC<any> = ({elements}) => {
    const [selectedTab, setSelectedTab] = React.useState(0)

    const handleChange = (event: any, newValue: any) => {
        setSelectedTab(newValue)
    }

    return (
        <ThemeProvider theme={theme}>
            <TabStyles.Wrapper>
                <MuiTabs
                    value={selectedTab}
                    onChange={handleChange}
                    indicatorColor="primary"
                    textColor="primary"
                    aria-label="disabled tabs example"
                >
                    {elements.map((tab: any) => (
                        <MuiTab key={tab.label} label={tab.label} />
                    ))}
                </MuiTabs>

                <TabStyles.Content>{elements[selectedTab]?.component()}</TabStyles.Content>
            </TabStyles.Wrapper>
        </ThemeProvider>
    )
}
