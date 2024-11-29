// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    GetUserProfileAttributesQuery,
    Sequent_Backend_Applicant_Attributes,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
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
import {convertToCamelCase, convertToSnakeCase} from "./UtilsApprovals"
import {GET_APPLICANT_ATTRIBUTES} from "@/queries/GetApplicantAttributes"

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

    const {data} = useQuery(GET_APPLICANT_ATTRIBUTES, {
        variables: {
            tenantId: tenantId,
            applicationId: currApprovalId,
        },
    })

    if (!task || isLoading) {
        return <CircularProgress />
    }

    const applicant_attributes: Sequent_Backend_Applicant_Attributes[] =
        data?.sequent_backend_applicant_attributes || []

    console.log("applicant_attributes", applicant_attributes)

    const renderDetails = () => {
        if (!applicant_attributes || applicant_attributes.length === 0) {
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

        if (userAttributes?.get_user_profile_attributes) {
            return userAttributes?.get_user_profile_attributes.map((attr, index) => {
                if (
                    attr &&
                    attr.name &&
                    applicant_attributes?.some(
                        (attribute) =>
                            attribute.applicant_attribute_name ===
                            convertToCamelCase(attr.name ?? "")
                    )
                ) {
                    const key = getAttributeLabel(attr["display_name"] ?? "")
                    let value = applicant_attributes.find(
                        (attribute) =>
                            attribute.applicant_attribute_name ===
                            convertToCamelCase(attr.name ?? "")
                    )?.applicant_attribute_value
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
        }

        return []
    }

    const Content = (
        <>
            <Accordion sx={{width: "100%"}} expanded={true}>
                <AccordionSummary expandIcon={false}>
                    <WizardStyles.AccordionTitle>
                        {t("approvalsScreen.approvalInformation")}
                    </WizardStyles.AccordionTitle>
                </AccordionSummary>
                <WizardStyles.AccordionDetails sx={{marginBottom: "3rem"}}>
                    <TableContainer component={Paper}>
                        <Table aria-label="approvals details table">
                            <TableBody>{renderDetails()}</TableBody>
                        </Table>
                    </TableContainer>
                </WizardStyles.AccordionDetails>
            </Accordion>
            {/* <Logs logs={task?.logs} /> */}
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
                title={t("approvalsScreen.taskInformation")}
                ok={t("approvalsScreen.ok")}
                fullWidth={true}
                maxWidth="md"
            >
                {Content}
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
        </WizardStyles.WizardContainer>
    )
}
