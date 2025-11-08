// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import styled from "@emotion/styled"
import {faCloudArrowUp} from "@fortawesome/free-solid-svg-icons"
import {Box, Typography} from "@mui/material"
import {Icon, theme} from "@sequentech/ui-essentials"
import React from "react"
import {FileInput} from "react-admin"
import {JsonField} from "react-admin-json-view"
import {useTranslation} from "react-i18next"

interface FileJsonInputProps {
    parsedValue: any
    fileSource: string
    jsonSource: string
}

const DragFileElement = styled(Box)`
    display: flex;
    flex-direction: column;
    width: 100%;
    margin-bottom: 1rem;
    padding: 1rem;
    border-radius: 1rem;
    border: 2px dashed ${theme.palette.grey[500]};
    background-color: ${theme.palette.lightBackground};
    & .RaFileInput-dropZone {
        margin-top: 1rem;
        height: 80px;
        border-radius: 1rem;
    }
`

export const FileJsonInput: React.FC<FileJsonInputProps> = (props) => {
    const {parsedValue, fileSource, jsonSource} = props

    const {t} = useTranslation()

    const getJsonText = (value: any, parsedValue: any) => {
        var url = URL.createObjectURL(value) //Create Object URL
        var xhr = new XMLHttpRequest()
        xhr.open("GET", url, false) //Synchronous XMLHttpRequest on Object URL
        xhr.overrideMimeType("text/plain; charset=x-user-defined") //Override MIME Type to prevent UTF-8 related errors
        xhr.send()
        URL.revokeObjectURL(url)
        var returnText = ""
        for (var i = 0; i < xhr.responseText.length; i++) {
            returnText += String.fromCharCode(xhr.responseText.charCodeAt(i) & 0xff)
        } //remove higher byte
        return {
            ...parsedValue.configuration,
            ...(JSON.parse(returnText as string) || {}),
        }
    }

    return (
        <Box sx={{padding: "1rem 0"}}>
            <DragFileElement>
                <Icon variant="info" icon={faCloudArrowUp as any} fontSize="50px" />
                <FileInput
                    label={false}
                    source={fileSource}
                    accept={"application/json"}
                    parse={(value) => {
                        return getJsonText(value, parsedValue)
                    }}
                    sx={{backgroundColor: "transparent"}}
                >
                    <Box
                        sx={{
                            display: "none",
                        }}
                    />
                </FileInput>
            </DragFileElement>
            <Typography
                variant="body1"
                component="span"
                sx={{fontWeight: "bold", margin: 0, display: {xs: "none", sm: "block"}}}
            >
                {t("common.label.json")}
            </Typography>
            <JsonField
                source={jsonSource}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
        </Box>
    )
}

export default FileJsonInput
