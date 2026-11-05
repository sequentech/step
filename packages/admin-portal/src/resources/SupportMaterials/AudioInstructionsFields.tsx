// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {MenuItem, TextField} from "@mui/material"
import {useTranslation} from "react-i18next"
import {EAudioInstructionsScreen} from "@sequentech/ui-core"

export interface AudioInstructionsValue {
    [key: string]: string
    screen: string
    language: string
}

export interface AudioInstructionsFieldsProps {
    /** The uploaded file's media type. The fields are only offered for audio. */
    kind?: string | null
    /** The event's enabled language codes. */
    languages: string[]
    value?: Partial<AudioInstructionsValue>
    /** `undefined` when the file is no longer the instructions for any screen. */
    onChange: (value: AudioInstructionsValue | undefined) => void
}

const NO_SCREEN = ""

/** Replaces or removes the assignment in a support material's `data`. */
export const withAudioInstructions = <T extends object>(
    data: T | null,
    value: AudioInstructionsValue | undefined
): T => {
    const {audio_instructions: _previous, ...rest} = (data ?? {}) as T & {
        audio_instructions?: unknown
    }
    return (value ? {...rest, audio_instructions: value} : rest) as T
}

/**
 * Marks an uploaded audio file as the Voting Portal's spoken instructions for one screen in
 * one language.
 */
export const AudioInstructionsFields: React.FC<AudioInstructionsFieldsProps> = ({
    kind,
    languages,
    value,
    onChange,
}) => {
    const {t} = useTranslation()

    if (!kind?.includes("audio")) {
        return null
    }

    const screen = value?.screen ?? NO_SCREEN
    const language = value?.language ?? languages[0] ?? ""

    return (
        <>
            <TextField
                select
                className="audio-instructions-screen"
                label={String(t("materials.audioInstructions.screenLabel"))}
                helperText={String(t("materials.audioInstructions.helperText"))}
                size="small"
                fullWidth
                value={screen}
                onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
                    onChange(
                        event.target.value === NO_SCREEN
                            ? undefined
                            : {screen: event.target.value, language}
                    )
                }
            >
                <MenuItem value={NO_SCREEN}>{t("materials.audioInstructions.none")}</MenuItem>
                {Object.values(EAudioInstructionsScreen).map((option) => (
                    <MenuItem key={option} value={option}>
                        {t(`materials.audioInstructions.screens.${option}`)}
                    </MenuItem>
                ))}
            </TextField>
            {screen === NO_SCREEN ? null : (
                <TextField
                    select
                    className="audio-instructions-language"
                    label={String(t("materials.audioInstructions.languageLabel"))}
                    size="small"
                    fullWidth
                    sx={{marginTop: "1rem"}}
                    value={language}
                    onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
                        onChange({screen, language: event.target.value})
                    }
                >
                    {languages.map((code) => (
                        <MenuItem key={code} value={code}>
                            {t(`common.language.${code}`)}
                        </MenuItem>
                    ))}
                </TextField>
            )}
        </>
    )
}
