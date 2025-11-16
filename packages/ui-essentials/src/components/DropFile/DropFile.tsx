// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useRef} from "react"
import Paper from "@mui/material/Paper"
import {faCloudArrowUp} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import CustomDropFile from "../CustomDropFile/CustomDropFile"
import Icon from "../Icon/Icon"
import {Box, Typography} from "@mui/material"
import {theme} from "../../services/theme"

interface DropFileProps {
    handleFiles: (files: FileList) => void | Promise<void>
}

const DropFile: React.FC<DropFileProps> = ({handleFiles}) => {
    const {t} = useTranslation()
    const inputRef = useRef<HTMLInputElement | null>(null)

    // triggers the input when the button is clicked
    // const onButtonClick = () => {
    //     inputRef.current?.click()
    // }

    return (
        <CustomDropFile handleFiles={handleFiles} ref={inputRef}>
            <div
                className="drop-file-container"
                // variant="responsive"
                // sx={{width: "100%", gap: "7px", padding: "16px", backgroundColor: "inherit"}}
                style={{
                    backgroundColor: "inherit",
                    display: "flex",
                    flexDirection: "column",
                    justifyContent: "center",
                    alignItems: "center",
                }}
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
                        // onClick={onButtonClick}
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
            </div>
        </CustomDropFile>
    )
}

DropFile.displayName = "DropFile"

export default DropFile
