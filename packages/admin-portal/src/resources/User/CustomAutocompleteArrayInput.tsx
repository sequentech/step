import React, {useState, useRef, ChangeEvent, KeyboardEvent} from "react"
import {Autocomplete, TextField, Chip} from "@mui/material"

interface Choice {
    name: string
}

interface CustomAutocompleteArrayInputProps {
    label: string
    defaultValue?: string[]
    onChange: (value: string[]) => void
    onCreate: (value: string) => void
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
    const inputRef = useRef<HTMLInputElement>(null)

    const handleInputChange = (event: ChangeEvent<{}>, newInputValue: string) => {
        setInputValue(newInputValue)
    }

    const handleChange = (event: ChangeEvent<{}>, newValue: string[]) => {
        setSelectedValues(newValue)
        onChange(newValue)
    }

    const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        if (event.key === "Enter" && inputValue) {
            event.preventDefault()
            const newLabels = inputValue.trim().split(/\s+/)
            const updatedValues = [...selectedValues]

            newLabels.forEach((newLabel) => {
                if (newLabel && !updatedValues.includes(newLabel)) {
                    updatedValues.push(newLabel)
                    onCreate(newLabel)
                }
            })

            setSelectedValues(updatedValues)
            setInputValue("")
            inputRef?.current?.focus()
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
            options={choices.map((choice) => choice.name)}
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
