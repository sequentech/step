import React, {useState, useRef, useEffect} from "react"

import styled from "@emotion/styled"

import {AccordionDetails, AccordionSummary, FormControl, MenuItem} from "@mui/material"

import EditorTextInput from "@/components/Editor"
import EmailEditor from "@/components/EmailEditor"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {Create, SimpleForm, required, useNotify} from "react-admin"

import {FieldValues, SubmitHandler} from "react-hook-form"
import {FormStyles} from "@/components/styles/FormStyles"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {useMutation} from "@apollo/client"

import {
    ICommunicationType,
    ICommunicationMethod,
    ISendCommunicationBody,
} from "@/types/communications"
import { useTranslation } from 'react-i18next'
import { useTenantStore } from '@/providers/TenantContextProvider'
import { INSERT_COMMUNICATION_TEMPLATE } from "@/queries/InsertCommunicationTemplate"

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

type TCommunicationTemplateCreate = {
    close?: () => void
}

const CommunicationTemplateByLanguage: React.FC<any> = ({sources, editorRef}) => {
    return (
        <CommunicationTemplateCreateStyle.ContainerLanguage>
            <FormStyles.TextInput source={sources.name} label="Name" validate={required()} />

            <EditorTextInput initialValue="" editorRef={editorRef} onEditorChange={console.log} />
        </CommunicationTemplateCreateStyle.ContainerLanguage>
    )
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

export const CommunicationTemplateCreate: React.FC<TCommunicationTemplateCreate> = ({close}) => {
    const editorRef = useRef()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const [createCommunicationTemplate] = useMutation(INSERT_COMMUNICATION_TEMPLATE) // <InsertCommunicationTemplateMutation>

    const [communicationTemplate, setCommunicationTemplate] = useState<any>({
        audience_selection: null,
        audience_voter_ids: [],
        communication_type: ICommunicationType.CREDENTIALS,
        communication_method: ICommunicationMethod.EMAIL,
        schedule_now: null,
        schedule_date: null,
        email: null,
        sms: null,
    })

    const handleSelectChange = async (e: any) => {
        const {value, name} = e.target

        if (name === 'communication_method') {
            if (value === ICommunicationMethod.EMAIL) {
                communicationTemplate.sms = null
            } else {
                communicationTemplate.email = null
            }
        }

        setCommunicationTemplate({
            ...communicationTemplate,
            [name]: value,
        })
    }

    const handleSmsChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {value} = e.target

        setCommunicationTemplate({
            ...communicationTemplate,
            sms: value,
        })
    }

    const handelEmailChange = async (newEmail: any) => {
        setCommunicationTemplate({
            ...communicationTemplate,
            email: newEmail,
        })
    }

    const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        const {value, name} = e.target

        setCommunicationTemplate({
            ...communicationTemplate,
            [name]: value
        })
    }

    useEffect(() => {
        console.log('Communication Template', communicationTemplate)
    }, [communicationTemplate])

    const onSubmit: SubmitHandler<FieldValues> = async (data) => {
        console.log("Communication Template", data)

        const {errors} = await createCommunicationTemplate({
            variables: {
                object: {
                    tenant_id: tenantId,
                    communication_type: communicationTemplate.communication_type,
                    communication_method: communicationTemplate.communication_method,
                    template: {
                        ...communicationTemplate,
                    }
                }
            }
        })
        close?.()
    }

    return (
        <Create>
            <SimpleForm onSubmit={onSubmit}>
                <PageHeaderStyles.Title>
                    {t('communicationTemplate.create.title')}
                </PageHeaderStyles.Title>

                <FormStyles.TextInput 
                    source="alias" 
                    validate={required()} 
                    onChange={handleInputChange} 
                    label={t('communicationTemplate.form.alias')} 
                />

                <FormStyles.TextInput 
                    source="name" 
                    validate={required()} 
                    onChange={handleInputChange} 
                    label={t('communicationTemplate.form.name')} 
                />

                <FormControl fullWidth>
                    <CommunicationTemplateTitleContainer title={t('communicationTemplate.form.communicationType')}>
                        <FormStyles.Select 
                            name="communication_type"
                            onChange={handleSelectChange}
                            value={communicationTemplate.communication_type}
                        >
                            {(Object.values(ICommunicationType) as ICommunicationType[]).map((value) => (
                                <MenuItem key={value} value={value}>
                                    {t(`communicationTemplate.type.${value.toLowerCase()}`)}
                                </MenuItem>
                            ))}
                        </FormStyles.Select>
                    </CommunicationTemplateTitleContainer>

                    <FormStyles.AccordionExpanded expanded={true} disableGutters>
                    <AccordionSummary
                        expandIcon={<ExpandMoreIcon id="communication-template-method-id" />}
                    >
                        <ElectionHeaderStyles.Wrapper>
                            <ElectionHeaderStyles.Title>
                                {t("communicationTemplate.form.communicationMethod")}
                            </ElectionHeaderStyles.Title>
                        </ElectionHeaderStyles.Wrapper>
                    </AccordionSummary>
                    <AccordionDetails>
                        <FormStyles.Select
                            name="communication_method"
                            onChange={handleSelectChange}
                            value={communicationTemplate.communication_method}
                        >
                            {(Object.keys(ICommunicationMethod) as ICommunicationMethod[]).map(
                                (key) => (
                                    <MenuItem key={key} value={key}>
                                        {t(`communicationTemplate.method.${key.toLowerCase()}`)}
                                    </MenuItem>
                                )
                            )}
                        </FormStyles.Select>
                        {communicationTemplate.communication_method === ICommunicationMethod.EMAIL && (
                                <EmailEditor
                                    record={communicationTemplate.email}
                                    setRecord={handelEmailChange}
                                />
                            )}
                        {communicationTemplate.communication_method === ICommunicationMethod.SMS && (
                            <FormStyles.TextField
                                name="sms"
                                minRows={4}
                                multiline={true}
                                onChange={handleSmsChange}
                                value={communicationTemplate.sms}
                                label={t("communicationTemplate.form.smsMessage")}
                            />
                        )}
                    </AccordionDetails>
                </FormStyles.AccordionExpanded>
                </FormControl>
            </SimpleForm>
        </Create>
    )
}
