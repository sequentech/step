// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import {useTranslation} from "react-i18next"
import {TallyStyles} from "@/components/styles/TallyStyles"
import {Identifier, Notification, useGetOne, useNotify} from "react-admin"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {
    Sequent_Backend_Contest,
    Sequent_Backend_Tally_Sheet,
    Sequent_Backend_Tally_Sheet_Insert_Input,
} from "@/gql/graphql"
import {CancelButton, NextButton} from "./styles"
import {EditTallySheet} from "./EditTallySheet"
import {ShowTallySheet} from "./ShowTallySheet"

export const WizardSteps = {
    List: -1,
    Start: 1,
    Edit: 2,
    Confirm: 3,
    View: 4,
}

interface TallySheetWizardProps {
    tallySheetId?: Identifier | undefined
    contest: Sequent_Backend_Contest
    action: number
    doAction: (action: number, id?: Identifier) => void
}

export const TallySheetWizard: React.FC<TallySheetWizardProps> = (props) => {
    const {action, contest, tallySheetId, doAction} = props

    const submitRef = React.useRef<HTMLButtonElement>(null)
    const notify = useNotify()

    const {t} = useTranslation()
    const [page, setPage] = useState<number>(WizardSteps.Edit)
    const [areaId, setAreaId] = useState<Identifier | undefined>()
    const [createdTallySheet, setCreatedTallySheet] = useState<
        Sequent_Backend_Tally_Sheet_Insert_Input | undefined
    >()
    const [editedTallySheet, setEditedTallySheet] = useState<
        Sequent_Backend_Tally_Sheet | undefined
    >()
    const [isButtonDisabled, setIsButtonDisabled] = useState<boolean>(false)

    const {data: tallySheet} = useGetOne("sequent_backend_tally_sheet", {id: tallySheetId})

    useEffect(() => {
        if (action) {
            setPage(action)
        }
    }, [action])

    const handleNext = () => {
        if (page === WizardSteps.Start || page === WizardSteps.Edit) {
            submitRef.current?.click()
            // needs to wait for the click handler to submit the data
            setTimeout(() => {
                const tallySheet = localStorage.getItem("tallySheetData")
                if (tallySheet) {
                    doAction(WizardSteps.Confirm)
                } else {
                    notify(t("tallysheet.allFieldsRequired"), {type: "error"})
                }
            }, 400)
        } else if (page === WizardSteps.Confirm) {
            submitRef.current?.click()
            doAction(WizardSteps.List)
        }
    }

    const handleBack = () => {
        const tallySheet = localStorage.getItem("tallySheetData")
        const tallySheetTemp = JSON.parse(tallySheet || "{}")
        if (page === WizardSteps.Start) {
            doAction(WizardSteps.List)
        } else if (page === WizardSteps.Edit) {
            doAction(WizardSteps.List)
        } else if (page === WizardSteps.Confirm) {
            if (tallySheetId && tallySheetTemp && tallySheetTemp.id) {
                doAction(WizardSteps.List)
            } else {
                doAction(WizardSteps.Edit)
            }
        } else if (page === WizardSteps.View) {
            doAction(WizardSteps.List)
        }
    }

    return (
        <>
            <WizardStyles.WizardWrapper>
                <TallyStyles.StyledHeader>
                    <BreadCrumbSteps
                        labels={[
                            "tallysheet.breadcrumbSteps.edit",
                            page === WizardSteps.View
                                ? "tallysheet.breadcrumbSteps.view"
                                : "tallysheet.breadcrumbSteps.confirm",
                        ]}
                        selected={page}
                        variant={BreadCrumbStepsVariant.Circle}
                        colorPreviousSteps={true}
                    />
                </TallyStyles.StyledHeader>

                {page === WizardSteps.Start && (
                    <>
                        <EditTallySheet
                            contest={contest}
                            doSelectArea={(id: Identifier) => setAreaId(id)}
                            doCreatedTalySheet={(
                                tallySheet: Sequent_Backend_Tally_Sheet_Insert_Input
                            ) => {
                                setCreatedTallySheet(tallySheet)
                            }}
                            submitRef={submitRef}
                        />
                    </>
                )}

                {page === WizardSteps.Edit && (
                    <>
                        <EditTallySheet
                            tallySheet={tallySheet}
                            contest={contest}
                            doCreatedTalySheet={(
                                tallySheet: Sequent_Backend_Tally_Sheet_Insert_Input
                            ) => {
                                setCreatedTallySheet(tallySheet)
                            }}
                            submitRef={submitRef}
                        />
                    </>
                )}

                {page === WizardSteps.Confirm && (
                    <>
                        <ShowTallySheet
                            tallySheet={createdTallySheet || tallySheet}
                            contest={contest}
                            doEditedTalySheet={(tallySheet: Sequent_Backend_Tally_Sheet) =>
                                setEditedTallySheet(tallySheet)
                            }
                            submitRef={submitRef}
                        />
                    </>
                )}

                {page === WizardSteps.View && (
                    <>
                        <ShowTallySheet
                            tallySheet={tallySheet}
                            contest={contest}
                            doEditedTalySheet={(tallySheet: Sequent_Backend_Tally_Sheet) =>
                                setEditedTallySheet(tallySheet)
                            }
                            submitRef={submitRef}
                        />
                    </>
                )}

                <TallyStyles.StyledFooter>
                    <CancelButton className="list-actions" onClick={handleBack}>
                        {t("tallysheet.common.cancel")}
                    </CancelButton>
                    {page !== WizardSteps.View && (
                        <NextButton
                            color="primary"
                            onClick={handleNext}
                            disabled={isButtonDisabled}
                        >
                            <>
                                {page === WizardSteps.Edit
                                    ? t("tallysheet.common.confirm")
                                    : page === WizardSteps.Confirm
                                      ? t("tallysheet.common.save")
                                      : t("tallysheet.common.next")}
                                <ChevronRightIcon />
                            </>
                        </NextButton>
                    )}
                </TallyStyles.StyledFooter>
            </WizardStyles.WizardWrapper>
        </>
    )
}
