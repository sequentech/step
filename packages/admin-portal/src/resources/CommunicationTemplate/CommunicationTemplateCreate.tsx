import React, { useState } from "react"

import styled from "@emotion/styled"

import {Switch} from "@mui/material"
import {RichTextInput} from "ra-input-rich-text"
import {FormControl, Select, MenuItem, InputLabel} from "@mui/material"

import {
    Create,
    SimpleForm,
    TextInput,
    Edit,
    required,
    useTranslate,
    Toolbar,
    SaveButton,
} from "react-admin"

import {Tabs} from "@/components/Tabs"
import {FormStyles} from "@/components/styles/FormStyles"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"

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
    ContainerLanguage: styled.div`

    `,
    Content: styled.div`
        display: flex;
        width: 239px;
        align-items: center;
        justify-content: space-between;
        padding: 0 16px;
    `,
    Text: styled.span`
        text-transform: capitalize;
    `,
}

type TCommunicationTemplateCreate = {
    close?: () => void
}

const CommunicationTemplateByLanguage: React.FC<any> = ({ sources }) => {
    return (
        <CommunicationTemplateCreateStyle.ContainerLanguage>
            <FormStyles.TextInput source={sources.name} label="Name" validate={required()} />

            <RichTextInput source={sources.richText} />
        </CommunicationTemplateCreateStyle.ContainerLanguage>
    )
}

const CommunicationTemplateTitleContainer: React.FC<any> = ({ children, title }) => {
    return (
        <CommunicationTemplateCreateStyle.Box>
            <CommunicationTemplateCreateStyle.BoxTitle>
                { title }
            </CommunicationTemplateCreateStyle.BoxTitle>
            
            {children}
        </CommunicationTemplateCreateStyle.Box>
    )
}

export const CommunicationTemplateCreate: React.FC<TCommunicationTemplateCreate> = () => {
    const [languages, setLanguages] = useState<{[key: string]: boolean}>({
        english: false,
        spanish: false,
    })

    return (
        <Create title={"CREATE"}>
            <SimpleForm>
                <PageHeaderStyles.Title>RECIBO EMAIL AND PDF</PageHeaderStyles.Title>

                <FormStyles.TextInput source="alias" label="Alias" validate={required()} />

                <FormControl fullWidth>
                    <InputLabel id="demo-simple-select-label">Age</InputLabel>

                    <CommunicationTemplateTitleContainer title="Type">
                        <Select
                            labelId="demo-simple-select-label"
                            id="demo-simple-select"
                            value={10}
                            label="Age"
                            onChange={() => null}
                        >
                            <MenuItem value={10}>Ten</MenuItem>
                            <MenuItem value={20}>Twenty</MenuItem>
                            <MenuItem value={30}>Thirty</MenuItem>
                        </Select>
                    </CommunicationTemplateTitleContainer>

                    <CommunicationTemplateTitleContainer title="Languages">
                        {Object.keys(languages).map((lang: string) => (
                            <CommunicationTemplateCreateStyle.Content key={lang}>
                                <CommunicationTemplateCreateStyle.Text>
                                    {lang}
                                </CommunicationTemplateCreateStyle.Text>

                                <Switch
                                    checked={languages?.[lang] || false}
                                    onChange={() => null}
                                />
                            </CommunicationTemplateCreateStyle.Content>
                        ))}
                    </CommunicationTemplateTitleContainer>

                    <Tabs
                        elements={[
                            {
                                label: "English",
                                component: () => <CommunicationTemplateByLanguage
                                    sources={{
                                        name: 'name_en',
                                        richText: 'annotation_en'
                                    }} 
                                />,
                            },
                            {
                                label: "Spanish",
                                component: () => <CommunicationTemplateByLanguage
                                    sources={{
                                        name: 'name_es',
                                        richText: 'annotation_es'
                                    }} 
                                />,
                            },
                        ]}
                    />
                </FormControl>
            </SimpleForm>
        </Create>
    )
}
