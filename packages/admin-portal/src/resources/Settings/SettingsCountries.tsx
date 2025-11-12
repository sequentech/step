// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {LimitAccessByCountriesMutation} from "@/gql/graphql"
import {COUNTRIES} from "@/lib/countries"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {LIMIT_ACCESS_BY_COUNTRIES} from "@/queries/limitAccessByCountries"
import {useMutation} from "@apollo/client"
import {Typography} from "@mui/material"
import React, {useEffect, useState} from "react"
import {
    AutocompleteArrayInput,
    SaveButton,
    SimpleForm,
    useEditController,
    useNotify,
} from "react-admin"
import {useTranslation} from "react-i18next"

export const SettingsCountries: React.FC<void> = () => {
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const notify = useNotify()

    const [limitAccessByCountries] =
        useMutation<LimitAccessByCountriesMutation>(LIMIT_ACCESS_BY_COUNTRIES)
    const {record, save} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })
    const [selectedVotingCountries, setSelectedVotingCountries] = useState<Array<string>>([])
    const [selectedEnrollmentCountries, setSelectedEnrollmentCountries] = useState<Array<string>>(
        []
    )

    useEffect(() => {
        if (record && record?.settings) {
            setSelectedVotingCountries(record.settings.voting_countries ?? [])
            setSelectedEnrollmentCountries(record.settings.enroll_countries ?? [])
        }
    }, [record])

    const handleSumbit = async () => {
        if (save) {
            const {data, errors} = await limitAccessByCountries({
                variables: {
                    votingCountries: selectedVotingCountries,
                    enrollCountries: selectedEnrollmentCountries,
                },
            })
            if (!errors) {
                save({
                    settings: {
                        ...(record?.settings ? record.settings : {}),
                        voting_countries: selectedVotingCountries,
                        enroll_countries: selectedEnrollmentCountries,
                    },
                })
            } else {
                notify(t(`settings.countries.error.errorSaving`), {type: "error"})
                console.log(`Error saving country list: ${errors}`)
            }
        }
    }

    return (
        <SimpleForm
            className="countries-form"
            toolbar={<SaveButton className="save" />}
            resource="sequent_backend_tenant"
            onSubmit={handleSumbit}
            record={record}
        >
            <Typography className="title" variant="h4">
                {t("settings.countries.title")}
            </Typography>
            <Typography className="description" variant="body2">
                {t("settings.countries.votingDescription")}
            </Typography>

            <AutocompleteArrayInput
                fullWidth
                className="country-list-input"
                source="settings.voting_countries"
                label={"Countries"}
                choices={COUNTRIES}
                onChange={setSelectedVotingCountries}
                optionValue="code"
            />

            <Typography className="description" variant="body2">
                {t("settings.countries.enrollmentDescription")}
            </Typography>

            <AutocompleteArrayInput
                fullWidth
                className="country-list-input"
                source="settings.enroll_countries"
                label={"Countries"}
                choices={COUNTRIES}
                onChange={setSelectedEnrollmentCountries}
                optionValue="code"
            />
        </SimpleForm>
    )
}
