// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import {
    BreadCrumbSteps,
    BreadCrumbStepsVariant,
    Dialog,
    sleep,
    theme,
} from "@sequentech/ui-essentials"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import {useTranslation} from "react-i18next"
import ElectionHeader from "@/components/ElectionHeader"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {Accordion, AccordionSummary, Button} from "@mui/material"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {ListActions} from "@/components/ListActions"
import {TallyStyles} from "@/components/styles/TallyStyles"
// import {TallyElectionsList} from "./TallyElectionsList"
// import {TallyTrusteesList} from "./TallyTrusteesList"
// import {TallyStartDate} from "./TallyStartDate"
// import {TallyElectionsProgress} from "./TallyElectionsProgress"
// import {TallyElectionsResults} from "./TallyElectionsResults"
// import {TallyResults} from "./TallyResults"
// import {TallyLogs} from "./TallyLogs"
import {Identifier, useGetList, useGetOne, useNotify, useRecordContext} from "react-admin"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {UPDATE_TALLY_CEREMONY} from "@/queries/UpdateTallyCeremony"
import {CREATE_TALLY_CEREMONY} from "@/queries/CreateTallyCeremony"
import {useMutation} from "@apollo/client"
import {ILog, ITallyExecutionStatus} from "@/types/ceremonies"
import {
    Sequent_Backend_Contest,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
    Sequent_Backend_Tally_Sheet,
    Sequent_Backend_Tally_Sheet_Insert_Input,
} from "@/gql/graphql"
import {CancelButton, NextButton} from "./styles"
import {statusColor} from "../../../../../constants"
import {useTenantStore} from "@/providers/TenantContextProvider"
import DownloadIcon from "@mui/icons-material/Download"
import {ExportElectionMenu} from "@/components/tally/ExportElectionMenu"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {CreateTallySheet} from "./CreateTallySheet"
import {EditTallySheet} from "./EditTallySheet"
import {ShowTallySheet} from "./ShowTallySheet"

export const WizardSteps = {
    List: -1,
    Start: 0,
    Edit: 1,
    Confirm: 2,
    View: 3,
}

interface IExpanded {
    [key: string]: boolean
}

interface TallySheetWizardProps {
    tallySheetId?: Identifier | undefined
    contest: Sequent_Backend_Contest
    action: number
    doAction: (action: number, id?: Identifier) => void
}

export const TallySheetWizard: React.FC<TallySheetWizardProps> = (props) => {
    const {action, contest, tallySheetId, doAction} = props

    const submitRef = React.useRef<any>(null)

    const {t} = useTranslation()
    const {tallyId, setTallyId, setCreatingFlag} = useElectionEventTallyStore()
    const notify = useNotify()
    const [tenantId] = useTenantStore()

    const [openModal, setOpenModal] = useState(false)
    const [openCeremonyModal, setOpenCeremonyModal] = useState(false)
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
        if (page === WizardSteps.Edit) {
            submitRef.current?.click()
            doAction(WizardSteps.Confirm)
        } else if (page === WizardSteps.Confirm) {
            submitRef.current?.click()
            doAction(WizardSteps.List)
        }
    }

    const handleBack = () => {
        if (page === WizardSteps.Edit) {
            doAction(WizardSteps.List)
        } else if (page === WizardSteps.Confirm) {
            doAction(WizardSteps.Edit)
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
                        <CreateTallySheet
                            contest={contest}
                            doSelectArea={(id: Identifier) => setAreaId(id)}
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
                            ) => setCreatedTallySheet(tallySheet)}
                            submitRef={submitRef}
                        />
                    </>
                )}

                {page === WizardSteps.Confirm && (
                    <>
                        <ShowTallySheet
                            tallySheet={createdTallySheet}
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
                    {/* {page < WizardSteps.Results && ( */}
                    <NextButton color="primary" onClick={handleNext} disabled={isButtonDisabled}>
                        <>
                            {page === WizardSteps.Edit
                                ? t("tallysheet.common.confirm")
                                : page === WizardSteps.Confirm
                                ? t("tallysheet.common.save")
                                : t("tallysheet.common.next")}
                            <ChevronRightIcon />
                        </>
                    </NextButton>
                    {/* )} */}
                </TallyStyles.StyledFooter>
            </WizardStyles.WizardWrapper>

            {/* <Dialog
                variant="info"
                open={openModal}
                ok={t("tally.common.dialog.ok")}
                cancel={t("tally.common.dialog.cancel")}
                title={t("tally.common.dialog.title")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmStartAction()
                    }
                    setOpenModal(false)
                }}
            >
                {t("tally.common.dialog.message")}
            </Dialog> */}

            {/* <Dialog
                variant="info"
                open={openCeremonyModal}
                ok={t("tally.common.dialog.okTally")}
                cancel={t("tally.common.dialog.cancel")}
                title={t("tally.common.dialog.tallyTitle")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmCeremonyAction()
                    }
                    setOpenCeremonyModal(false)
                }}
            >
                {t("tally.common.dialog.ceremony")}
            </Dialog> */}
        </>
    )
}
