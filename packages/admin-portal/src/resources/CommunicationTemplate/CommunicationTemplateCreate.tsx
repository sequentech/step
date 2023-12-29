import React, {useState, useRef} from "react"

import styled from "@emotion/styled"

import {FormControl, MenuItem} from "@mui/material"
// import {ICommunicationType, ICommunicationMethod, ISendCommunicationBody} from "sequent-core"

import EditorTextInput from "@/components/Editor"

import {Create, SimpleForm, required} from "react-admin"

import {FormStyles} from "@/components/styles/FormStyles"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {
    ICommunicationType,
    ICommunicationMethod,
    ISendCommunicationBody,
} from "@/types/communications"

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

    return (
        <Create title={"CREATE"} transform={onTransform}>
            <SimpleForm>
                <PageHeaderStyles.Title>RECIBO EMAIL AND PDF</PageHeaderStyles.Title>

                <FormStyles.TextInput source="alias" label="Alias" validate={required()} />

                <FormStyles.TextInput source="name" label="Name" validate={required()} />

                <FormControl fullWidth>
                    <CommunicationTemplateTitleContainer title="Type">
                        <FormStyles.Select value={10} onChange={() => null}>
                            {Object.values(ICommunicationType).map((value) => (
                                <MenuItem key={value} value={value}>
                                    {value}
                                </MenuItem>
                            ))}
                        </FormStyles.Select>
                    </CommunicationTemplateTitleContainer>

                    <CommunicationTemplateTitleContainer title="Method">
                        <FormStyles.Select value={10} onChange={() => null}>
                            {Object.values(ICommunicationMethod).map((value) => (
                                <MenuItem key={value} value={value}>
                                    {value}
                                </MenuItem>
                            ))}
                        </FormStyles.Select>
                    </CommunicationTemplateTitleContainer>

                    <CommunicationTemplateByLanguage
                        editorRef={editorRef}
                        sources={{
                            name: "name",
                            richText: "template",
                        }}
                    />
                </FormControl>
            </SimpleForm>
        </Create>
    )
}
