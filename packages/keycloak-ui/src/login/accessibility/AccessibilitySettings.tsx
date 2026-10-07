// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useId, useLayoutEffect, useMemo, useState} from "react"
import Button from "@mui/material/Button"
import Dialog from "@mui/material/Dialog"
import DialogActions from "@mui/material/DialogActions"
import DialogContent from "@mui/material/DialogContent"
import DialogTitle from "@mui/material/DialogTitle"
import FormControl from "@mui/material/FormControl"
import FormControlLabel from "@mui/material/FormControlLabel"
import FormLabel from "@mui/material/FormLabel"
import Radio from "@mui/material/Radio"
import RadioGroup from "@mui/material/RadioGroup"
import {
    EAccessibilityContrast,
    EAccessibilityMotion,
    EAccessibilityTextSize,
    EAccessibilityTextSpacing,
    type IAccessibilitySettings,
    applyAccessibilitySettings,
    clearAccessibilitySettings,
    getSystemAccessibilitySettings,
    readStoredAccessibilitySettings,
    resolveAccessibilitySettings,
    storeAccessibilitySettings,
} from "../../../../ui-core/src/services/accessibilitySettings"
import {accessibilityStyles} from "../../../../ui-essentials/src/components/AccessibilityMenu/accessibilityStyles"
import {AccessibilityIcon} from "../icons"
import type {AccessibilityCopy} from "./copy"

type SettingName = keyof IAccessibilitySettings

const SETTING_VALUES: Record<SettingName, string[]> = {
    textSize: Object.values(EAccessibilityTextSize),
    contrast: Object.values(EAccessibilityContrast),
    textSpacing: Object.values(EAccessibilityTextSpacing),
    motion: Object.values(EAccessibilityMotion),
}

const SETTING_NAMES = Object.keys(SETTING_VALUES) as SettingName[]

/**
 * The Voting Portal's display settings, on the pages a voter sees before signing in.
 *
 * The choices live in the cookie the portal reads, so they carry over in both directions.
 */
export function AccessibilitySettings({copy}: {copy: AccessibilityCopy}) {
    const [open, setOpen] = useState(false)
    const [chosen, setChosen] = useState<Partial<IAccessibilitySettings>>(
        readStoredAccessibilitySettings
    )
    const [announcement, setAnnouncement] = useState("")
    const id = useId()
    const words = copy.accessibility
    const labels = words as unknown as Record<SettingName, Record<string, string>>

    const settings = useMemo(
        () => resolveAccessibilitySettings(chosen, getSystemAccessibilitySettings()),
        [chosen]
    )

    useLayoutEffect(() => {
        applyAccessibilitySettings(settings)
    }, [settings])

    useEffect(() => clearAccessibilitySettings, [])

    const choose = (name: SettingName, value: string) => {
        const next = {...chosen, [name]: value}
        setChosen(next)
        storeAccessibilitySettings(next)
        setAnnouncement(
            words.applied
                .replace("{{setting}}", labels[name].label)
                .replace("{{value}}", labels[name][value])
        )
    }

    const reset = () => {
        setChosen({})
        storeAccessibilitySettings({})
        setAnnouncement(words.resetDone)
    }

    return (
        <>
            <style>{accessibilityStyles}</style>
            <button
                type="button"
                className="auth-accessibility"
                aria-haspopup="dialog"
                lang={copy.languageTag}
                onClick={() => {
                    setAnnouncement("")
                    setOpen(true)
                }}
            >
                <AccessibilityIcon />
                {words.button}
            </button>
            <Dialog
                className="auth-accessibility-dialog"
                open={open}
                onClose={() => setOpen(false)}
                aria-labelledby={`${id}-title`}
                fullWidth
                maxWidth="xs"
                lang={copy.languageTag}
            >
                <DialogTitle id={`${id}-title`}>{words.title}</DialogTitle>
                <DialogContent>
                    <p className="auth-accessibility-description">{words.description}</p>
                    {SETTING_NAMES.map((name) => (
                        <FormControl
                            key={name}
                            component="fieldset"
                            className="auth-accessibility-setting"
                        >
                            <FormLabel component="legend" id={`${id}-${name}`}>
                                {labels[name].label}
                            </FormLabel>
                            <RadioGroup
                                row
                                aria-labelledby={`${id}-${name}`}
                                name={`accessibility-${name}`}
                                value={settings[name]}
                                onChange={(_event, value) => choose(name, value)}
                            >
                                {SETTING_VALUES[name].map((value) => (
                                    <FormControlLabel
                                        key={value}
                                        value={value}
                                        control={<Radio />}
                                        label={labels[name][value]}
                                    />
                                ))}
                            </RadioGroup>
                        </FormControl>
                    ))}
                    <span className="auth-visually-hidden" role="status">
                        {announcement}
                    </span>
                </DialogContent>
                <DialogActions>
                    <Button variant="outlined" onClick={reset}>
                        {words.reset}
                    </Button>
                    <Button variant="contained" onClick={() => setOpen(false)}>
                        {words.close}
                    </Button>
                </DialogActions>
            </Dialog>
        </>
    )
}
