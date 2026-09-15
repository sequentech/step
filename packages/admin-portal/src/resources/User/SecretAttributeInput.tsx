// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Button, IconButton, InputAdornment, Stack, Tooltip} from "@mui/material"
import VisibilityOffOutlinedIcon from "@mui/icons-material/VisibilityOffOutlined"
import VisibilityOutlinedIcon from "@mui/icons-material/VisibilityOutlined"
import ClearIcon from "@mui/icons-material/Clear"
import {FormStyles} from "@/components/styles/FormStyles"

export interface SecretAttributeInputProps {
    label: string
    values: string[]
    stored: boolean
    editable: boolean
    multivalued: boolean
    canReveal: boolean
    revealed: boolean
    revealing: boolean
    required: boolean
    error?: boolean
    helperText?: string
    labels: {reveal: string; hide: string; clear: string; add: string; remove: string}
    onChange: (values: string[]) => void
    onReveal: () => void
}

export const SecretAttributeInput: React.FC<SecretAttributeInputProps> = ({
    label,
    values,
    stored,
    editable,
    multivalued,
    canReveal,
    revealed,
    revealing,
    required,
    error,
    helperText,
    labels,
    onChange,
    onReveal,
}) => {
    const displayedValues = values.length ? values : [""]
    const visibilityLabel = revealed ? labels.hide : labels.reveal
    const canClear = editable && (stored || values.length > 0)
    return (
        <Stack spacing={1} sx={{width: "100%"}}>
            {displayedValues.map((value, index) => (
                <Box key={index} sx={{display: "flex", gap: 1, alignItems: "center"}}>
                    <FormStyles.TextField
                        type={revealed ? "text" : "password"}
                        label={label}
                        value={value}
                        placeholder={stored && !value ? "••••••••" : undefined}
                        onChange={(event: React.ChangeEvent<HTMLInputElement>) => {
                            const next = [...displayedValues]
                            next[index] = event.target.value
                            onChange(next)
                        }}
                        disabled={revealing}
                        error={error}
                        helperText={index === 0 ? helperText : undefined}
                        required={required && index === 0}
                        fullWidth
                        slotProps={{
                            inputLabel: stored && !value ? {shrink: true} : undefined,
                            input: {
                                readOnly: !editable,
                                endAdornment:
                                    index === 0 && (canReveal || canClear) ? (
                                        <InputAdornment position="end">
                                            {canReveal && (
                                                <Tooltip title={visibilityLabel}>
                                                    <span>
                                                        <IconButton
                                                            type="button"
                                                            size="small"
                                                            aria-label={visibilityLabel}
                                                            aria-busy={revealing}
                                                            disabled={revealing}
                                                            onClick={onReveal}
                                                        >
                                                            {revealed ? (
                                                                <VisibilityOffOutlinedIcon fontSize="small" />
                                                            ) : (
                                                                <VisibilityOutlinedIcon fontSize="small" />
                                                            )}
                                                        </IconButton>
                                                    </span>
                                                </Tooltip>
                                            )}
                                            {canClear && (
                                                <Tooltip title={labels.clear}>
                                                    <span>
                                                        <IconButton
                                                            type="button"
                                                            edge="end"
                                                            size="small"
                                                            aria-label={labels.clear}
                                                            disabled={revealing}
                                                            onClick={() => onChange([])}
                                                        >
                                                            <ClearIcon fontSize="small" />
                                                        </IconButton>
                                                    </span>
                                                </Tooltip>
                                            )}
                                        </InputAdornment>
                                    ) : undefined,
                            },
                        }}
                    />
                    {editable && multivalued && displayedValues.length > 1 && (
                        <Button
                            type="button"
                            disabled={revealing}
                            onClick={() => onChange(values.filter((_, i) => i !== index))}
                        >
                            {labels.remove}
                        </Button>
                    )}
                </Box>
            ))}
            {editable && multivalued && (
                <Stack direction="row" spacing={1}>
                    <Button
                        type="button"
                        disabled={revealing}
                        onClick={() => onChange([...displayedValues, ""])}
                    >
                        {labels.add}
                    </Button>
                </Stack>
            )}
        </Stack>
    )
}
