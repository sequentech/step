import React from "react"
import {Box} from "@mui/system"
import {Resource} from "react-admin"

import {Tabs} from "../components/Tabs"
import {HeaderTitle} from "../components/HeaderTitle"
import {SettingsElectionsTypes} from "../resources/Settings/SettingsElectionsTypes"
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
                                name="sequent_backend_election_event"
                                list={SettingsElectionsTypes}
                            />
                        ),
                    },
                    {
                        label: "VOTING CHANNELS",
                        component: () => (
                            <Resource
                                name="sequent_backend_election_event"
                                list={SettingsVotingChannels}
                            />
                        ),
                    },
                    {label: "COMMUNICATION", component: () => <>C</>},
                    {label: "LANGUAGES", component: () => <>D</>},
                ]}
            />
        </Box>
    )
}
