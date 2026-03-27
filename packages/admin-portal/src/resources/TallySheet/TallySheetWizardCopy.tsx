// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import {useTranslation} from "react-i18next"
import {TallyStyles} from "@/components/styles/TallyStyles"
import {useNotify} from "react-admin"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Sheet,
    Sequent_Backend_Tally_Sheet_Insert_Input,
    CreateNewTallySheetMutation,
} from "@/gql/graphql"
import {useMutation} from "@apollo/client"
import {CREATE_NEW_TALLY_SHEET} from "@/queries/createNewTallySheet"
import {IPermissions} from "@/types/keycloak"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {TallySheetConfigStep} from "./TallySheetConfigStep"
import {IAreaContestResults, ITallySheetConfig} from "@/types/TallySheets"
import {TallySheetsDataStep} from "./TallySheetsDataStep"
import {TallySheetReview} from "./TallySheetReview"

export const WizardSteps = {
    List: 0,
    Configuration: 1,
    Data: 2,
    Review: 3,
}

interface TallySheetWizardProps {
    tallySheet?: Sequent_Backend_Tally_Sheet
    election: Sequent_Backend_Election
    action: number
    doAction: (action: number) => void
    isShowTallySheet: boolean
}

export const TallySheetWizard: React.FC<TallySheetWizardProps> = (props) => {
    const {action, election: election, tallySheet, doAction, isShowTallySheet} = props
    const submitRef = React.useRef<HTMLButtonElement>(null)
    const notify = useNotify()

    const {t} = useTranslation()
    const [page, setPage] = useState<number>(action)
    const [config, setConfig] = useState<ITallySheetConfig | undefined>(
        tallySheet
            ? {
                  area_id: tallySheet.area_id,
                  contest_id: tallySheet.contest_id,
                  channel: tallySheet.channel ?? "",
              }
            : undefined
    )

    const [isButtonDisabled, setIsButtonDisabled] = useState<boolean>(true)
    const [choosenContest, setChoosenContest] = useState<Sequent_Backend_Contest | undefined>()
    const [createdTallySheet, setCreatedTallySheet] = useState<
        Sequent_Backend_Tally_Sheet_Insert_Input | undefined
    >()

    const [CreateNewTallySheet] = useMutation<CreateNewTallySheetMutation>(CREATE_NEW_TALLY_SHEET, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.TALLY_SHEET_CREATE,
            },
        },
    })

    const submitDataStep = (results: IAreaContestResults) => {
        if (config) {
            let content: IAreaContestResults = {
                ...results,
                contest_id: config?.contest_id,
                area_id: config?.area_id,
            }

            const tallySheetData:
                | Sequent_Backend_Tally_Sheet
                | Sequent_Backend_Tally_Sheet_Insert_Input = {
                tenant_id: election.tenant_id,
                election_event_id: election.election_event_id,
                election_id: election.id,
                contest_id: config.contest_id,
                area_id: config.area_id,
                channel: config.channel,
                content: content,
            }

            localStorage.setItem("tallySheetData", JSON.stringify(tallySheetData))
            setCreatedTallySheet(tallySheetData)
        } else {
            notify(t("tallysheet.allFieldsRequired"), {type: "error"})
        }
    }

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

    const handleNext = () => {
        if (page === WizardSteps.Configuration || page === WizardSteps.Data) {
            submitRef.current?.click()
            if (page === WizardSteps.Data) {
                // needs to wait for the click handler to submit the data
                setTimeout(() => {
                    const tallySheet = localStorage.getItem("tallySheetData")
                    if (tallySheet) {
                        setPage(WizardSteps.Review)
                    } else {
                        notify(t("tallysheet.allFieldsRequired"), {type: "error"})
                    }
                }, 400)
            } else {
                setPage(WizardSteps.Data)
            }
        } else if (page === WizardSteps.Review) {
            insertTallySheetAction()
            doAction(WizardSteps.List)
        }
    }

    const handleBack = () => {
        if (page === WizardSteps.Configuration) {
            doAction(WizardSteps.List)
        } else if (page === WizardSteps.Data) {
            setPage(WizardSteps.Configuration)
        } else if (page === WizardSteps.Review) {
            if (tallySheet) {
                doAction(WizardSteps.List)
            } else {
                setPage(WizardSteps.Data)
            }
        }
    }

    const reviewTallySheet = tallySheet ?? createdTallySheet

    return (
        <>
            <WizardStyles.WizardWrapper>
                <TallyStyles.StyledHeader>
                    <BreadCrumbSteps
                        labels={[
                            "tallysheet.breadcrumbSteps.edit",
                            page === WizardSteps.Review
                                ? "tallysheet.breadcrumbSteps.view"
                                : "tallysheet.breadcrumbSteps.confirm",
                        ]}
                        selected={page}
                        variant={BreadCrumbStepsVariant.Circle}
                        colorPreviousSteps={true}
                    />
                </TallyStyles.StyledHeader>

                {page === WizardSteps.Configuration && (
                    <TallySheetConfigStep
                        election={election}
                        submitRef={submitRef}
                        setConfig={setConfig}
                        setChoosenContest={setChoosenContest}
                        setIsButtonDisabled={setIsButtonDisabled}
                        currentConfig={config}
                        version={tallySheet?.version ? tallySheet.version + 1 : undefined}
                    />
                )}
                {page === WizardSteps.Data && (
                    <TallySheetsDataStep
                        election={election}
                        submitRef={submitRef}
                        choosenContest={choosenContest}
                        setIsButtonDisabled={setIsButtonDisabled}
                        submitDataStep={submitDataStep}
                        tallySheet={tallySheet}
                    />
                )}
                {page === WizardSteps.Review && reviewTallySheet && (
                    <TallySheetReview tallySheet={reviewTallySheet} election={election} />
                )}
                <WizardStyles.Toolbar>
                    <WizardStyles.BackButton
                        color="info"
                        onClick={handleBack}
                        className="tsw-back-button"
                    >
                        <ArrowBackIosIcon />
                        {t("common.label.back")}
                    </WizardStyles.BackButton>
                    {!isShowTallySheet && (
                        <WizardStyles.NextButton
                            color="primary"
                            disabled={isButtonDisabled}
                            onClick={handleNext}
                            className="tsw-next-button"
                        >
                            {page === WizardSteps.Data
                                ? t("tallysheet.common.confirm")
                                : page === WizardSteps.Review
                                  ? t("tallysheet.common.save")
                                  : t("tallysheet.common.next")}
                            <ChevronRightIcon />
                        </WizardStyles.NextButton>
                    )}
                </WizardStyles.Toolbar>
            </WizardStyles.WizardWrapper>
        </>
    )
}
