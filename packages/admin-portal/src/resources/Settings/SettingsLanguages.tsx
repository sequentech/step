// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {styled} from "@mui/material/styles"
import {Switch, Typography, Select, MenuItem, InputLabel, FormControl} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useEditController} from "react-admin"

import {useTenantStore} from "@/providers/TenantContextProvider"
import {
    ELanguageDetectionPolicy,
    ILanguageConf,
    ITenantSettings,
    getDefaultLanguageDetectionPolicy,
    getLanguages,
} from "@sequentech/ui-core"

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
        language_detection_policy: getDefaultLanguageDetectionPolicy(),
    }

    const [languageConf, setLanguageConf] = useState<ILanguageConf>(
        (record?.settings as ITenantSettings | undefined)?.language_conf ?? defaultLanguageConf
    )

    const [defaultLanguage, setDefaultLanguage] = useState<string>(
        languageConf.default_language_code ?? "en"
    )

    const [languageDetectionPolicy, setLanguageDetectionPolicy] =
        useState<ELanguageDetectionPolicy>(
            languageConf.language_detection_policy ?? getDefaultLanguageDetectionPolicy()
        )

    const checkIncludesLang = (lang: string) =>
        languageConf.enabled_language_codes?.includes(lang) ?? false

    const enabledLanguagesList = listLangs.filter((lang: string) =>
        languageConf.enabled_language_codes?.includes(lang)
    )

    const onDefaultLanguageChange = (lang: string) => {
        setDefaultLanguage(lang)
        const updatedLanguageConf = {
            ...languageConf,
            default_language_code: lang,
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

    const onLanguageDetectionPolicyChange = (policy: ELanguageDetectionPolicy) => {
        setLanguageDetectionPolicy(policy)
        const updatedLanguageConf = {
            ...languageConf,
            language_detection_policy: policy,
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

    const languageDetectionPolicyOptions = Object.values(ELanguageDetectionPolicy).map((value) => ({
        id: value,
        name: t(`electionEventScreen.field.languageDetectionPolicy.options.${value}`),
    }))

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
            <FormControl sx={{width: "30%"}} key="select-language">
                <InputLabel id="select-language">{t("settings.languages.default")}</InputLabel>
                <Select
                    labelId="select-language"
                    label={t("settings.languages.default")}
                    value={defaultLanguage}
                    onChange={(event) => onDefaultLanguageChange(event.target.value)}
                    fullWidth
                >
                    {enabledLanguagesList.map((lang: string) => (
                        <MenuItem key={lang} value={lang}>
                            {t("language", {lng: lang})}
                        </MenuItem>
                    ))}
                </Select>
            </FormControl>

            <FormControl sx={{width: "30%"}} key="select-language-detection-policy">
                <InputLabel id="select-language-detection-policy">
                    {t("electionEventScreen.field.languageDetectionPolicy.policyLabel")}
                </InputLabel>
                <Select
                    labelId="select-language-detection-policy"
                    fullWidth
                    label={t("electionEventScreen.field.languageDetectionPolicy.policyLabel")}
                    value={languageDetectionPolicy}
                    onChange={(event) =>
                        onLanguageDetectionPolicyChange(
                            event.target.value as ELanguageDetectionPolicy
                        )
                    }
                >
                    {languageDetectionPolicyOptions.map((option) => (
                        <MenuItem key={option.id} value={option.id}>
                            {option.name}
                        </MenuItem>
                    ))}
                </Select>
            </FormControl>
        </SettingsLanguagesStyles.Wrapper>
    )
}
