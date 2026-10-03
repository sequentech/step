// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useId, useLayoutEffect, useMemo, useState} from "react"
import AccessibilityNewIcon from "@mui/icons-material/AccessibilityNew"
import {
    Box,
    Button,
    FormControl,
    FormControlLabel,
    FormLabel,
    GlobalStyles,
    Radio,
    RadioGroup,
    Typography,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    EAccessibilityContrast,
    EAccessibilityMotion,
    EAccessibilityTextSize,
    EAccessibilityTextSpacing,
    IAccessibilitySettings,
    applyAccessibilitySettings,
    clearAccessibilitySettings,
    getSystemAccessibilitySettings,
    readStoredAccessibilitySettings,
    resolveAccessibilitySettings,
    storeAccessibilitySettings,
} from "@sequentech/ui-core"
import Dialog from "../Dialog/Dialog"
import VisuallyHidden from "../VisuallyHidden/VisuallyHidden"
import {accessibilityStyles} from "./accessibilityStyles"

const SETTING_VALUES: {[K in keyof IAccessibilitySettings]: IAccessibilitySettings[K][]} = {
    textSize: Object.values(EAccessibilityTextSize),
    contrast: Object.values(EAccessibilityContrast),
    textSpacing: Object.values(EAccessibilityTextSpacing),
    motion: Object.values(EAccessibilityMotion),
}

const SETTING_NAMES = Object.keys(SETTING_VALUES) as (keyof IAccessibilitySettings)[]

/**
 * The voter's display settings: text size, contrast, text spacing and motion.
 *
 * Mounting it applies what the voter chose earlier, or what their operating system asks for;
 * unmounting it removes the settings, so a portal that does not render it is untouched.
 */
const AccessibilityMenu: React.FC = () => {
    const {t} = useTranslation()
    const [open, setOpen] = useState(false)
    const [chosen, setChosen] = useState<Partial<IAccessibilitySettings>>(
        readStoredAccessibilitySettings
    )
    const [announcement, setAnnouncement] = useState("")
    const groupId = useId()

    const settings = useMemo(
        () => resolveAccessibilitySettings(chosen, getSystemAccessibilitySettings()),
        [chosen]
    )

    useLayoutEffect(() => {
        applyAccessibilitySettings(settings)
    }, [settings])

    useEffect(() => clearAccessibilitySettings, [])

    const choose = (name: keyof IAccessibilitySettings, value: string) => {
        const next = {...chosen, [name]: value}
        setChosen(next)
        storeAccessibilitySettings(next)
        setAnnouncement(
            t("accessibility.applied", {
                setting: t(`accessibility.${name}.label`),
                value: t(`accessibility.${name}.${value}`),
            })
        )
    }

    const reset = () => {
        setChosen({})
        storeAccessibilitySettings({})
        setAnnouncement(t("accessibility.resetDone"))
    }

    return (
        <Box className="accessibility-settings">
            <GlobalStyles styles={accessibilityStyles} />
            <Button
                className="accessibility-settings-button"
                variant="actionbar"
                aria-haspopup="dialog"
                aria-label={t("accessibility.button")}
                onClick={() => {
                    setAnnouncement("")
                    setOpen(true)
                }}
                sx={{gap: "10px"}}
            >
                <AccessibilityNewIcon className="accessibility-settings-icon" aria-hidden />
                <Box
                    className="accessibility-settings-label"
                    component="span"
                    sx={{display: {xs: "none", md: "block"}}}
                >
                    {t("accessibility.button")}
                </Box>
            </Button>
            <Dialog
                className="accessibility-settings-dialog"
                variant="info"
                open={open}
                handleClose={() => setOpen(false)}
                title={t("accessibility.title")}
                ok={t("accessibility.close")}
                maxWidth="sm"
                fullWidth
            >
                <Typography className="accessibility-settings-description" component="p">
                    {t("accessibility.description")}
                </Typography>
                {SETTING_NAMES.map((name) => (
                    <FormControl
                        className={`accessibility-setting accessibility-setting-${name}`}
                        key={name}
                        component="fieldset"
                        sx={{display: "block", marginBottom: "16px"}}
                    >
                        <FormLabel
                            className="accessibility-setting-label"
                            component="legend"
                            id={`${groupId}-${name}`}
                            sx={{fontWeight: "bold"}}
                        >
                            {t(`accessibility.${name}.label`)}
                        </FormLabel>
                        <RadioGroup
                            row
                            aria-labelledby={`${groupId}-${name}`}
                            name={`accessibility-${name}`}
                            value={settings[name]}
                            onChange={(_event, value) => choose(name, value)}
                        >
                            {SETTING_VALUES[name].map((value) => (
                                <FormControlLabel
                                    className="accessibility-setting-option"
                                    key={value}
                                    value={value}
                                    control={<Radio />}
                                    label={t(`accessibility.${name}.${value}`)}
                                />
                            ))}
                        </RadioGroup>
                    </FormControl>
                ))}
                <Button
                    className="accessibility-settings-reset"
                    variant="secondary"
                    onClick={reset}
                >
                    {t("accessibility.reset")}
                </Button>
                <VisuallyHidden className="accessibility-settings-status" role="status">
                    {announcement}
                </VisuallyHidden>
            </Dialog>
        </Box>
    )
}

export default AccessibilityMenu
