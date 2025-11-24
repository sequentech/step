// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {Identifier, TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Contest} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditContestData} from "./EditContestData"
import {ListTallySheet} from "../TallySheet/ListTallySheet"
import {TallySheetWizard, WizardSteps} from "../TallySheet/TallySheetWizard"
import {CircularProgress} from "@mui/material"

export const ContestTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Contest>()

    const [action, setAction] = useState<number>(WizardSteps.List)
    const [refresh, setRefresh] = useState<string | null>(null)
    const [tallySheetId, setTallySheetId] = useState<Identifier | undefined>()

    const handleAction = (action: number, id?: Identifier) => {
        setAction(action)
        setRefresh(new Date().getTime().toString())
        if (id) {
            setTallySheetId(id)
        }
    }
    if (!record) {
        return <CircularProgress />
    }

    return (
        <>
            <ElectionHeader title={record?.name || ""} subtitle="contestScreen.common.subtitle" />
            <TabbedShowLayout>
                <TabbedShowLayout.Tab label="Data">
                    <EditContestData />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab
                    label="Tally Sheets"
                    onClick={() => {
                        setAction(WizardSteps.List)
                    }}
                >
                    {action === WizardSteps.List ? (
                        <ListTallySheet contest={record} doAction={handleAction} reload={refresh} />
                    ) : action === WizardSteps.Start ? (
                        <TallySheetWizard
                            contest={record}
                            action={action}
                            doAction={handleAction}
                        />
                    ) : action === WizardSteps.Edit ? (
                        <TallySheetWizard
                            tallySheetId={tallySheetId}
                            contest={record}
                            action={action}
                            doAction={handleAction}
                        />
                    ) : action === WizardSteps.Confirm ? (
                        <TallySheetWizard
                            tallySheetId={tallySheetId}
                            contest={record}
                            action={action}
                            doAction={handleAction}
                        />
                    ) : action === WizardSteps.View ? (
                        <TallySheetWizard
                            tallySheetId={tallySheetId}
                            contest={record}
                            action={action}
                            doAction={handleAction}
                        />
                    ) : null}
                </TabbedShowLayout.Tab>
            </TabbedShowLayout>
        </>
    )
}
