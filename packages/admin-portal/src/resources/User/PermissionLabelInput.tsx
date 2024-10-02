// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {useInput} from "react-admin"
import {TextField, Button, Box, Chip} from "@mui/material"

interface PermissionLabelInputProps {
    source: string
    permissionLabels: string[] | undefined
    handleAddedLabel?: (labels: string[]) => void
}

const PermissionLabelInput: React.FC<PermissionLabelInputProps> = ({
    source,
    permissionLabels,
    handleAddedLabel,
}) => {
    const [inputValue, setInputValue] = useState("")
    const [permissionLabelsArr, setPermissionLabels] = useState<string[]>(permissionLabels || [])

    const handleAddLabel = () => {
        if (inputValue.trim() && !permissionLabelsArr.includes(inputValue.trim())) {
            const newLabels = [...permissionLabelsArr, inputValue.trim()]
            setPermissionLabels(newLabels)
            handleAddedLabel && handleAddedLabel(newLabels)
            setInputValue("")
        }
    }

    return (
        <Box>
            <Box sx={{display: "flex", alignItems: "center", marginBottom: "10px"}}>
                <TextField
                    label="Permission Labels"
                    value={inputValue}
                    onChange={(e) => setInputValue(e.target.value)}
                />
                <Button
                    variant="text"
                    onClick={handleAddLabel}
                    sx={{minWidth: "auto", marginLeft: "10px"}}
                >
                    Add
                </Button>
            </Box>
            {permissionLabelsArr?.map((label, index) => {
                return <Chip label={label} key={index} style={{marginLeft: "4px"}} />
            })}
        </Box>
    )
}

export default PermissionLabelInput
