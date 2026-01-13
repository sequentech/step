// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Sequent_Backend_Election} from "@/gql/graphql"
import {Identifier} from "react-admin"
import {useTranslation} from "react-i18next"
import ElectionHeader from "@/components/ElectionHeader"
import {ListTallySheet} from "./ListTallySheet"
import {WizardSteps} from "./TallySheetWizardCopy"

interface ITallySheetProps {
    election: Sequent_Backend_Election
}

export const TallySheets: React.FC<ITallySheetProps> = (props) => {
    const {election} = props
    const [action, setAction] = useState<number>(WizardSteps.List)
    const [refresh, setRefresh] = useState<string | null>(null)
    const [tallySheetId, setTallySheetId] = useState<Identifier | undefined>()
    const {t} = useTranslation()

    const handleAction = (action: number, id?: Identifier) => {
        setAction(action)
        setRefresh(new Date().getTime().toString())
        if (id) {
            setTallySheetId(id)
        }
    }

    return (
        <>
            <ElectionHeader title={t("tallysheet.title")} subtitle="tallysheet.subtitle" />
            {action === WizardSteps.List ? (
                <ListTallySheet election={election} doAction={handleAction} reload={refresh} />
            ) : (
                <>TODO</>
            )}
        </>
    )
}
