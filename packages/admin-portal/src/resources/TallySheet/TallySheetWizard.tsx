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
} from "@/gql/graphql"
import {CancelButton, NextButton} from "./styles"
import {statusColor} from "../../../../../constants"
import {useTenantStore} from "@/providers/TenantContextProvider"
import DownloadIcon from "@mui/icons-material/Download"
import {ExportElectionMenu} from "@/components/tally/ExportElectionMenu"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {CreateTallySheet} from "./CreateTallySheet"
import {EditTallySheet} from "./EditTallySheet"

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

    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const submitRef = React.useRef<any>(null)

    const {t} = useTranslation()
    const {tallyId, setTallyId, setCreatingFlag} = useElectionEventTallyStore()
    const notify = useNotify()
    const {globalSettings} = useContext(SettingsContext)
    const [tenantId] = useTenantStore()

    const [openModal, setOpenModal] = useState(false)
    const [openCeremonyModal, setOpenCeremonyModal] = useState(false)
    const [page, setPage] = useState<number>(WizardSteps.Edit)
    const [areaId, setAreaId] = useState<Identifier | undefined>()
    const [editedTallySheet, setEditedTallySheet] = useState<
        Sequent_Backend_Tally_Sheet | undefined
    >()
    const [tallySheet, setTallySheet] = useState<Sequent_Backend_Tally_Sheet>()
    const [isButtonDisabled, setIsButtonDisabled] = useState<boolean>(true)
    const [localTallyId, setLocalTallyId] = useState<string | null>(null)

    const [selectedElections, setSelectedElections] = useState<string[]>([])
    const [selectedTrustees, setSelectedTrustees] = useState<boolean>(false)

    const [CreateTallyCeremonyMutation] = useMutation(CREATE_TALLY_CEREMONY)
    const [UpdateTallyCeremonyMutation] = useMutation(UPDATE_TALLY_CEREMONY)

    useEffect(() => {
        if (action) {
            setPage(action)
        }
    }, [action])

    useEffect(() => {
        if (tallySheetId) {
            // if (tally?.last_updated_at !== tallySheetId.last_updated_at) {
            // setPage(
            //     !tallySheetId
            //         ? WizardSteps.Start
            //         : // : tallySheetId.execution_status === ITallyExecutionStatus.STARTED ||
            //           //   tallySheetId.execution_status === ITallyExecutionStatus.CONNECTED
            //           // ? WizardSteps.Ceremony
            //           // : tallySheetId.execution_status === ITallyExecutionStatus.IN_PROGRESS
            //           // ? WizardSteps.Tally
            //           // : tallySheetId.execution_status === ITallyExecutionStatus.SUCCESS
            //           // ? WizardSteps.Results
            //           WizardSteps.Start
            // )
            // setTallySheet(tallySheetId)
            // }
        }
    }, [tallySheetId])

    useEffect(() => {
        if (page === WizardSteps.Start) {
            setIsButtonDisabled(!areaId)
        }
    }, [areaId])

    useEffect(() => {
        if (page === WizardSteps.Edit) {
            setIsButtonDisabled(!!editedTallySheet)
        }
    }, [editedTallySheet])

    // useEffect(() => {
    //     if (page === WizardSteps.Ceremony) {
    //         setIsButtonDisabled(tally?.execution_status !== ITallyExecutionStatus.CONNECTED)
    //     }
    //     if (page === WizardSteps.Tally) {
    //         setIsButtonDisabled(tally?.execution_status !== ITallyExecutionStatus.SUCCESS)
    //     }
    // }, [tally])

    const handleNext = () => {
        if (page === WizardSteps.Start) {
            doAction(WizardSteps.Edit)
            // setIsButtonDisabled(true)
        } else if (page === WizardSteps.Edit) {
            console.log("editedTallySheet :>> ", editedTallySheet)
            submitRef.current?.click()
            // doAction(WizardSteps.Confirm)
            // } else if (page === WizardSteps.Tally) {
            //     setPage(WizardSteps.Results)
            // } else {
            //     setPage(page < 2 ? page + 1 : 0)
        }
    }

    const handleBack = () => {
        if (page === WizardSteps.Edit) {
            doAction(WizardSteps.List)
            // } else if (page === WizardSteps.Edit) {
            //     doAction(WizardSteps.Start)
            // } else if (page === WizardSteps.Tally) {
            //     setPage(WizardSteps.Results)
            // } else {
            //     setPage(page < 2 ? page + 1 : 0)
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
                            contest={contest}
                            doEditedTalySheet={(tallySheet: Sequent_Backend_Tally_Sheet) =>
                                setEditedTallySheet(tallySheet)
                            }
                            submitRef={submitRef}
                        />
                    </>
                )}

                {page === WizardSteps.Confirm && <></>}

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
                                ? t("tallysheet.common.back")
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
