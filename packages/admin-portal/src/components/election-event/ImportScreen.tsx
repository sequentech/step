import {Box, styled, Button, TextField, CircularProgress} from "@mui/material"
import {DropFile} from "@sequentech/ui-essentials"
import React, {useEffect, useCallback, useRef} from "react"
import {useTranslation} from "react-i18next"

interface ImportScreenProps {
    doImport: (file: FileList | null, sha: string) => void
    doCancel: () => void
    isLoading: boolean
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
    const {doCancel, doImport, isLoading} = props

    const {t} = useTranslation()

    const [shaField, setShaField] = React.useState<string>("")
    // const [fileImport, setFileImport] = React.useState<FileList | null>(null)
    const fileImport = useRef<FileList | null>(null)

    const handleFiles = useCallback((files: FileList | null) => {
        console.log("handleFiles()", files)

        // setFileImport(files)
        // fileImport.current = files
    }, [])

    useEffect(() => {
        console.log("ImportScreen", doCancel, doImport, isLoading)
    }, [])

    useEffect(() => {
        console.log("fileImport", fileImport.current)
    }, [fileImport])

    useEffect(() => {
        console.log("shaField", shaField)
    }, [shaField])

    return (
        <Box sx={{padding: "16px"}}>
            <TextField
                label={t("electionEventScreen.import.sha")}
                size="small"
                value={shaField}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setShaField(e.target.value)}
            />
            <DropFile handleFiles={handleFiles} />

            <Box
                sx={{
                    display: "flex",
                    flexDirection: "row",
                    justifyContent: "center",
                    alignItems: "center",
                    margin: "16px auto",
                    height: "48px",
                }}
            >
                {isLoading ? <CircularProgress /> : null}
            </Box>

            <Box
                sx={{
                    display: "flex",
                    flexDirection: "row",
                    justifyContent: "space-between",
                    alignItems: "center",
                    marginTop: "16px",
                }}
            >
                <ImportStyles.CancelButton onClick={() => doCancel()}>
                    {t("electionEventScreen.import.cancel")}
                </ImportStyles.CancelButton>
                <ImportStyles.ImportButton
                    // disabled={!fileImport.current || shaField === ""}
                    disabled={shaField === ""}
                    onClick={() => doImport(fileImport.current, shaField)}
                >
                    {t("electionEventScreen.import.import")}
                </ImportStyles.ImportButton>
            </Box>
        </Box>
    )
}
