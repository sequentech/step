// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Where the user gives the receipt screen a trust policy: its text, pasted or
// from a file, or its https address, as the official election notice gives
// it.

import React, {useState} from "react"
import Alert from "@mui/material/Alert"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import TextField from "@mui/material/TextField"
import {useTranslation} from "react-i18next"
import {MAX_POLICY_BYTES} from "../../services/vitrine/http"
import {EInputProblem, parsePolicyInput, TPolicySource} from "../../services/vitrine/input"

export interface IPolicyInputProps {
    onSource: (source: TPolicySource) => void
}

export const PolicyInput: React.FC<IPolicyInputProps> = ({onSource}) => {
    const {t} = useTranslation()
    const [value, setValue] = useState("")
    const [problem, setProblem] = useState<EInputProblem | undefined>(undefined)

    const use = (raw: string) => {
        const parsed = parsePolicyInput(raw)
        if (!parsed.ok) {
            setProblem(parsed.problem)
            return
        }
        setProblem(undefined)
        onSource(parsed.value)
    }

    const onFile = async (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0]
        event.target.value = ""
        if (!file) return
        if (file.size > MAX_POLICY_BYTES) {
            setProblem(EInputProblem.TOO_LARGE)
            return
        }
        try {
            use(await file.text())
        } catch {
            setProblem(EInputProblem.UNREADABLE)
        }
    }

    return (
        <Box display="flex" flexDirection="column" gap="12px">
            <TextField
                multiline
                minRows={2}
                maxRows={8}
                fullWidth
                label={t("receiptScreen.policy.inputLabel")}
                placeholder={t("receiptScreen.policy.inputPlaceholder")}
                InputLabelProps={{shrink: true}}
                value={value}
                onChange={(e) => setValue(e.target.value)}
                inputProps={{"data-testid": "policy-input", "spellCheck": false}}
            />
            {problem ? (
                <Alert severity="warning" data-testid="policy-input-problem">
                    {t(`receiptScreen.policy.problems.${problem}`)}
                </Alert>
            ) : null}
            <Box display="flex" gap="8px" flexWrap="wrap">
                <Button onClick={() => use(value)} data-testid="policy-load">
                    {t("receiptScreen.policy.load")}
                </Button>
                <Button component="label" variant="secondary">
                    {t("receiptScreen.policy.openFile")}
                    <input
                        type="file"
                        hidden
                        accept="text/plain,.txt,.policy"
                        onChange={onFile}
                        data-testid="policy-file"
                    />
                </Button>
            </Box>
        </Box>
    )
}
