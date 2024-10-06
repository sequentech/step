// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"

import styled from "@emotion/styled"

import {Switch} from "@mui/material"
import {useEditController} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useTranslation} from "react-i18next"

const SettingsTemplatesStyles = {
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

export const SettingsTemplates: React.FC<void> = () => {
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const {record, save, isLoading} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })

    const [setting, setSetting] = useState<any>({
        mail: record?.settings?.mail || true,
        sms: record?.settings?.sms || false,
    })

    const handleToggle = (method: any) => {
        const updatedSetting = {
            ...setting,
            [method]: !setting[method],
        }

        setSetting(updatedSetting)

        if (save) {
            save({
                settings: {
                    ...(record?.settings ? record.settings : {}),
                    mail: updatedSetting.mail,
                    sms: updatedSetting.sms,
                },
            })
        }
    }

    useEffect(() => {
        if (record.settings) {
            setSetting({
                mail: record?.settings?.mail || true,
                sms: record?.settings?.sms || false,
            })
        }
    }, [record])

    if (isLoading) return null

    return (
        <SettingsTemplatesStyles.Wrapper>
            {Object.keys(setting).map((method: string) => (
                <SettingsTemplatesStyles.Content key={method}>
                    <SettingsTemplatesStyles.Text>
                        {t(`electionTypeScreen.common.${method}`)}
                    </SettingsTemplatesStyles.Text>

                    <Switch
                        disabled={true}
                        checked={setting?.[method] || false}
                        onChange={() => handleToggle(method)}
                    />
                </SettingsTemplatesStyles.Content>
            ))}
        </SettingsTemplatesStyles.Wrapper>
    )
}
