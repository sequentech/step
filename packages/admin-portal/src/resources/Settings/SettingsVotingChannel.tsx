import React from "react"
import styled from "@emotion/styled"

import {Switch, createTheme, ThemeProvider} from "@mui/material"

const SettingsVotingChannelsStyles = {
    Wrapper: styled.div`
        display: flex;
        flex-direction: column;
    `,
    Content: styled.div`
        display: flex;
        width: 239px;
        align-items: center;
        justify-content: space-between;
    `,
    Text: styled.span``,
}

const theme = createTheme({
    components: {
        MuiSwitch: {
            styleOverrides: {
                thumb: {},
                track: {},
                switchBase: {
                    "& + .MuiSwitch-track": {
                        backgroundColor: "rgba(0, 0, 0, 0.12)",
                    },
                    ".MuiSwitch-thumb": {
                        color: "#fff",
                    },
                    "&.Mui-checked": {
                        "& + .MuiSwitch-track": {
                            backgroundColor: "#0F054C",
                            opacity: 0.5,
                        },
                        ".MuiSwitch-thumb": {
                            color: "#0F054C",
                        },
                    },
                    "&.Mui-disabled + .MuiSwitch-track": {
                        opacity: 0.5,
                    },
                },
            },
        },
    },
})

export const SettingsVotingChannels: React.FC<void> = () => {
    const voting: {[key: string]: boolean} = {
        online: false,
        kiosk: true,
        telephone: false,
        paper: false,
    }

    return (
        <ThemeProvider theme={theme}>
            <SettingsVotingChannelsStyles.Wrapper>
                {Object.keys(voting).map((method: string) => (
                    <SettingsVotingChannelsStyles.Content
                        key={method}
                        style={{
                            display: "flex",
                        }}
                    >
                        <SettingsVotingChannelsStyles.Text>
                            {method.charAt(0).toUpperCase() + method.slice(1)} Voting
                        </SettingsVotingChannelsStyles.Text>

                        <Switch defaultChecked />
                    </SettingsVotingChannelsStyles.Content>
                ))}
            </SettingsVotingChannelsStyles.Wrapper>
        </ThemeProvider>
    )
}
