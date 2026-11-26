// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useId} from "react"
import {useTranslation} from "react-i18next"
import {
    Box,
    Button,
    FormControl,
    FormControlLabel,
    FormHelperText,
    FormLabel,
    IconButton,
    MenuItem,
    Radio,
    RadioGroup,
    TextField,
    Typography,
} from "@mui/material"
import AddIcon from "@mui/icons-material/Add"
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline"
import {ETemplateButtonType, ITemplateButton, TEMPLATE_BUTTON_CODE} from "@/types/messaging"

/** Whether a template sends the provider's default buttons or sets its own. */
export enum ETemplateButtonsChoice {
    DEFAULT = "DEFAULT",
    CUSTOM = "CUSTOM",
}

export interface ITemplateButtonsInputProps {
    /** The template's own buttons; absent or `null`: the defaults. */
    buttons: ITemplateButton[] | null | undefined
    /** What the provider sends for the template's purpose when it sets none. */
    defaults: ITemplateButton[]
    disabled: boolean
    /** `null` goes back to the defaults. */
    onChange: (buttons: ITemplateButton[] | null) => void
}

/**
 * The values an approved template's buttons are sent with, in its button
 * order: the provider's defaults for the purpose, or the template's own.
 */
export const TemplateButtonsInput: React.FC<ITemplateButtonsInputProps> = ({
    buttons,
    defaults,
    disabled,
    onChange,
}) => {
    const {t} = useTranslation()
    const id = useId()
    const choice = buttons ? ETemplateButtonsChoice.CUSTOM : ETemplateButtonsChoice.DEFAULT
    const describe = (list: ITemplateButton[]) =>
        list.length
            ? list
                  .map(
                      (button) =>
                          `${t(`messaging.templateButtons.types.${button.type}`)}: ${button.parameter}`
                  )
                  .join(" · ")
            : t("messaging.templateButtons.none")
    const update = (index: number, patch: Partial<ITemplateButton>) =>
        onChange((buttons ?? []).map((button, i) => (i === index ? {...button, ...patch} : button)))
    return (
        <FormControl
            disabled={disabled}
            sx={{display: "flex", flexDirection: "column", gap: 1, containerType: "inline-size"}}
        >
            <FormLabel id="template-buttons-choice">
                <Typography variant="body2" component="span" color="text.primary">
                    {t("messaging.templateButtons.title")}
                </Typography>
            </FormLabel>
            <Typography variant="caption" color="text.secondary">
                {t("messaging.templateButtons.help", {code: TEMPLATE_BUTTON_CODE})}
            </Typography>
            <RadioGroup
                row
                aria-labelledby="template-buttons-choice"
                value={choice}
                onChange={(event) =>
                    onChange(
                        event.target.value === ETemplateButtonsChoice.CUSTOM
                            ? defaults.map((button) => ({...button}))
                            : null
                    )
                }
            >
                {Object.values(ETemplateButtonsChoice).map((value) => (
                    <FormControlLabel
                        key={value}
                        value={value}
                        control={<Radio size="small" />}
                        label={t(`messaging.templateButtons.choice.${value}`)}
                    />
                ))}
            </RadioGroup>
            {choice === ETemplateButtonsChoice.DEFAULT ? (
                <Typography variant="body2" color="text.secondary">
                    {describe(defaults)}
                </Typography>
            ) : (buttons ?? []).length === 0 ? (
                <Typography variant="body2" color="text.secondary">
                    {t("messaging.templateButtons.none")}
                </Typography>
            ) : null}
            {(buttons ?? []).map((button, index) => {
                const blank = !button.parameter.trim()
                const helperId = `${id}-value-${index}-helper`
                return (
                    <Box
                        key={index}
                        sx={{
                            "display": "grid",
                            "gridTemplateColumns": "minmax(0, 1fr) auto",
                            "gridTemplateAreas": '"type ." "value remove" "helper ."',
                            "columnGap": 1,
                            "alignItems": "start",
                            "@container (min-width: 440px)": {
                                gridTemplateColumns: "minmax(0, 1fr) minmax(0, 1.4fr) auto",
                                gridTemplateAreas: '"type value remove" ". helper ."',
                            },
                        }}
                    >
                        <TextField
                            select
                            size="small"
                            label={t("messaging.templateButtons.type")}
                            value={button.type}
                            disabled={disabled}
                            onChange={(event) =>
                                update(index, {type: event.target.value as ETemplateButtonType})
                            }
                            fullWidth
                            sx={{
                                "gridArea": "type",
                                "mb": 1,
                                "@container (min-width: 440px)": {mb: 0},
                            }}
                        >
                            {Object.values(ETemplateButtonType).map((type) => (
                                <MenuItem key={type} value={type}>
                                    {t(`messaging.templateButtons.types.${type}`)}
                                </MenuItem>
                            ))}
                        </TextField>
                        <TextField
                            size="small"
                            label={t("messaging.templateButtons.parameter")}
                            value={button.parameter}
                            disabled={disabled}
                            error={blank}
                            slotProps={{
                                htmlInput: {"aria-describedby": blank ? helperId : undefined},
                            }}
                            onChange={(event) => update(index, {parameter: event.target.value})}
                            fullWidth
                            sx={{gridArea: "value"}}
                        />
                        {!disabled && (
                            <IconButton
                                aria-label={t("messaging.templateButtons.remove")}
                                sx={{gridArea: "remove", alignSelf: "center"}}
                                onClick={() =>
                                    onChange((buttons ?? []).filter((_, i) => i !== index))
                                }
                            >
                                <DeleteOutlineIcon />
                            </IconButton>
                        )}
                        {blank && (
                            <FormHelperText
                                id={helperId}
                                variant="outlined"
                                error
                                sx={{gridArea: "helper"}}
                            >
                                {t("messaging.templateButtons.required")}
                            </FormHelperText>
                        )}
                    </Box>
                )
            })}
            {choice === ETemplateButtonsChoice.CUSTOM && !disabled ? (
                <Box>
                    <Button
                        size="small"
                        startIcon={<AddIcon />}
                        onClick={() =>
                            onChange([
                                ...(buttons ?? []),
                                {type: ETemplateButtonType.QUICK_REPLY, parameter: ""},
                            ])
                        }
                    >
                        {t("messaging.templateButtons.add")}
                    </Button>
                </Box>
            ) : null}
        </FormControl>
    )
}
