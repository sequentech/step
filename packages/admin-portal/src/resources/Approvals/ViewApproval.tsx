// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {GetUserProfileAttributesQuery, Sequent_Backend_Election_Event} from "@/gql/graphql"
import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {WizardStyles} from "@/components/styles/WizardStyles"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {Accordion, AccordionSummary, CircularProgress} from "@mui/material"
import Table from "@mui/material/Table"
import TableBody from "@mui/material/TableBody"
import TableCell from "@mui/material/TableCell"
import TableContainer from "@mui/material/TableContainer"
import TableRow from "@mui/material/TableRow"
import Paper from "@mui/material/Paper"
import {Identifier, useGetOne} from "react-admin"
import {useQuery} from "@apollo/client"
import {CancelButton} from "../Tally/styles"
import {ListApprovalsMatches} from "./ListApprovalsMatches"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {getAttributeLabel} from "@/services/UserService"
import {USER_PROFILE_ATTRIBUTES} from "@/queries/GetUserProfileAttributes"
import {convertOneToSnakeCase, convertToCamelCase, convertToSnakeCase} from "./UtilsApprovals"
import {IApplicationsStatus} from "@/types/applications"
import {RejectApplicationButton, RejectApplicationDialog} from "./RejectApplication"

export interface ViewApprovalProps {
    electionEventId: string
    electionId?: string
    currApprovalId: Identifier | String | null
    goBack: () => void
    electionEventRecord?: Sequent_Backend_Election_Event
    isModal?: boolean
}

export const ViewApproval: React.FC<ViewApprovalProps> = ({
    electionEventId,
    electionId,
    currApprovalId,
    goBack,
    electionEventRecord,
    isModal = false,
}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const [rejectDialogOpen, setRejectDialogOpen] = useState(false)

    const {data: userAttributes} = useQuery<GetUserProfileAttributesQuery>(
        USER_PROFILE_ATTRIBUTES,
        {
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
            },
        }
    )

    const {data: task, isLoading} = useGetOne("sequent_backend_applications", {id: currApprovalId})

    if (!task || isLoading) {
        return <CircularProgress />
    }

    const renderDetails = () => {
        if (!task.applicant_data || typeof task.applicant_data !== "object") {
            return (
                <TableRow>
                    <TableCell colSpan={2}>{t("common.noData")}</TableCell>
                </TableRow>
            )
        }

        const formatValue = (value: any): string | React.ReactNode => {
            if (value === null || value === undefined) {
                return "-"
            }

            // Handle different data types
            if (value instanceof Date) {
                return value.toLocaleString()
            }
            if (typeof value === "boolean") {
                return value ? "Yes" : "No"
            }
            if (typeof value === "object") {
                return JSON.stringify(value)
            }

            return String(value)
        }

        const userApprovalInfo = Object.entries(convertToSnakeCase(task.applicant_data)).map(
            ([key, value]) => key
        )

        if (userAttributes?.get_user_profile_attributes) {
            const applicantData = userAttributes?.get_user_profile_attributes.map((attr, index) => {
                if (
                    attr &&
                    attr.name &&
                    userApprovalInfo.includes(convertOneToSnakeCase(attr.name))
                ) {
                    const key = getAttributeLabel(attr["display_name"] ?? attr.name)
                    let value = task.applicant_data[convertToCamelCase(attr.name)]
                    return (
                        <TableRow key={index}>
                            <TableCell
                                sx={{
                                    fontWeight: "500",
                                    width: "40%",
                                    textTransform: "capitalize",
                                }}
                            >
                                {/* Try to translate the key, fallback to formatted key if no translation exists */}
                                {t(key)}
                            </TableCell>
                            <TableCell>{formatValue(value)}</TableCell>
                        </TableRow>
                    )
                }
                return null
            })

            task.status === IApplicationsStatus.REJECTED &&
                applicantData.push(
                    <TableRow key={100}>
                        <TableCell
                            sx={{
                                fontWeight: "500",
                                width: "40%",
                                textTransform: "capitalize",
                            }}
                        >
                            {t("approvalsScreen.reject.rejectReason")}
                        </TableCell>
                        <TableCell>
                            {formatValue(
                                t(
                                    `approvalsScreen.reject.reasons.${
                                        task.annotations.rejection_reason ?? "undefined"
                                    }`
                                )
                            )}
                        </TableCell>
                    </TableRow>
                )

            return applicantData
        }

        return []
    }

    const Content = (
        <>
            <Accordion sx={{width: "100%"}} expanded={true}>
                <AccordionSummary expandIcon={false}>
                    <WizardStyles.AccordionTitle>
                        {t("approvalsScreen.approvalRequest")}
                    </WizardStyles.AccordionTitle>
                </AccordionSummary>

                <WizardStyles.AccordionDetails sx={{marginBottom: "3rem"}}>
                    <TableContainer component={Paper}>
                        <Table aria-label="approvals details table">
                            <TableBody>{renderDetails()}</TableBody>
                        </Table>
                    </TableContainer>
                </WizardStyles.AccordionDetails>

                {task.status === IApplicationsStatus.PENDING && (
                    <RejectApplicationButton
                        label={String(t("approvalsScreen.reject.label"))}
                        onClick={setRejectDialogOpen}
                    />
                )}
            </Accordion>
            <ListApprovalsMatches
                electionEventId={electionEventId}
                electionId={electionId}
                task={task}
                goBack={goBack}
            />
        </>
    )

    if (isModal) {
        return (
            <Dialog
                open={true}
                variant="info"
                handleClose={goBack}
                title={String(t("approvalsScreen.taskInformation"))}
                ok={String(t("approvalsScreen.ok"))}
                fullWidth={true}
                maxWidth="md"
            >
                <>{Content}</>
            </Dialog>
        )
    }

    return (
        <WizardStyles.WizardContainer>
            <WizardStyles.ContentWrapper>
                <WizardStyles.ContentBox>{Content}</WizardStyles.ContentBox>
            </WizardStyles.ContentWrapper>

            <WizardStyles.FooterContainer>
                <WizardStyles.StyledFooter>
                    <CancelButton className="list-actions" onClick={goBack}>
                        <ArrowBackIosIcon />
                        {t("common.label.back")}
                    </CancelButton>
                </WizardStyles.StyledFooter>
            </WizardStyles.FooterContainer>

            <RejectApplicationDialog
                electionEventId={electionEventId}
                task={task}
                goBack={goBack}
                rejectDialogOpen={rejectDialogOpen}
                setRejectDialogOpen={setRejectDialogOpen}
            />
        </WizardStyles.WizardContainer>
    )
}
