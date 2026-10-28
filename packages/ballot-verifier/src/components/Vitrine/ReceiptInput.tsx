// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Where the user gives the receipt screen a receipt: its file, or its text,
// its link or its reference pasted. The text is passed on as it is; the
// screen reads it as data.

import React, {useState} from "react"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import TextField from "@mui/material/TextField"
import {useTranslation} from "react-i18next"
import {DropFile} from "@sequentech/ui-essentials"
import {CHAIN_FILE_EXTENSION, EInputProblem, MAX_RECEIPT_TEXT} from "../../services/vitrine/input"

export interface IReceiptInputProps {
    onText: (text: string) => void
    onProblem: (problem: EInputProblem) => void
}

export const ReceiptInput: React.FC<IReceiptInputProps> = ({onText, onProblem}) => {
    const {t} = useTranslation()
    const [pasted, setPasted] = useState("")

    const handleFiles = async (files: FileList) => {
        const file = files[0]
        if (!file) return
        if (file.size > MAX_RECEIPT_TEXT) {
            onProblem(EInputProblem.TOO_LARGE)
            return
        }
        try {
            onText(await file.text())
        } catch {
            onProblem(EInputProblem.UNREADABLE)
        }
    }

    return (
        <Box display="flex" flexDirection="column" gap="12px">
            <DropFile
                handleFiles={handleFiles}
                accept={`${CHAIN_FILE_EXTENSION},text/plain`}
                formatLabel={t("receiptScreen.receipt.fileFormat")}
            />
            <TextField
                multiline
                minRows={2}
                maxRows={8}
                fullWidth
                label={t("receiptScreen.receipt.pasteLabel")}
                placeholder={t("receiptScreen.receipt.pastePlaceholder")}
                InputLabelProps={{shrink: true}}
                value={pasted}
                onChange={(e) => setPasted(e.target.value)}
                inputProps={{"data-testid": "receipt-paste", "spellCheck": false}}
            />
            <Box>
                <Button onClick={() => onText(pasted)} data-testid="receipt-open">
                    {t("receiptScreen.receipt.open")}
                </Button>
            </Box>
        </Box>
    )
}
