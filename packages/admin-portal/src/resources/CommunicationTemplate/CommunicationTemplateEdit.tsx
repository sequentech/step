// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"

import styled from "@emotion/styled"

import {AccordionDetails, AccordionSummary, FormControl} from "@mui/material"

import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {
    EditBase,
    Identifier,
    RaRecord,
    RecordContext,
    SaveButton,
    SelectInput,
    SimpleForm,
    required,
    useNotify,
    useRefresh,
} from "react-admin"

import {FieldValues, SubmitHandler} from "react-hook-form"
import {FormStyles} from "@/components/styles/FormStyles"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {useMutation} from "@apollo/client"

import {ICommunicationType, ICommunicationMethod} from "@/types/communications"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import EmailEditEditor from "@/components/EmailEditEditor"
import {Sequent_Backend_Communication_Template} from "@/gql/graphql"
import {useWatch} from "react-hook-form"
import {UPDATE_COMMUNICATION_TEMPLATE} from "@/queries/UpdateCommunicationTemplate"
import {ContentInput} from "./CommunicationTemplateCreate"

const CommunicationTemplateCreateStyle = {
    Box: styled.div`
        display: flex;
        flex-direction: column;
        border: 1px solid #f2f2f2;
        padding: 8px;
        border-radius: 4px;
        margin-bottom: 16px;
    `,
    BoxTitle: styled.span`
        font-family: Roboto;
        font-size: 24px;
        font-weight: 700;
        line-height: 32px;
        letter-spacing: 0px;
        text-align: left;
        padding: 0 16px;
    `,
    ContainerLanguage: styled.div``,
    RichText: styled.div`
        border: 1px solid #f2f2f2;
        border-radius: 4px;
        width: 100%;
        padding: 0;
    `,
}

const CommunicationTemplateTitleContainer: React.FC<any> = ({children, title}) => {
    return (
        <CommunicationTemplateCreateStyle.Box>
            <CommunicationTemplateCreateStyle.BoxTitle>
                {title}
            </CommunicationTemplateCreateStyle.BoxTitle>

            {children}
        </CommunicationTemplateCreateStyle.Box>
    )
}

type TCommunicationTemplateEdit = {
    id?: Identifier | undefined
    close?: () => void
}

export const CommunicationTemplateEdit: React.FC<TCommunicationTemplateEdit> = (props) => {
    const {id, close} = props

    console.log("CommunicationTemplateEdit", id)

    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()
    const notify = useNotify()

    const [UpdateCommunicationTemplate] = useMutation(UPDATE_COMMUNICATION_TEMPLATE)

    const communicationTypeChoices = () => {
        return (Object.values(ICommunicationType) as ICommunicationType[]).map((value) => ({
            id: value,
            name: t(`communicationTemplate.type.${value.toLowerCase()}`),
        }))
    }

    const communicationMethodChoices = () => {
        return (Object.values(ICommunicationMethod) as ICommunicationMethod[]).map((value) => ({
            id: value,
            name: t(`communicationTemplate.method.${value.toLowerCase()}`),
        }))
    }

    const onSubmit: SubmitHandler<FieldValues> = async (data) => {
        console.log("Submit Template", data)

        const {data: updated, errors} = await UpdateCommunicationTemplate({
            variables: {
                id: id,
                tenantId: tenantId,
                set: {...data},
            },
        })

        if (updated) {
            notify("communicationTemplate.update.success", {type: "success"})
        }

        if (errors) {
            notify("communicationTemplate.update.error", {type: "error"})
        }

        close?.()
    }

    const onSuccess = async (res: any) => {
        console.log("onSuccess :>> ", res)

        refresh()
        notify("Area updated", {type: "success"})
        if (close) {
            setTimeout(() => {
                close()
            }, 400)
        }
    }

    const onError = async (res: any) => {
        console.log("onError :>> ", res)

        refresh()
        notify("Could not update Area", {type: "error"})
        if (close) {
            setTimeout(() => {
                close()
            }, 400)
        }
    }

    const parseValues = (incoming: RaRecord<Identifier> | Omit<RaRecord<Identifier>, "id">) => {
        const temp = {...(incoming as Sequent_Backend_Communication_Template)}
        return temp
    }

    return (
        <EditBase
            id={id}
            resource="sequent_backend_communication_template"
            mutationMode="pessimistic"
            mutationOptions={{onSuccess, onError}}
            redirect={false}
        >
            <PageHeaderStyles.Wrapper>
                <RecordContext.Consumer>
                    {(incoming) => {
                        const parsedValue: RaRecord<Identifier> | Omit<RaRecord<Identifier>, "id"> =
                            parseValues(incoming)
                        console.log("parsedValue :>> ", parsedValue)

                        return (
                            <SimpleForm
                                record={parsedValue}
                                onSubmit={onSubmit}
                                toolbar={<SaveButton />}
                            >
                                <PageHeaderStyles.Title>
                                    {t("communicationTemplate.edit.title")}
                                </PageHeaderStyles.Title>

                                <FormStyles.TextInput
                                    source="template.alias"
                                    validate={required()}
                                    // onChange={handleInputChange}
                                    label={t("communicationTemplate.form.alias")}
                                />
                                <FormStyles.TextInput
                                    source="template.name"
                                    validate={required()}
                                    // onChange={handleInputChange}
                                    label={t("communicationTemplate.form.name")}
                                />
                                <FormControl fullWidth>
                                    <CommunicationTemplateTitleContainer
                                        title={t("communicationTemplate.form.communicationType")}
                                    >
                                        <SelectInput
                                            source="communication_type"
                                            validate={required()}
                                            choices={communicationTypeChoices()}
                                        />
                                    </CommunicationTemplateTitleContainer>

                                    <FormStyles.AccordionExpanded expanded={true} disableGutters>
                                        <AccordionSummary
                                            expandIcon={
                                                <ExpandMoreIcon id="communication-template-method-id" />
                                            }
                                        >
                                            <ElectionHeaderStyles.Wrapper>
                                                <ElectionHeaderStyles.Title>
                                                    {t(
                                                        "communicationTemplate.form.communicationMethod"
                                                    )}
                                                </ElectionHeaderStyles.Title>
                                            </ElectionHeaderStyles.Wrapper>
                                        </AccordionSummary>
                                        <AccordionDetails>
                                            <SelectInput
                                                source="communication_method"
                                                validate={required()}
                                                choices={communicationMethodChoices()}
                                            />
                                            <ContentInput />
                                        </AccordionDetails>
                                    </FormStyles.AccordionExpanded>
                                </FormControl>
                            </SimpleForm>
                        )
                    }}
                </RecordContext.Consumer>
            </PageHeaderStyles.Wrapper>
        </EditBase>
    )
}
