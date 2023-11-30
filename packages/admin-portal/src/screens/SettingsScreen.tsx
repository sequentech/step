import React from "react"
import {Box} from "@mui/system"
import {Resource} from "react-admin"

import {Tabs} from "../components/Tabs"
import {HeaderTitle} from "../components/HeaderTitle"
import {SettingsElectionsTypes} from "../resources/Settings/SettingsElectionsTypes"
import { SettingsElectionsTypesCreate } from '@/resources/Settings/SettingsElectionsTypesCreate'

import {SettingsVotingChannels} from "../resources/Settings/SettingsVotingChannel"

export const SettingsScreen: React.FC = () => {
    return (
        <Box>
            <HeaderTitle title="Settings" subtitle="General Configuration" />

            <Tabs
                elements={[
                    {
                        label: "Election types",
                        component: () => (
                            <Resource
                                name="sequent_backend_election_type"
                                list={SettingsElectionsTypes}
                                create={SettingsElectionsTypesCreate}
                                edit={SettingsElectionsTypesCreate}
                                show={SettingsElectionsTypesCreate}
                            />
                        ),
                    },
                    {
                        label: "VOTING CHANNELS",
                        component: () => (
                            <Resource
                                name="sequent_backend_tenant"
                                list={SettingsVotingChannels}
                            />
                        ),
                    },
                    {label: "COMMUNICATION", component: () => <></>},
                    {label: "LANGUAGES", component: () => <></>},
                ]}
            />
        </Box>
    )
}
