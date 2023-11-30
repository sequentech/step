import React, { useEffect, useState } from "react"
import styled from "@emotion/styled"

import {Switch} from "@mui/material"
import { useEditController, Loading, Error } from 'react-admin';
import { useTenantStore } from '@/providers/TenantContextProvider';


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

export const SettingsVotingChannels: React.FC<any> = () => {
    const [tenantId] = useTenantStore()
    const { record, save, isLoading } = useEditController({ resource: 'sequent_backend_tenant', id: tenantId, redirect: false, undoable: false });

    console.log('record', record);

    const [voting, setVoting] = useState<any>({
        online: record?.votting_channels?.online || false,
        kiosk: record?.votting_channels?.kiosk || false,
        telephone: record?.votting_channels?.telephone || false,
        paper: record?.votting_channels?.paper || false,
    });

    const handleToggle = (method: any) => {
        const updatedVoting = {
            ...voting,
            [method]: !voting[method],
        };

        setVoting(updatedVoting);

        if (save) {
            save({ 
                ...record, voting_channels: {
                    ...record?.voting_channels,
                    ...updatedVoting,
                }
            });
        }
    };

    useEffect(() => {
        if (record.voting_channels) {
            setVoting({
                ...record?.voting_channels,
            })
        }
    }, [record]);

    if (isLoading) return null;

    return (
        <SettingsVotingChannelsStyles.Wrapper>
            {Object.keys(voting).map((method: string) => (
                <SettingsVotingChannelsStyles.Content key={method}>
                    <SettingsVotingChannelsStyles.Text>
                        {method} Voting
                    </SettingsVotingChannelsStyles.Text>

                    <Switch 
                        checked={voting[method]}
                        onChange={() => handleToggle(method)}
                    />
                </SettingsVotingChannelsStyles.Content>
            ))}
        </SettingsVotingChannelsStyles.Wrapper>
    )
}

