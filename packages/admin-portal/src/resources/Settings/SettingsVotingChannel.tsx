// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {styled} from "@mui/material/styles"
import {Switch} from "@mui/material"
import {useEditController} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useTranslation} from "react-i18next"

const SettingsVotingChannelsStyles = {
    Wrapper: styled("div")`
        display: flex;
        flex-direction: column;
    `,
    Content: styled("div")`
        display: flex;
        width: 239px;
        align-items: center;
        justify-content: space-between;
    `,
    Text: styled("span")`
        text-transform: capitalize;
    `,
}

export const SettingsVotingChannels: React.FC<void> = () => {
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const {record, save, isLoading} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })

    const [voting, setVoting] = useState<any>({
        online: record?.voting_channels?.online || true,
        kiosk: record?.voting_channels?.kiosk || false,
    })

    const handleToggle = (method: any) => {
        const updatedVoting = {
            ...voting,
            [method]: !voting[method],
        }

        console.log("Update Voting", updatedVoting, method)

        setVoting(updatedVoting)

        if (save) {
            save({
                voting_channels: {
                    online: updatedVoting.online,
                    kiosk: updatedVoting.kiosk,
                },
            })
        }
    }

    useEffect(() => {
        console.log(record)
        if (record.voting_channels) {
            setVoting({
                online: record?.voting_channels?.online || true,
                kiosk: record?.voting_channels?.kiosk || false,
            })
        }
    }, [record])

    if (isLoading) return null

    return (
        <SettingsVotingChannelsStyles.Wrapper>
            {Object.keys(voting).map((method: string) => (
                <SettingsVotingChannelsStyles.Content key={method}>
                    <SettingsVotingChannelsStyles.Text>
                        {t(`electionTypeScreen.common.${method}Voting`)}
                    </SettingsVotingChannelsStyles.Text>

                    <Switch
                        checked={voting?.[method] || false}
                        onChange={() => handleToggle(method)}
                    />
                </SettingsVotingChannelsStyles.Content>
            ))}
        </SettingsVotingChannelsStyles.Wrapper>
    )
}
