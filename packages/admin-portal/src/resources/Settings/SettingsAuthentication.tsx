// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"

import styled from "@emotion/styled"

import {Switch} from "@mui/material"
import {useEditController} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useTranslation} from "react-i18next"

import {ITenantSettings} from "@sequentech/ui-core"

const SettingsAuthenticationStyles = {
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
        text-transform: capitalize;
    `,
}

export const SettingsAuthentication: React.FC<void> = () => {
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const {record, save, isLoading} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })

    const [goldenAuthentication, setGoldenAuthentication] = useState<boolean>(
        (record?.settings as ITenantSettings | undefined)?.golden_authentication ?? true
    )

    const handleToggle = () => {
        const updatedGoldenAuthentication = !goldenAuthentication

        console.log("Update Golden Authentication", updatedGoldenAuthentication)

        setGoldenAuthentication(updatedGoldenAuthentication)

        if (save) {
            save({
                settings: {
                    ...((record?.settings as ITenantSettings | undefined) ?? {}),
                    golden_authentication: updatedGoldenAuthentication,
                },
            })
        }
    }

    useEffect(() => {
        console.log(record)
        if (record.golden_authentication) {
            setGoldenAuthentication(record?.golden_authentication || true)
        }
    }, [record])

    if (isLoading) return null

    return (
        <SettingsAuthenticationStyles.Wrapper>
            <SettingsAuthenticationStyles.Content key="golden_authentication">
                <SettingsAuthenticationStyles.Text>
                    {t(`todo golden_authentication`)}
                </SettingsAuthenticationStyles.Text>

                <Switch checked={goldenAuthentication} onChange={() => handleToggle()} />
            </SettingsAuthenticationStyles.Content>
        </SettingsAuthenticationStyles.Wrapper>
    )
}
