// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren, useState, DragEventHandler, ChangeEventHandler} from "react"
import styledEmotion from "@emotion/styled"
import {styled} from "@mui/material/styles"
import Box from "@mui/material/Box"
import {useForwardedRef} from "@sequentech/ui-core"
import {Typography} from "@mui/material"
import theme from "../../services/theme"

const StyledForm = styledEmotion.form`
    height: 16rem;
    width: 100%;
    max-width: 100%;
    text-align: center;
    position: relative;
    margin-bottom: 16px;
`

const StyledInput = styledEmotion.input`
    display: none;
`

// const StyledLabel = styledEmotion(Paper)<{dragActive: boolean}>`
const StyledLabel = styledEmotion.label<{dragActive: boolean}>`
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    border-style: dashed;
    border-color: ${({dragActive, theme}) =>
        dragActive ? theme.palette.customGreen.dark : theme.palette.customGrey.contrastText};
    border-width: 2px;
    &:hover {
        cursor: pointer;
    }
    backgroundColor: ${({dragActive, theme}) =>
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
        const [fileName, setFileName] = useState<string>("")

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
                setFileName(e.dataTransfer.files[0].name)
                handleFiles(e.dataTransfer.files)
            }
        }

        // triggers when file is selected with click
        const handleChange: ChangeEventHandler<HTMLInputElement> = (e) => {
            e.preventDefault()
            if (e.target.files && e.target.files[0]) {
                setFileName(e.target.files[0].name)
                handleFiles(e.target.files)
            }
        }
        // triggers the input when the button is clicked
        const onButtonClick = () => {
            setFileName("")
            if (innerRef.current?.value) {
                innerRef.current.value = ""
            }
            innerRef.current?.click()
        }

        return (
            <>
                <StyledForm
                    onDragEnter={handleDrag}
                    onSubmit={(e) => e.preventDefault()}
                    // className="drop-file-form"
                    className="drop-file-dropzone"
                >
                    <StyledInput
                        className="drop-input-file"
                        ref={innerRef}
                        type="file"
                        onChange={handleChange}
                        data-testid="drop-input-file"
                        aria-label="Drop Input File"
                    />
                    <StyledLabel
                        dragActive={dragActive}
                        onClick={onButtonClick}
                        data-testid="drop-label-file"
                        className="drop-label-file"
                    >
                        {children}
                    </StyledLabel>
                    {dragActive && (
                        <DragFileElement
                            className="drag-file-element"
                            onDragEnter={handleDrag}
                            onDragLeave={handleDrag}
                            onDragOver={handleDrag}
                            onDrop={handleDrop}
                        />
                    )}
                </StyledForm>
                <Typography
                    className="file-name"
                    variant="h6"
                    sx={{
                        fontSize: "16px",
                        margin: "8px 0",
                        height: "24px",
                        color: theme.palette.brandSuccess,
                    }}
                >
                    {fileName}
                </Typography>
            </>
        )
    }
)

CustomDropFile.displayName = "CustomDropFile"

export default CustomDropFile
