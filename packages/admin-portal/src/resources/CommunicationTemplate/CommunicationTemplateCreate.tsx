import React, {useState, useRef, useEffect} from "react"

import styled from "@emotion/styled"

import {AccordionDetails, AccordionSummary, FormControl, MenuItem} from "@mui/material"

import EditorTextInput from "@/components/Editor"
import EmailEditor from "@/components/EmailEditor"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {Create, SimpleForm, required} from "react-admin"

import {FormStyles} from "@/components/styles/FormStyles"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"

import {
    ICommunicationType,
    ICommunicationMethod,
    ISendCommunicationBody,
} from "@/types/communications"
import { useTranslation } from 'react-i18next'

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

export const CommunicationTemplateCreate: React.FC<TCommunicationTemplateCreate> = () => {
    const editorRef = useRef()
    const {t} = useTranslation()

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

    const onTransform = (data: any) => {
        console.log("COMMUNICATION TEMPLATE => CREATE", data)

        return {
            communication_type: communicationTemplate.communication_type,
            communication_method: communicationTemplate.communication_method,
            template: {
                ...communicationTemplate,
            }
        }
    }

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

    return (
        <Create title={"CREATE"} transform={onTransform}>
            <SimpleForm>
                <PageHeaderStyles.Title>RECIBO EMAIL AND PDF</PageHeaderStyles.Title>

                <FormStyles.TextInput source="alias" label="Alias" validate={required()} onChange={handleInputChange} />

                <FormStyles.TextInput source="name" label="Name" validate={required()} onChange={handleInputChange} />

                <FormControl fullWidth>
                    <CommunicationTemplateTitleContainer title="Type">
                        <FormStyles.Select 
                            name="communication_type"
                            onChange={handleSelectChange}
                            value={communicationTemplate.communication_type}
                        >
                            {Object.values(ICommunicationType).map((value) => (
                                <MenuItem key={value} value={value}>
                                    {value}
                                </MenuItem>
                            ))}
                        </FormStyles.Select>
                    </CommunicationTemplateTitleContainer>

                    <FormStyles.AccordionExpanded expanded={true} disableGutters>
                    <AccordionSummary
                        expandIcon={<ExpandMoreIcon id="send-communication-method" />}
                    >
                        <ElectionHeaderStyles.Wrapper>
                            <ElectionHeaderStyles.Title>
                                {t("sendCommunication.methodTitle")}
                            </ElectionHeaderStyles.Title>
                        </ElectionHeaderStyles.Wrapper>
                    </AccordionSummary>
                    <AccordionDetails>
                        <FormStyles.Select
                            name="communication_method"
                            onChange={handleSelectChange}
                            value={communicationTemplate.communication_method}
                        >
                            {(Object.keys(ICommunicationMethod) as Array<ICommunicationMethod>).map(
                                (key) => (
                                    <MenuItem key={key} value={key}>
                                        {t(`sendCommunication.communicationMethod.${key}`)}
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
                                label={t("sendCommunication.smsMessage")}
                                value={communicationTemplate.sms}
                                onChange={handleSmsChange}
                                multiline={true}
                                minRows={4}
                            />
                        )}
                    </AccordionDetails>
                </FormStyles.AccordionExpanded>
                </FormControl>
            </SimpleForm>
        </Create>
    )
}
