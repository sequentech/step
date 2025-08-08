// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import {useTranslation} from "react-i18next"
import {TallyStyles} from "@/components/styles/TallyStyles"
import {Identifier, Notification, useGetList, useGetOne, useNotify} from "react-admin"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Sheet,
    Sequent_Backend_Tally_Sheet_Insert_Input,
    CreateNewTallySheetMutation,
    ReviewTallySheetMutation,
    AddTallySheetVersionMutation,
} from "@/gql/graphql"
import {CancelButton, NextButton} from "./styles"
import {EditTallySheet} from "./EditTallySheet"
import {ShowTallySheet} from "./ShowTallySheet"
import {useMutation} from "@apollo/client"
import {CREATE_NEW_TALLY_SHEET} from "@/queries/createNewTallySheet"
import {ADD_TALLY_SHEET_VERSION} from "@/queries/addTallySheetVersion"
import {REVIEW_TALLY_SHEET} from "@/queries/reviewTallySheet"
import {IPermissions} from "@/types/keycloak"

export const WizardSteps = {
    List: -1,
    Start: 1,
    Edit: 2,
    Confirm: 3,
    View: 4,
}

interface TallySheetWizardProps {
    tallySheetId?: Identifier | undefined
    election: Sequent_Backend_Election
    action: number
    doAction: (action: number, id?: Identifier) => void
}

export const TallySheetWizard: React.FC<TallySheetWizardProps> = (props) => {
    const {action, election: election, tallySheetId, doAction} = props

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
    const [choosenContest, setChoosenContest] = useState<Sequent_Backend_Contest | undefined>()

    const {data: tallySheet} = useGetOne<Sequent_Backend_Tally_Sheet>(
        "sequent_backend_tally_sheet",
        {id: tallySheetId},
        {enabled: !!tallySheetId}
    )

    const {data: listTallySheets} = useGetList<Sequent_Backend_Tally_Sheet>(
        "sequent_backend_tally_sheet",
        {filter: {contest_id: election.id}},
        {enabled: !!election.id}
    )

    const [CreateNewTallySheet] = useMutation<CreateNewTallySheetMutation>(CREATE_NEW_TALLY_SHEET, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.TALLY_SHEET_CREATE,
            },
        },
    })

    useEffect(() => {
        if (action) {
            setPage(action)
        }
    }, [action])

    const insertTallySheetAction = async () => {
        try {
            const tallySheetString = localStorage.getItem("tallySheetData")
            if (!tallySheetString) {
                return
            }
            const tallySheetData: Sequent_Backend_Tally_Sheet_Insert_Input =
                JSON.parse(tallySheetString)
            let {errors} = await CreateNewTallySheet({
                variables: {
                    electionEventId: tallySheetData.election_event_id,
                    channel: tallySheetData.channel,
                    content: tallySheetData.content,
                    contestId: tallySheetData.contest_id,
                    areaId: tallySheetData.area_id,
                },
            })
            if (errors) {
                notify(t("tallysheet.createTallyError"), {type: "error"})
                console.log(`Error creating tally sheet: ${errors}`)
            } else {
                notify(t("tallysheet.createTallySuccess"), {type: "success"})
            }
        } catch (error) {
            notify(t("tallysheet.createTallyError"), {type: "error"})
            console.log(`Error creating tally sheet: ${error}`)
        }
    }

    const sameKindOfTallySheetExists = (tallySheet: string) => {
        const tallySheetData: Sequent_Backend_Tally_Sheet_Insert_Input = JSON.parse(tallySheet)
        return listTallySheets?.find(
            (tallySheet) =>
                tallySheet.area_id === tallySheetData.area_id &&
                tallySheet.contest_id === tallySheetData.contest_id &&
                tallySheet.channel === tallySheetData.channel
        )
    }

    const handleNext = () => {
        if (page === WizardSteps.Start || page === WizardSteps.Edit) {
            submitRef.current?.click()
            // needs to wait for the click handler to submit the data
            setTimeout(() => {
                const tallySheet = localStorage.getItem("tallySheetData")
                if (tallySheet) {
                    // Do not allow creating a new tally sheet if one with the same channel and area already exists
                    if (page === WizardSteps.Start && sameKindOfTallySheetExists(tallySheet)) {
                        notify(t("tallysheet.createTallyErrorSameKindExists"), {type: "error"})
                        return
                    }
                    doAction(WizardSteps.Confirm)
                } else {
                    notify(t("tallysheet.allFieldsRequired"), {type: "error"})
                }
            }, 400)
        } else if (page === WizardSteps.Confirm) {
            insertTallySheetAction()
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

    console.log("page: ", page)
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
                                election={election}
                                choosenContest={choosenContest}
                                setChoosenContest={setChoosenContest}
                                doSelectArea={(id: Identifier) => setAreaId(id)}
                                doCreatedTalySheet={(
                                    tallySheet: Sequent_Backend_Tally_Sheet_Insert_Input
                                ) => {
                                    setCreatedTallySheet(tallySheet)
                                }}
                                submitRef={submitRef}
                                setIsButtonDisabled={setIsButtonDisabled}
                            />
                        </>
                    )}
                {page === WizardSteps.Edit &&
                    choosenContest /* TODO: check if this is necessary*/ && (
                        <>
                            <EditTallySheet // TODO: EditTallySheet will keep the business logic for entering the results and calculations. But the area/contest/channel selection should be done on a separate component
                                tallySheet={tallySheet}
                                election={election}
                                choosenContest={choosenContest}
                                setChoosenContest={setChoosenContest}
                                doCreatedTalySheet={(
                                    tallySheet: Sequent_Backend_Tally_Sheet_Insert_Input
                                ) => {
                                    setCreatedTallySheet(tallySheet)
                                }}
                                submitRef={submitRef}
                                setIsButtonDisabled={setIsButtonDisabled}
                            />
                        </>
                    )}

                {page === WizardSteps.Confirm &&
                    choosenContest /* TODO: check if this is necessary*/ && (
                        <>
                            <ShowTallySheet
                                tallySheet={createdTallySheet || tallySheet}
                                contest={choosenContest}
                                doEditedTalySheet={(tallySheet: Sequent_Backend_Tally_Sheet) =>
                                    setEditedTallySheet(tallySheet)
                                }
                                submitRef={submitRef}
                            />
                        </>
                    )}

                {page === WizardSteps.View &&
                    choosenContest /* TODO: check if this is necessary*/ && (
                        <>
                            <ShowTallySheet
                                tallySheet={tallySheet}
                                contest={choosenContest}
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
