import React, {useEffect, useState} from "react"

import styled from "@emotion/styled"

import {Switch} from "@mui/material"
import {useEditController} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useTranslation} from "react-i18next"

const SettingsLanguagesStyles = {
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

export const SettingsLanguages: React.FC<void> = () => {
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const {record, save, isLoading} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })

    const [setting, setSetting] = useState<any>({
        english: record?.settings?.english || true,
        spanish: record?.settings?.spanish || false,
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
                    english: updatedSetting.english,
                    spanish: updatedSetting.spanish,
                },
            })
        }
    }

    useEffect(() => {
        if (record.settings) {
            setSetting({
                english: record?.settings?.english || true,
                spanish: record?.settings?.spanish || false,
            })
        }
    }, [record])

    if (isLoading) return null

    return (
        <SettingsLanguagesStyles.Wrapper>
            {Object.keys(setting).map((method: string) => (
                <SettingsLanguagesStyles.Content key={method}>
                    <SettingsLanguagesStyles.Text>
                        {t(`electionTypeScreen.common.${method}`)}
                    </SettingsLanguagesStyles.Text>

                    <Switch
                        disabled={true}
                        checked={setting?.[method] || false}
                        onChange={() => handleToggle(method)}
                    />
                </SettingsLanguagesStyles.Content>
            ))}
        </SettingsLanguagesStyles.Wrapper>
    )
}
