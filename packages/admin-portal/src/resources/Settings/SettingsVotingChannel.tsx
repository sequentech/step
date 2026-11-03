// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {styled} from "@mui/material/styles"
import {Switch} from "@mui/material"
import {useEditController} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useTranslation} from "react-i18next"
import {IVotingChannelsConfig} from "@sequentech/ui-core"

type TenantVotingChannels = Pick<IVotingChannelsConfig, "online" | "kiosk" | "telephone">
type TenantVotingChannel = keyof TenantVotingChannels

// `??`, not `||`: disabling a channel stores `false`, and `false || true`
// reads back as enabled, so the online toggle could never be turned off.
const toTenantVotingChannels = (
    channels?: Partial<TenantVotingChannels> | null
): TenantVotingChannels => ({
    online: channels?.online ?? true,
    kiosk: channels?.kiosk ?? false,
    telephone: channels?.telephone ?? false,
})

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

    const [voting, setVoting] = useState<TenantVotingChannels>(
        toTenantVotingChannels(record?.voting_channels)
    )

    const handleToggle = (method: TenantVotingChannel) => {
        const updatedVoting: TenantVotingChannels = {
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
                    telephone: updatedVoting.telephone,
                },
            })
        }
    }

    useEffect(() => {
        if (record?.voting_channels) {
            setVoting(toTenantVotingChannels(record.voting_channels))
        }
    }, [record])

    if (isLoading) return null

    return (
        <SettingsVotingChannelsStyles.Wrapper>
            {(Object.keys(voting) as TenantVotingChannel[]).map((method) => (
                <SettingsVotingChannelsStyles.Content key={method}>
                    <SettingsVotingChannelsStyles.Text>
                        {t(`electionTypeScreen.common.${method}Voting`)}
                    </SettingsVotingChannelsStyles.Text>

                    <Switch checked={voting[method]} onChange={() => handleToggle(method)} />
                </SettingsVotingChannelsStyles.Content>
            ))}
        </SettingsVotingChannelsStyles.Wrapper>
    )
}
