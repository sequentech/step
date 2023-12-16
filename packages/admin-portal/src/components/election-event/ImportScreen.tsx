import {Box, styled, Button, TextField} from "@mui/material"
import {DropFile} from "@sequentech/ui-essentials"
import React from "react"
import {useTranslation} from "react-i18next"

interface ImportScreenProps {
    doImport: (file: FileList | null, sha: string) => void
    doCancel: () => void
}

export const ImportStyles = {
    CancelButton: styled(Button)`
        margin-right: auto;
        background-color: ${({theme}) => theme.palette.grey[100]};
        color: ${({theme}) => theme.palette.brandColor};
    `,

    ImportButton: styled(Button)`
        margin-left: auto;
    `,
}

export const ImportScreen: React.FC<ImportScreenProps> = (props) => {
    const {doCancel, doImport} = props

    const {t} = useTranslation()

    const [shaField, setShaField] = React.useState<string>("")
    const [file, setFile] = React.useState<FileList | null>(null)

    const handleFiles = (files: FileList | null) => {
        console.log("handleFiles(): files:", files);
        setFile(files)
    }

    return (
        <Box sx={{padding: "16px"}}>
            <TextField
                label={t("electionEventScreen.import.sha")}
                size="small"
                value={shaField}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setShaField(e.target.value)}
            />
            <DropFile handleFiles={(files: FileList | null) => handleFiles(files)} />
            <Box
                sx={{
                    display: "flex",
                    flexDirection: "row",
                    justifyContent: "space-between",
                    alignItems: "center",
                    marginTop: "16px",
                }}
            >
                <ImportStyles.CancelButton onClick={doCancel}>
                    {t("electionEventScreen.import.cancel")}
                </ImportStyles.CancelButton>
                <ImportStyles.ImportButton
                    disabled={!file && shaField === ""}
                    onClick={() => doImport(file, shaField)}
                >
                    {t("electionEventScreen.import.import")}
                </ImportStyles.ImportButton>
            </Box>
        </Box>
    )
}
