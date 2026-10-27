// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Alert, Stack, Typography} from "@mui/material"
import {EditBase, SimpleForm, SelectInput, Toolbar, SaveButton} from "react-admin"
import {EventTimeZoneSettings} from "@/components/timezones/EventTimeZoneSettings"

const choices = (values: Array<[string, string]>) => values.map(([id, name]) => ({id, name}))

/** Only fields present in the database's administrator projection are edited.
 * Certificate issuers and quorums have their dedicated Signatures interfaces. */
export function AdministratorSecuritySettings() {
    return (
        <EditBase
            mutationMode="pessimistic"
            redirect={false}
            transform={(data) => ({id: data.id, presentation: data.presentation})}
        >
            <SimpleForm
                toolbar={
                    <Toolbar>
                        <SaveButton label="Save security settings" />
                    </Toolbar>
                }
            >
                <Stack spacing={2} sx={{width: "100%"}}>
                    <Typography variant="h5">Security settings</Typography>
                    <Alert severity="info">
                        Change these settings during Preparation. Configure certificates and
                        required board approvals in Signatures, and background checks in Security.
                    </Alert>
                    <SelectInput
                        source="presentation.keys_ceremony_policy"
                        label="Key ceremony scope"
                        choices={choices([
                            ["ELECTION_EVENT", "One ceremony for the event"],
                            ["ELECTION", "One ceremony for each election"],
                        ])}
                    />
                    <SelectInput
                        source="presentation.ceremonies_policy"
                        label="Trustee participation"
                        choices={choices([
                            ["manual-ceremonies", "Trustees participate"],
                            ["automated-ceremonies", "Automated ceremonies"],
                        ])}
                    />
                    <SelectInput
                        source="presentation.contest_encryption_policy"
                        label="Ballot encryption"
                        choices={choices([
                            ["single-contest", "Separate encryption for each contest"],
                            ["multiple-contests", "Encrypt contests together"],
                        ])}
                    />
                    <SelectInput
                        source="presentation.decoded_ballot_inclusion_policy"
                        label="Decoded ballots in results"
                        choices={choices([
                            ["not-included", "Do not include"],
                            ["included", "Include"],
                        ])}
                    />
                    <SelectInput
                        source="presentation.voter_signing_policy"
                        label="Voter ballot signatures"
                        choices={choices([
                            ["with-signature", "Required"],
                            ["no-signature", "Not required"],
                        ])}
                    />
                    <SelectInput
                        source="presentation.ballot_box_seal_policy"
                        label="Ballot box sealing"
                        choices={choices([
                            ["seal-at-close", "Seal when voting closes"],
                            ["do-not-seal", "Do not seal"],
                        ])}
                    />
                    <SelectInput
                        source="presentation.ballot_box_seal_record_policy"
                        label="Seal record access"
                        choices={choices([
                            ["restricted", "Election board only"],
                            ["public", "Public"],
                        ])}
                    />
                    <EventTimeZoneSettings elections={[]} />
                </Stack>
            </SimpleForm>
        </EditBase>
    )
}
