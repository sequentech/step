// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren, useState, DragEventHandler, ChangeEventHandler} from "react"
import styledEmotion from "@emotion/styled"
import {styled} from "@mui/material/styles"
import Box from "@mui/material/Box"
import Paper from "@mui/material/Paper"
import {useForwardedRef} from "../../utils/ref"

const StyledInput = styledEmotion.input`
    display: none;
`

const BorderContainer = styled(Box)`
    display: flex;
    justify-content: center;
    align-items: center;
`
//  border: 2px dashed ${({dragActive, theme}) => dragActive ? theme.palette.customGreen.dark : theme.palette.grey[500]};
const StyledLabel = styledEmotion(Paper)<{dragActive: boolean}>`
    height: 100%;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    &:hover {
        cursor: pointer;
    }
    background-color: ${({dragActive, theme}) =>
        dragActive ? theme.palette.customGreen.light : theme.palette.lightBackground};
`

const DragFileElement = styled(Box)`
    position: absolute;
    width: 100%;
    height: 100%;
    border-radius: 1rem;
    top: 0px;
    right: 0px;
    bottom: 0px;
    left: 0px;
`

export interface DropFileProps {
    handleFiles: (files: FileList) => void | Promise<void>
}

// based on https://www.codemzy.com/blog/react-drag-drop-file-upload
export const CustomDropFile = React.forwardRef<HTMLInputElement, PropsWithChildren<DropFileProps>>(
    ({handleFiles, children}, inputRef) => {
        const innerRef = useForwardedRef(inputRef)
        const [dragActive, setDragActive] = useState(false)

        // handle drag events
        const handleDrag: DragEventHandler<HTMLElement> = (e) => {
            e.preventDefault()
            e.stopPropagation()
            if (e.type === "dragenter" || e.type === "dragover") {
                setDragActive(true)
            } else if (e.type === "dragleave") {
                setDragActive(false)
            }
        }

        // triggers when file is dropped
        const handleDrop: DragEventHandler<HTMLElement> = (e) => {
            e.preventDefault()
            e.stopPropagation()
            setDragActive(false)
            if (e.dataTransfer.files && e.dataTransfer.files[0]) {
                handleFiles(e.dataTransfer.files)
            }
        }

        // triggers when file is selected with click
        const handleChange: ChangeEventHandler<HTMLInputElement> = (e) => {
            e.preventDefault()
            if (e.target.files && e.target.files[0]) {
                handleFiles(e.target.files)
            }
        }
        // triggers the input when the button is clicked
        const onButtonClick = () => {
            innerRef.current?.click()
        }

        return (
            <Box
                onDragEnter={handleDrag}
                onSubmit={(e) => e.preventDefault()}
                sx={{position: "relative"}}
            >
                <StyledInput
                    ref={innerRef}
                    type="file"
                    onChange={handleChange}
                    data-testid="drop-input-file"
                    aria-label="Drop Input File"
                />
                <BorderContainer>
                    <StyledLabel
                        variant="dashed"
                        dragActive={dragActive}
                        onClick={onButtonClick}
                        data-testid="drop-label-file"
                    >
                        {children}
                    </StyledLabel>
                </BorderContainer>
                {dragActive && (
                    <DragFileElement
                        onDragEnter={handleDrag}
                        onDragLeave={handleDrag}
                        onDragOver={handleDrag}
                        onDrop={handleDrop}
                    />
                )}
            </Box>
        )
    }
)

CustomDropFile.displayName = "CustomDropFile"

export default CustomDropFile
