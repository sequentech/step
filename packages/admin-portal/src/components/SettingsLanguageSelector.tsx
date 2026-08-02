// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Radio, Typography} from "@mui/material"
import {BooleanInput, useInput} from "react-admin"
import {useTranslation} from "react-i18next"

type SettingsLanguageSelectorProps = {
    languageSettings: string[]
    canEdit?: boolean
}

export const SettingsLanguageSelector = ({
    languageSettings,
    canEdit = true,
}: SettingsLanguageSelectorProps) => {
    const {t} = useTranslation()

    const {
        field: {value: defaultLanguage, onChange: onDefaultLanguageChange},
    } = useInput<string>({
        source: "presentation.language_conf.default_language_code",
    })

    return (
        <Box display="flex" flexDirection="column" gap={1}>
            {languageSettings.map((lang) => (
                <Box
                    key={lang}
                    display="grid"
                    gridTemplateColumns="11fr 1fr"
                    alignItems="flex-start"
                    columnGap={4}
                >
                    <BooleanInput
                        disabled={!canEdit}
                        source={`enabled_languages.${lang}`}
                        label={String(t(`common.language.${lang}`))}
                    />

                    <Box display="flex" alignItems="center">
                        <Radio
                            checked={defaultLanguage === lang}
                            onChange={() => onDefaultLanguageChange(lang)}
                            disabled={!canEdit}
                        />
                        <Typography>{String(t("electionScreen.edit.default"))}</Typography>
                    </Box>
                </Box>
            ))}
        </Box>
    )
}
