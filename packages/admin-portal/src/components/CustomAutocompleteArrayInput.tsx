// SPDX-FileCopyrightText: 2025 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, useRef, ChangeEvent, KeyboardEvent} from "react"
import {Autocomplete, TextField, Chip} from "@mui/material"

interface Choice {
    name: string
}

interface CustomAutocompleteArrayInputProps {
    label: string
    defaultValue?: string[]
    onChange: (value: string[]) => void
    onCreate?: (value: string) => void
    choices: Choice[]
    disabled?: boolean
}

export const CustomAutocompleteArrayInput: React.FC<CustomAutocompleteArrayInputProps> = ({
    label,
    defaultValue,
    onChange,
    onCreate,
    choices,
    disabled,
}) => {
    const [inputValue, setInputValue] = useState<string>("")
    const [selectedValues, setSelectedValues] = useState<string[]>(defaultValue || [])
    const [updatedChoices, setUpdatedChoices] = useState<Choice[]>(choices)
    const inputRef = useRef<HTMLInputElement>(null)

    const handleInputChange = (event: ChangeEvent<{}>, newInputValue: string) => {
        setInputValue(newInputValue)
    }

    const handleChange = (event: ChangeEvent<{}>, newValue: string[]) => {
        const newLabels = newValue.flatMap((value) => value.split(/\s+/))
        const uniqueLabels = Array.from(new Set(newLabels))
        setSelectedValues(uniqueLabels)
        onChange(uniqueLabels)
    }

    const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        if (event.key === "Enter" && inputValue) {
            event.preventDefault()
            const newLabels = inputValue.trim().split(/\s+/)

            const updatedValues = [...selectedValues]
            const newChoices = [...updatedChoices]

            newLabels.forEach((newLabel) => {
                if (newLabel && !updatedValues.includes(newLabel)) {
                    updatedValues.push(newLabel)
                    newChoices.push({name: newLabel})
                }
            })

            setSelectedValues(updatedValues)
            setUpdatedChoices(newChoices)
            setInputValue("")
            inputRef?.current?.focus()

            // Call onCreate for each new label after updating state
            newLabels.forEach((newLabel) => {
                if (onCreate && newLabel && !selectedValues.includes(newLabel)) {
                    onCreate(newLabel)
                }
            })

            // Ensure all labels are saved correctly
            onChange(updatedValues)
        }
    }

    return (
        <Autocomplete
            multiple
            freeSolo
            fullWidth
            value={selectedValues}
            onChange={handleChange}
            inputValue={inputValue}
            onInputChange={handleInputChange}
            options={updatedChoices.map((choice) => choice.name)}
            renderTags={(value, getTagProps) =>
                value.map((option, index) => (
                    <Chip
                        variant="outlined"
                        label={option}
                        {...getTagProps({index})}
                        key={option}
                    />
                ))
            }
            renderInput={(params) => (
                <TextField
                    {...params}
                    variant="outlined"
                    label={label}
                    onKeyDown={handleKeyDown}
                    inputRef={inputRef}
                    disabled={disabled}
                />
            )}
        />
    )
}
