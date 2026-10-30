// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What makes a receipt file the user's own: the ballot ID the voting portal
// showed, with the election and the event (Vitrine's specs/receipt.md,
// section 1.1). The event and the elections come from the pinned policy when
// it names them; the ballot ID always comes from the user, never from the
// receipt.

import React, {useState} from "react"
import Alert from "@mui/material/Alert"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import TextField from "@mui/material/TextField"
import {useTranslation} from "react-i18next"
import type {Expect, Policy} from "@vitrine/verify"
import {expectFromForm, IBallotForm} from "../../services/vitrine/check"
import {policyElections} from "../../services/vitrine/policies"
import type {IVitrineSdk} from "../../services/vitrine/sdk"
import {BallotIdField} from "./BallotIdField"

export interface IBallotFormProps {
    sdk: IVitrineSdk
    policy: Policy
    onExpect: (expect: Expect) => void
}

export const BallotForm: React.FC<IBallotFormProps> = ({sdk, policy, onExpect}) => {
    const {t} = useTranslation()
    const {eventId, electionIds} = policyElections(policy)
    const [form, setForm] = useState<IBallotForm>({
        eventId: eventId ?? "",
        electionId: electionIds.length === 1 ? electionIds[0] : "",
        ballotId: "",
    })
    const [invalid, setInvalid] = useState(false)

    const set =
        (field: keyof IBallotForm) =>
        (event: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) =>
            setForm((f) => ({...f, [field]: event.target.value}))

    const submit = (event: React.FormEvent) => {
        event.preventDefault()
        const expect = expectFromForm(sdk, form)
        setInvalid(!expect)
        if (expect) onExpect(expect)
    }

    return (
        <Box
            component="form"
            onSubmit={submit}
            display="flex"
            flexDirection="column"
            gap="12px"
            maxWidth="630px"
            data-testid="ballot-form"
        >
            <BallotIdField
                value={form.ballotId}
                onChange={(ballotId) => setForm((f) => ({...f, ballotId}))}
            />
            {electionIds.length > 1 ? (
                <TextField
                    select
                    required
                    label={t("receiptScreen.ballot.election")}
                    InputLabelProps={{shrink: true}}
                    SelectProps={{native: true}}
                    value={form.electionId}
                    onChange={set("electionId")}
                    inputProps={{"data-testid": "ballot-election"}}
                >
                    <option value="" disabled>
                        {t("receiptScreen.ballot.electionPlaceholder")}
                    </option>
                    {electionIds.map((id) => (
                        <option key={id} value={id}>
                            {id}
                        </option>
                    ))}
                </TextField>
            ) : (
                <TextField
                    required
                    label={t("receiptScreen.ballot.election")}
                    InputLabelProps={{shrink: true}}
                    InputProps={{readOnly: electionIds.length === 1}}
                    value={form.electionId}
                    onChange={set("electionId")}
                    inputProps={{"data-testid": "ballot-election", "spellCheck": false}}
                />
            )}
            <TextField
                required
                label={t("receiptScreen.ballot.event")}
                InputLabelProps={{shrink: true}}
                InputProps={{readOnly: eventId !== undefined}}
                value={form.eventId}
                onChange={set("eventId")}
                inputProps={{"data-testid": "ballot-event", "spellCheck": false}}
            />
            {invalid ? (
                <Alert severity="warning" data-testid="ballot-invalid">
                    {t("ballotIdField.formatError")}
                </Alert>
            ) : null}
            <Box>
                <Button type="submit" data-testid="ballot-check">
                    {t("receiptScreen.ballot.check")}
                </Button>
            </Box>
        </Box>
    )
}
