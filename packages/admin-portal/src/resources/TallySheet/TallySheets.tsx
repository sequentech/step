// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Sequent_Backend_Election, Sequent_Backend_Tally_Sheet} from "@/gql/graphql"
import {Identifier, RaRecord} from "react-admin"
import {useTranslation} from "react-i18next"
import ElectionHeader from "@/components/ElectionHeader"
import {ListTallySheet} from "./ListTallySheet"
import {TallySheetWizard, WizardSteps} from "./TallySheetWizardCopy"
import {ListTallySheetVersions} from "./ListTallySheetVersions"

interface ITallySheetProps {
    election: Sequent_Backend_Election
}

export const TallySheets: React.FC<ITallySheetProps> = (props) => {
    const {election} = props
    const [action, setAction] = useState<number>(WizardSteps.List)
    const [refresh, setRefresh] = useState<string | null>(null)
    const [tallySheetRecord, setTallySheetRecord] = useState<RaRecord<Identifier> | undefined>()
    const [showVersionsTable, setShowVersionsTable] = useState(false)
    const {t} = useTranslation()

    const handleAction = (action: number) => {
        setAction(action)
        setRefresh(new Date().getTime().toString())
        if (action === WizardSteps.List) {
            setTallySheetRecord(undefined)
        }
    }

    return (
        <>
            <ElectionHeader title={t("tallysheet.title")} subtitle="tallysheet.subtitle" />
            {action === WizardSteps.List ? (
                showVersionsTable && tallySheetRecord ? (
                    <ListTallySheetVersions
                        election={election}
                        tallySheet={tallySheetRecord as Sequent_Backend_Tally_Sheet}
                        doAction={handleAction}
                        setShowVersionsTable={setShowVersionsTable}
                        setTallySheetRecord={setTallySheetRecord}
                    />
                ) : (
                    <ListTallySheet
                        election={election}
                        doAction={handleAction}
                        reload={refresh}
                        tallySheetRecord={tallySheetRecord}
                        setTallySheetRecord={setTallySheetRecord}
                        setShowVersionsTable={setShowVersionsTable}
                    />
                )
            ) : (
                <TallySheetWizard
                    election={election}
                    tallySheet={tallySheetRecord as Sequent_Backend_Tally_Sheet}
                    doAction={handleAction}
                    action={action}
                    isShowTallySheet={action === WizardSteps.Review && showVersionsTable}
                />
            )}
        </>
    )
}
