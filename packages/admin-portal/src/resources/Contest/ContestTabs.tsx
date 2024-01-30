import React, {useState} from "react"
import {Identifier, TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Contest} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditContestData} from "./EditContestData"
import {ListTallySheet} from "../TallySheet/ListTallySheet"
import {TallySheetWizard, WizardSteps} from "../TallySheet/TallySheetWizard"
import { useTranslation } from 'react-i18next'
import { translateElection } from '@sequentech/ui-essentials'

export const ContestTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Contest>()
    const {t, i18n} = useTranslation()
    
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

    return (
        <>
            <ElectionHeader
                title={translateElection(record, "name", i18n.language) ?? ""}
                subtitle="electionEventScreen.common.subtitle"
            />
            <TabbedShowLayout>
                <TabbedShowLayout.Tab label={t("contestScreen.tab.data")}>
                    <EditContestData />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab
                    label={t("contestScreen.tab.tallySheet")}
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
