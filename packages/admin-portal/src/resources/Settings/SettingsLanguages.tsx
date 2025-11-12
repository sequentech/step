// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {styled} from "@mui/material/styles"
import {Switch, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useEditController} from "react-admin"

import {useTenantStore} from "@/providers/TenantContextProvider"
import {ILanguageConf, ITenantSettings, getLanguages} from "@sequentech/ui-core"

const SettingsLanguagesStyles = {
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

export const SettingsLanguages: React.FC<void> = () => {
    const [tenantId] = useTenantStore()
    const {t, i18n} = useTranslation()
    const listLangs = getLanguages(i18n)
    const {record, save, isLoading} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })

    const defaultLanguageConf: ILanguageConf = {
        enabled_language_codes: ["en"],
        default_language_code: "en",
    }

    const [languageConf, setLanguageConf] = useState<ILanguageConf>(
        (record?.settings as ITenantSettings | undefined)?.language_conf ?? defaultLanguageConf
    )

    const checkIncludesLang = (lang: string) =>
        languageConf.enabled_language_codes?.includes(lang) ?? false

    const handleToggle = (lang: string) => {
        const includesLang = checkIncludesLang(lang)

        const currentLangs = languageConf.enabled_language_codes ?? []

        const enabledLangs = includesLang
            ? currentLangs.filter((code) => code !== lang)
            : [...currentLangs, lang]

        const updatedLanguageConf = {
            ...languageConf,
            enabled_language_codes: enabledLangs,
        }

        setLanguageConf(updatedLanguageConf)

        if (save) {
            save({
                settings: {
                    ...((record?.settings as ITenantSettings | undefined) ?? {}),
                    language_conf: updatedLanguageConf,
                },
            })
        }
    }

    useEffect(() => {
        if (record.settings) {
            setLanguageConf(
                (record?.settings as ITenantSettings | undefined)?.language_conf ??
                    defaultLanguageConf
            )
        }
    }, [record])

    if (isLoading) return null

    return (
        <SettingsLanguagesStyles.Wrapper>
            <Typography variant="body2" paragraph>
                {t("generalSettingsScreen.body")}
            </Typography>
            {listLangs.map((lang: string) => (
                <SettingsLanguagesStyles.Content key={lang}>
                    <SettingsLanguagesStyles.Text>
                        {t("language", {lng: lang})}
                    </SettingsLanguagesStyles.Text>

                    <Switch checked={checkIncludesLang(lang)} onChange={() => handleToggle(lang)} />
                </SettingsLanguagesStyles.Content>
            ))}
        </SettingsLanguagesStyles.Wrapper>
    )
}
