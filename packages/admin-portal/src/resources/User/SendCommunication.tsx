// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, { useState } from "react"
import {
    SaveButton,
    SimpleForm,
    useListContext,
    Toolbar,
    DateTimeInput,
} from "react-admin"
import {
    AccordionDetails,
    AccordionSummary,
    MenuItem,
    FormControlLabel,
    Switch,
    Grid,
} from "@mui/material"
import {SubmitHandler} from "react-hook-form"
import MailIcon from '@mui/icons-material/Mail'
import ExpandMoreIcon from '@mui/icons-material/ExpandMore'
import {useTenantStore} from "@/providers/TenantContextProvider"
import { PageHeaderStyles } from "@/components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import { FormStyles } from "@/components/styles/FormStyles"
import { ElectionHeaderStyles } from "@/components/styles/ElectionHeaderStyles"

enum IVotersSelection {
    ALL_USERS = "ALL_USERS",
    NOT_VOTED = "NOT_VOTED",
    VOTED = "VOTED",
    SELECTED = "SELECTED",
}

interface ICommunication {
    voters: {
        selection: IVotersSelection,
        voter_ids?: Array<string>,
    }
    communication_type: string,
    communication_methods: {
        email: boolean,
        sms: boolean,
    }
    schedule: {
        now: boolean
        date?: Date
    }
    i18n: {
        [lang_code: string]: {
            email?: {
                subject: string
                plaintext_body: string
                html_body: string
            },
            sms?: {
                message: string
            }
        }
    }
    language_conf: {
        enabled_languages: Array<string>
        default_language_code: string
    }
}

interface SendCommunicationProps {
    id?: string
    electionEventId?: string
    close?: () => void
}

