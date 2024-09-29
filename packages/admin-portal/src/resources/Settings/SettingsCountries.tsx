import {LimitAccessByCountriesMutation} from "@/gql/graphql"
import {COUNTRIES} from "@/lib/countries"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {LIMIT_ACCESS_BY_COUNTRIES} from "@/queries/limitAccessByCountries"
import {IPermissions} from "@/types/keycloak"
import {useMutation} from "@apollo/client"
import React, {useEffect, useState} from "react"
import {AutocompleteArrayInput, SaveButton, SimpleForm, useEditController} from "react-admin"

export const SettingsCountries: React.FC<void> = () => {
    const [tenantId] = useTenantStore()
    const [limitAccessByCountries] = useMutation<LimitAccessByCountriesMutation>(
        LIMIT_ACCESS_BY_COUNTRIES,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.ADMIN_USER,
                },
            },
        }
    )
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
