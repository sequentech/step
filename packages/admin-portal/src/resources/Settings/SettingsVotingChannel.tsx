import React from "react"
import styled from "@emotion/styled"

import {Switch} from "@mui/material"

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
    Text: styled.span`
        text-transform: capitalize
    `,
}

export const SettingsVotingChannels: React.FC<void> = () => {
    const voting: {[key: string]: boolean} = {
        online: false,
        kiosk: true,
        telephone: false,
        paper: false,
    }

    return (
        <SettingsVotingChannelsStyles.Wrapper>
            {Object.keys(voting).map((method: string) => (
                <SettingsVotingChannelsStyles.Content
                    key={method}
                    style={{
                        display: "flex",
                    }}
                >
                    <SettingsVotingChannelsStyles.Text>
                        {method} Voting
                    </SettingsVotingChannelsStyles.Text>

                    <Switch defaultChecked />
                </SettingsVotingChannelsStyles.Content>
            ))}
        </SettingsVotingChannelsStyles.Wrapper>
    )
}
