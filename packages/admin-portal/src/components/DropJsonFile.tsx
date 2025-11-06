// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useRef} from "react"
import Paper from "@mui/material/Paper"
import {faCloudArrowUp} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import {Box, Typography} from "@mui/material"
import {CustomDropFile, Icon, theme} from "@sequentech/ui-essentials"

interface DropJsonFileProps {
    handleFiles: (files: FileList) => void | Promise<void>
}

const DropJsonFile: React.FC<DropJsonFileProps> = ({handleFiles}) => {
    const {t} = useTranslation()

    const inputRef = useRef<HTMLInputElement | null>(null)

    // triggers the input when the button is clicked
    const onButtonClick = () => {
        inputRef.current?.click()
    }

    return (
        <CustomDropFile handleFiles={handleFiles} ref={inputRef}>
            <Paper
                variant="responsive"
                sx={{width: "100%", gap: "7px", padding: "16px", backgroundColor: "inherit"}}
            >
                <Icon variant="info" icon={faCloudArrowUp} fontSize="50px" />
                <Box
                    sx={{
                        display: "flex",
                        flexDirection: "row",
                        gap: 1,
                        margin: 0,
                    }}
                >
                    <Typography
                        variant="body1"
                        component="span"
                        sx={{fontWeight: "bold", margin: 0, display: {xs: "none", sm: "block"}}}
                    >
                        {t("dragNDrop.firstLine")}
                    </Typography>
                    <Typography
                        variant="body1"
                        component="span"
                        onClick={onButtonClick}
                        data-testid="drop-file-button"
                        sx={{fontWeight: "bold", textDecoration: "underline", margin: 0}}
                    >
                        {t("dragNDrop.browse")}
                    </Typography>
                </Box>
                <Box>
                    <Typography
                        sx={{
                            margin: 0,
                            fontSize: "12px",
                            lineHeight: "28px",
                            color: theme.palette.customGrey.dark,
                        }}
                    >
                        {t("dragNDrop.format")}
                    </Typography>
                </Box>
            </Paper>
        </CustomDropFile>
    )
}

DropJsonFile.displayName = "DropJsonFile"

export default DropJsonFile