export const SendCommunication: React.FC<SendCommunicationProps> = ({
    id, close, electionEventId
}) => {
    const {data, isLoading} = useListContext()
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const possibleLanguages = ["en", "es"]
    const [communication, setCommunication] = useState<ICommunication>({
        voters: {
            selection: IVotersSelection.SELECTED,
            voter_ids: [id ?? ""],
        },
        communication_type: "credentials",
        communication_methods: {
            email: true,
            sms: true,
        },
        schedule: {
            now: true,
            date: undefined,
        },
        i18n: {
            en: {
                email: {
                    subject: "",
                    plaintext_body: "",
                    html_body: "",
                },
                sms: {
                    message: ""
                },
            },
            es: {
                email: {
                    subject: "",
                    plaintext_body: "",
                    html_body: "",
                },
                sms: {
                    message: ""
                },
            },
        },
        language_conf: {
            enabled_languages: ["en"],
            default_language_code: "en"
        },
    })
    //const [sendCommunication] = useMutation<SendCommunicationMutationVariables>(SEND_COMMUNICATION)


    const onSubmit: SubmitHandler<any> = async () => {
        console.log("sending notification")
    }

    const handleNowChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {checked} = e.target
        var newCommunication = {...communication}
        newCommunication.schedule.now = checked
        setCommunication(newCommunication)
    }
    const handleSelectChange = async (e: any) => {
        const {value} = e.target
        var newCommunication = {...communication}
        newCommunication.voters.selection = value
        setCommunication(newCommunication)
    }
    const handleLangChange = 
        (lang:string) =>
        async (e: React.ChangeEvent<HTMLInputElement>) =>
    {
        const {checked} = e.target
        var newCommunication = {...communication}
        if (checked) {
            // Add the language if it's not already in the array
            if (!newCommunication
                .language_conf
                .enabled_languages
                .includes(lang)
            ) {
                newCommunication.language_conf.enabled_languages.push(lang)
            }
        } else {
            // Remove the language if it's in the array
            newCommunication.language_conf.enabled_languages = 
                newCommunication
                .language_conf
                .enabled_languages
                .filter(l => l !== lang)
        }
        setCommunication(newCommunication)
    }

    const renderLangs = () => {
        let langNodes = []
        for (let lang of possibleLanguages) {
            let checked = communication.language_conf.enabled_languages.includes(lang)
            console.log(`lang(${lang}) in communication.language_conf.enabled_languages(${communication.language_conf.enabled_languages}) = checked(${checked})`)
            langNodes.push(
                <FormControlLabel
                    sx={{width: "100%"}}
                    label={t(`common.language.${lang}`)}
                    control={<Switch
                        checked={checked}
                        onChange={handleLangChange(lang)}
                    />}
                />
            )
        }
        return <div>{langNodes}</div>
    }


    return (
        <PageHeaderStyles.Wrapper>
            <SimpleForm
                toolbar={<Toolbar>
                    <SaveButton 
                        icon={<MailIcon />}
                        label={t("sendCommunication.sendButton")}
                        alwaysEnable
                    />
                </Toolbar>}
                record={communication}
                onSubmit={onSubmit}
                sanitizeEmptyValues
            >
                <PageHeaderStyles.Title>
                    {t(`sendCommunication.title`)}
                </PageHeaderStyles.Title>
                <PageHeaderStyles.SubTitle>
                    {t(`sendCommunication.subtitle`)}
                </PageHeaderStyles.SubTitle>

                {/* Voters */}
                <FormStyles.AccordionExpanded expanded={true} disableGutters>
                    <AccordionSummary
                        expandIcon={<ExpandMoreIcon
                            id="send-communication-voters"
                        />}
                    >
                        <ElectionHeaderStyles.Wrapper>
                            <ElectionHeaderStyles.Title>
                                {t("sendCommunication.voters")}
                            </ElectionHeaderStyles.Title>
                        </ElectionHeaderStyles.Wrapper>
                    </AccordionSummary>
                    <AccordionDetails>
                        <FormStyles.Select
                            name="voters.selection"
                            value={communication.voters.selection}
                            onChange={handleSelectChange}
                        >
                            {(Object.keys(IVotersSelection) as Array<IVotersSelection>)
                                .map((key) => (
                                    <MenuItem key={key} value={key}>
                                        {t(`sendCommunication.votersSelection.${key}`)}
                                    </MenuItem>
                                ))
                            }
                        </FormStyles.Select>
                    </AccordionDetails>
                </FormStyles.AccordionExpanded>

                {/* Schedule */}
                <FormStyles.AccordionExpanded expanded={true} disableGutters>
                    <AccordionSummary
                        expandIcon={<ExpandMoreIcon
                            id="send-communication-schedule"
                        />}
                    >
                        <ElectionHeaderStyles.Wrapper>
                            <ElectionHeaderStyles.Title>
                                {t("sendCommunication.schedule")}
                            </ElectionHeaderStyles.Title>
                        </ElectionHeaderStyles.Wrapper>
                    </AccordionSummary>
                    <AccordionDetails>
                        <FormControlLabel
                            label={t("sendCommunication.nowInput")}
                            control={<Switch
                                checked={communication.schedule.now}
                                onChange={handleNowChange}
                            />}
                        />
                        <DateTimeInput
                            disabled={communication.schedule.now}
                            source="schedule.date"
                            label={t("sendCommunication.dateInput")}
                            parse={(value) => new Date(value).toISOString()}
                        />

                    </AccordionDetails>
                </FormStyles.AccordionExpanded>

                {/* Languages */}
                <FormStyles.AccordionExpanded expanded={true} disableGutters>
                    <AccordionSummary
                        expandIcon={<ExpandMoreIcon
                            id="send-communication-languages"
                        />}
                    >
                        <ElectionHeaderStyles.Wrapper>
                            <ElectionHeaderStyles.Title>
                                {t("sendCommunication.languages")}
                            </ElectionHeaderStyles.Title>
                        </ElectionHeaderStyles.Wrapper>
                    </AccordionSummary>
                    <AccordionDetails>
                        <Grid container spacing={4}>
                            <Grid item xs={12} md={6}>
                                {renderLangs()}
                            </Grid>
                        </Grid>
                    </AccordionDetails>
                </FormStyles.AccordionExpanded>
            </SimpleForm>
        </PageHeaderStyles.Wrapper>
    )
}
