import React, {useState, useRef, useEffect} from "react"

import styled from "@emotion/styled"

import {AccordionDetails, AccordionSummary, FormControl, MenuItem} from "@mui/material"

import EditorTextInput from "@/components/Editor"
import EmailEditor from "@/components/EmailEditor"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {
    EditBase,
    Identifier,
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

import {
    ICommunicationType,
    ICommunicationMethod,
    ISendCommunicationBody,
} from "@/types/communications"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {INSERT_COMMUNICATION_TEMPLATE} from "@/queries/InsertCommunicationTemplate"
import EmailEditEditor from '@/components/EmailEditEditor'

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

type TCommunicationTemplateEdit = {
    id?: Identifier | undefined
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

export const CommunicationTemplateEdit: React.FC<TCommunicationTemplateEdit> = (props) => {
    const {id, close} = props

    console.log("CommunicationTemplateEdit", id)

    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()
    const notify = useNotify()

    const [createCommunicationTemplate] = useMutation(INSERT_COMMUNICATION_TEMPLATE)

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

    // const handleSelectChange = async (e: any) => {
    //     const {value, name} = e.target

    //     if (name === "communication_method") {
    //         if (value === ICommunicationMethod.EMAIL) {
    //             communicationTemplate.sms = null
    //         } else {
    //             communicationTemplate.email = null
    //         }
    //     }

    //     setCommunicationTemplate({
    //         ...communicationTemplate,
    //         [name]: value,
    //     })
    // }

    // const handleSmsChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    //     const {value} = e.target

    //     setCommunicationTemplate({
    //         ...communicationTemplate,
    //         sms: value,
    //     })
    // }

    // const handelEmailChange = async (newEmail: any) => {
    //     setCommunicationTemplate({
    //         ...communicationTemplate,
    //         email: newEmail,
    //     })
    // }

    // const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    //     const {value, name} = e.target

    //     setCommunicationTemplate({
    //         ...communicationTemplate,
    //         [name]: value,
    //     })
    // }

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

    useEffect(() => {
        console.log("Communication Template", communicationTemplate)
    }, [communicationTemplate])

    const onSubmit: SubmitHandler<FieldValues> = async (data) => {
        console.log("Submit Template", data)

        // const {errors} = await createCommunicationTemplate({
        //     variables: {
        //         object: {
        //             tenant_id: tenantId,
        //             communication_type: communicationTemplate.communication_type,
        //             communication_method: communicationTemplate.communication_method,
        //             template: {
        //                 ...communicationTemplate,
        //             },
        //         },
        //     },
        // })
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

    function shallowEqual(object1: any, object2: any) {
        const keys1 = Object.keys(object1)
        const keys2 = Object.keys(object2)

        if (keys1.length !== keys2.length) {
            return false
        }

        for (let key of keys1) {
            if (object1[key] !== object2[key]) {
                return false
            }
        }

        return true
    }

    const parseValues = (incoming: any) => {
        const temp = {...incoming}

        if (!shallowEqual(temp.template, communicationTemplate)) {
            setCommunicationTemplate((prev: any) => ({
                ...prev,
                ...temp.template,
            }))
        }

        console.log("parseValues TEMPLATE", {
            ...communicationTemplate,
            ...temp.template,
        })
        console.log("parseValues TEMP", temp)

        return temp
    }

    useEffect(() => {
        console.log("CommunicationTemplateEdit", communicationTemplate)
    }, [communicationTemplate])

    const transform = async (data: any, {previousData}: any) => {
        const temp = {...data}
        console.log("transform", temp);
        
        // return temp
    }

    return (
        <EditBase
            id={id}
            transform={transform}
            resource="sequent_backend_communication_template"
            mutationMode="pessimistic"
            mutationOptions={{onSuccess, onError}}
            redirect={false}
        >
            <PageHeaderStyles.Wrapper>
                <RecordContext.Consumer>
                    {(incoming) => {
                        const parsedValue = parseValues(incoming)
                        console.log("parsedValue :>> ", parsedValue);
                        
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
                                            {parsedValue.communication_method ===
                                                ICommunicationMethod.EMAIL && (
                                                <EmailEditEditor record={parsedValue} />
                                            )}
                                            {parsedValue.communication_method ===
                                                ICommunicationMethod.SMS && (
                                                <FormStyles.TextInput
                                                    minRows={4}
                                                    multiline={true}
                                                    // onChange={handleSmsChange}
                                                    source="template.sms"
                                                    label={t(
                                                        "communicationTemplate.form.smsMessage"
                                                    )}
                                                />
                                            )}
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
