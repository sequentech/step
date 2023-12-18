import {Box, styled, Button, TextField, CircularProgress} from "@mui/material"
import {DropFile} from "@sequentech/ui-essentials"
import React, {useEffect, memo, useRef} from "react"
import {useTranslation} from "react-i18next"

interface ImportScreenProps {
    doImport: (file: FileList | null, sha: string) => void
    doCancel: () => void
    isLoading: boolean
    refresh?: string
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

export const ImportScreenMemo: React.MemoExoticComponent<React.FC<ImportScreenProps>> = memo(
    (props: ImportScreenProps): React.JSX.Element => {
        const {doCancel, doImport, isLoading, refresh} = props

        const {t} = useTranslation()

        const [shaField, setShaField] = React.useState<string>("")
        const [fileImport, setFileImport] = React.useState<FileList | null>(null)

        const handleFiles = (files: FileList | null) => {
            setFileImport(files)
        }

        useEffect(() => {
            setShaField("")
            setFileImport(null)
        }, [refresh])

        return (
            <Box sx={{padding: "16px"}}>
                <TextField
                    label={t("electionEventScreen.import.sha")}
                    size="small"
                    value={shaField}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                        setShaField(e.target.value)
                    }
                />

                <DropFile handleFiles={handleFiles} />

                <Box
                    sx={{
                        display: "flex",
                        flexDirection: "row",
                        justifyContent: "center",
                        alignItems: "center",
                        marginTop: "32px",
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
                        disabled={!fileImport || shaField === ""}
                        onClick={() => doImport(fileImport, shaField)}
                    >
                        {t("electionEventScreen.import.import")}
                    </ImportStyles.ImportButton>
                </Box>
            </Box>
        )
    }
)

ImportScreenMemo.displayName = "ImportScreen"

export const ImportScreen = ImportScreenMemo
