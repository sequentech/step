// SPDX-FileCopyrightText: 2025 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, useRef, ChangeEvent, KeyboardEvent} from "react"
import {Autocomplete, TextField, Chip} from "@mui/material"
import {create} from "@mui/material/styles/createTransitions"

export type Choice = {
    id: string
    name: string
}

interface CustomAutocompleteArrayInputProps {
    label: string
    defaultValue?: string[]
    onChange: (value: string[]) => void
    onCreate?: (value: string) => void
    choices?: Choice[] | undefined
    disabled?: boolean
    prefixCreate?: string
}

const CustomAutocompleteArrayInput: React.FC<CustomAutocompleteArrayInputProps> = ({
    label,
    defaultValue,
    onChange,
    onCreate,
    choices,
    disabled,
    prefixCreate = "Create",
}) => {
    const [inputValue, setInputValue] = useState<string>("")
    const [selectedValues, setSelectedValues] = useState<string[]>(defaultValue || [])
    const [updatedChoices, setUpdatedChoices] = useState<Choice[] | undefined>(choices)
    const inputRef = useRef<HTMLInputElement>(null)

    const handleInputChange = (event: ChangeEvent<{}>, newInputValue: string) => {
        setInputValue(newInputValue)
    }

    const handleChange = (event: ChangeEvent<{}>, newValue: string[]) => {
        const createValue = newValue.filter(
            (value) => value === `${prefixCreate || "Create"} ${inputValue}`
        )

        if (createValue && createValue.length > 0) {
            handleCreateOption()
        } else {
            const filteredValue = newValue.filter(
                (value) => value !== `${prefixCreate || "Create"} ${inputValue}`
            )
            const newLabels = filteredValue.flatMap((value) => value.split(/\s+/))
            const uniqueLabels = Array.from(new Set(newLabels))
            setSelectedValues(uniqueLabels)
            onChange(uniqueLabels)
        }
    }

    const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        if (event.key === "Enter" && inputValue) {
            event.preventDefault()
            handleCreateOption()
        }
    }

    const handleCreateOption = () => {
        if (inputValue.trim()) {
            // Ensure inputValue is not blank
            const newLabels = inputValue.trim().split(/\s+/)

            const updatedValues = [...selectedValues]
            const newChoices = [...(updatedChoices || [])]

            newLabels.forEach((newLabel) => {
                if (
                    newLabel &&
                    !updatedValues.includes(newLabel) &&
                    !newChoices.some((choice) => choice.name === newLabel)
                ) {
                    updatedValues.push(newLabel)
                    newChoices.push({id: newLabel, name: newLabel})
                }
            })

            setSelectedValues(updatedValues)
            setUpdatedChoices(newChoices)
            setInputValue("") // Clear inputValue after creating labels
            inputRef?.current?.focus()

            newLabels.forEach((newLabel) => {
                if (onCreate && newLabel && !selectedValues.includes(newLabel)) {
                    onCreate(newLabel)
                }
            })

            onChange(updatedValues)
        }
    }

    const handleSelectOption = (option: string) => {
        if (option === `${prefixCreate || "Create"} ${inputValue}`) {
            handleCreateOption()
        } else if (option && option.trim()) {
            const updatedValues = selectedValues.includes(option)
                ? selectedValues.filter((value) => value !== option)
                : [...selectedValues, option]
            setSelectedValues(updatedValues)
            onChange(updatedValues)
        }
    }

    return (
        <>
            <Autocomplete
                multiple
                freeSolo
                fullWidth
                disabled={disabled}
                value={selectedValues}
                onChange={handleChange}
                inputValue={inputValue}
                onInputChange={handleInputChange}
                options={[
                    ...(inputValue.trim() ? [`${prefixCreate || "Create"} ${inputValue}`] : []),
                    ...(updatedChoices || []).map((choice) => choice.name),
                ]}
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
                onClose={() => setInputValue("")} // Clear input on dropdown close
                onSelect={(event) => {
                    const option = (event.target as HTMLElement).textContent || ""

                    if (option !== "") {
                        handleSelectOption(option)
                    }
                }}
            />
        </>
    )
}

export default CustomAutocompleteArrayInput
