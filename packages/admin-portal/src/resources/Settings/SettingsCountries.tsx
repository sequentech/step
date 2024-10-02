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
import {AutocompleteArrayInput, SaveButton, SimpleForm, useEditController} from "react-admin"
import {useTranslation} from "react-i18next"

export const SettingsCountries: React.FC<void> = () => {
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()

    const [limitAccessByCountries] =
        useMutation<LimitAccessByCountriesMutation>(LIMIT_ACCESS_BY_COUNTRIES)
    const {record, save} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })
    const [selectedCountries, setSelectedCountries] = useState([])

    useEffect(() => {
        if (record && record?.settings) {
            setSelectedCountries(record.settings.countries ?? [])
        }
    }, [record])

    const handleSumbit = async () => {
        if (save) {
            const {data, errors} = await limitAccessByCountries({
                variables: {
                    countries: selectedCountries,
                },
            })
            if (!errors) {
                save({
                    settings: {
                        ...(record?.settings ? record.settings : {}),
                        countries: selectedCountries,
                    },
                })
            }
        }
    }

    return (
        <SimpleForm
            toolbar={<SaveButton />}
            resource="sequent_backend_tenant"
            onSubmit={handleSumbit}
            record={record}
        >
            <Typography variant="h4">{t("settings.countries.title")}</Typography>
            <Typography variant="body2">{t("settings.countries.description")}</Typography>

            <AutocompleteArrayInput
                fullWidth
                source="settings.countries"
                label={"Countries"}
                choices={COUNTRIES}
                onChange={setSelectedCountries}
                optionValue="code"
            />
        </SimpleForm>
    )
}
