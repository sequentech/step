// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import SelectElection from "@/components/election/SelectElection"
import {EReportElectionPolicy, EReportType, ReportActions, reportTypeConfig} from "@/types/reports"
import {Typography, Autocomplete, Chip, TextField, Box, InputLabel} from "@mui/material"
import React, {useContext, useEffect, useMemo, useState} from "react"
import {
    BooleanInput,
    Create,
    Identifier,
    SaveButton,
    SelectInput,
    SimpleForm,
    Toolbar,
    useGetOne,
    useNotify,
    useInput,
    InputProps,
    AutocompleteArrayInput,
    choices,
} from "react-admin"
import SelectTemplate from "../Template/SelectTemplate"
import {useTranslation} from "react-i18next"
import {EncryptReportMutation, Sequent_Backend_Report} from "@/gql/graphql"
import {useMutation} from "@apollo/client"
import {CREATE_REPORT} from "@/queries/CreateReport"
import {UPDATE_REPORT} from "@/queries/UpdateReport"
import {ETemplateType} from "@/types/templates"
import {useFormContext} from "react-hook-form"
import {Cron} from "react-js-cron"
import "react-js-cron/dist/styles.css"
import {ENCRYPT_REPORT} from "@/queries/EncryptReport"
import {IPermissions} from "@/types/keycloak"
import {CustomAutocompleteArrayInput, Dialog} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import {AuthContext} from "@/providers/AuthContextProvider"
import {FormStyles} from "@/components/styles/FormStyles"

type Choice = {
    id: string
    name: string
}

interface CreateReportProps {
    close?: () => void
    electionEventId: string | undefined
    tenantId: string | null
    isEditReport: boolean
    reportId?: Identifier | null | undefined
    report?: Sequent_Backend_Report | null | undefined
    doCronActive?: (isActive: boolean) => void
    cronValue?: string
    setCronValue?: (v: string) => void
    setEnabled?: (v: boolean) => void
}

export enum EReportEncryption {
    UNENCRYPTED = "unencrypted",
    CONFIGURED_PASSWORD = "configured_password",
}

const InputLabelStyle = styled(InputLabel)<{paddingTop?: boolean}>`
    width: 135px;
    ${({paddingTop = true}) => paddingTop && "padding-top: 15px;"}
`

const InputContainerStyle = styled(Box)`
    display: flex;
    gap: 12px;
    width: 100%;
    align-items: baseline;
    @media (max-width: ${({theme}) => theme.breakpoints.values.sm}px) {
        flex-direction: column;
    }
`

const PasswordInputStyle = styled(FormStyles.PasswordInput)`
    flex: 1;
    margin: 0 auto;
`

interface PasswordComponentProps {
    report?: Sequent_Backend_Report
    setPassword: (val: string | null) => void
    reportEncryptionPolicy?: EReportEncryption
    setReportEncryptionPolicy: (val: EReportEncryption) => void
}

interface PasswordInputData {
    password: string
    confirmPassword: string
}

const getDefaultPasswordInputData = () => ({
    password: "",
    confirmPassword: "",
})

const PasswordComponent: React.FC<PasswordComponentProps> = ({
    report,
    setPassword,
    reportEncryptionPolicy,
    setReportEncryptionPolicy,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const {setValue} = useFormContext()
    const [isDialogOpen, setIsDialogOpen] = useState<boolean>(false)
    const [filePassword, setFilePassword] = useState<PasswordInputData>(
        getDefaultPasswordInputData()
    )

    const reportEncryptionPolicyChoices = Object.keys(EReportEncryption).map((key) => ({
        id: EReportEncryption[key as keyof typeof EReportEncryption],
        name: t(`reportsScreen.reportEncryptionPolicy.${key}`),
    }))

    const onChangePolicy = (value: EReportEncryption) => {
        if (reportEncryptionPolicy === value) {
            return
        }
        if (EReportEncryption.UNENCRYPTED === value) {
            setReportEncryptionPolicy(value)
            setPassword(null)
        } else {
            setIsDialogOpen(true)
        }
    }

    const checkIsValidPassword = () =>
        filePassword.password === filePassword.confirmPassword && !!filePassword.password

    const handleCloseDialog = (value: boolean) => {
        const isValidPassword = checkIsValidPassword()
        if (!value || !isValidPassword) {
            setPassword(null)
            setReportEncryptionPolicy(EReportEncryption.UNENCRYPTED)
            setValue("encryption_policy", EReportEncryption.UNENCRYPTED)

            if (!isValidPassword && filePassword.password) {
                notify(t("reportsScreen.messages.passwordMismatch"), {type: "error"})
            }
        } else {
            setPassword(filePassword.password)
            setReportEncryptionPolicy(EReportEncryption.CONFIGURED_PASSWORD)
            setValue("encryption_policy", EReportEncryption.CONFIGURED_PASSWORD)
        }
        setIsDialogOpen(false)
        setFilePassword(getDefaultPasswordInputData())
    }

    const handleChangePassword = (event: React.ChangeEvent<HTMLInputElement>) => {
        setFilePassword({
            ...filePassword,
            password: event.target.value,
        })
    }

    const handleChangeConfirmPassword = (event: React.ChangeEvent<HTMLInputElement>) => {
        setFilePassword({
            ...filePassword,
            confirmPassword: event.target.value,
        })
    }

    const equalToPassword = (value: any, allValues: any) => {
        if (!allValues.password || allValues.password.length == 0) {
            return
        }
        if (value !== allValues.password) {
            return t("usersAndRolesScreen.users.fields.passwordMismatch")
        }
    }

    return (
        <>
            <SelectInput
                label={String(t("reportsScreen.reportEncryptionPolicy.title"))}
                source={"encryption_policy"}
                defaultValue={EReportEncryption.UNENCRYPTED}
                choices={reportEncryptionPolicyChoices}
                disabled={report?.encryption_policy === EReportEncryption.CONFIGURED_PASSWORD}
                onChange={(event) => {
                    onChangePolicy(event.target.value)
                }}
                value={reportEncryptionPolicy ?? EReportEncryption.UNENCRYPTED}
                isRequired
            />

            <Dialog
                variant="info"
                open={isDialogOpen}
                handleClose={handleCloseDialog}
                okEnabled={checkIsValidPassword}
                aria-labelledby="password-dialog-title"
                title={String(t("electionEventScreen.export.passwordTitle"))}
                ok={String(t("usersAndRolesScreen.users.fields.savePassword"))}
            >
                <>
                    <InputContainerStyle>
                        <InputLabelStyle>
                            {t("usersAndRolesScreen.users.fields.password")}:
                        </InputLabelStyle>
                        <PasswordInputStyle
                            label={false}
                            source="password"
                            onChange={handleChangePassword}
                            value={filePassword.password}
                        />
                    </InputContainerStyle>
                    <InputContainerStyle>
                        <InputLabelStyle>
                            {t("usersAndRolesScreen.users.fields.repeatPassword")}:
                        </InputLabelStyle>
                        <PasswordInputStyle
                            label={false}
                            source="confirmPassword"
                            validate={equalToPassword}
                            onChange={handleChangeConfirmPassword}
                            value={filePassword.confirmPassword}
                        />
                    </InputContainerStyle>
                </>
            </Dialog>
        </>
    )
}

export const EditReportForm: React.FC<CreateReportProps> = ({
    close,
    tenantId,
    electionEventId,
    isEditReport,
    reportId,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const [reportEncryptionPolicy, setReportEncryptionPolicy] = useState<
        EReportEncryption | undefined
    >(EReportEncryption.UNENCRYPTED)
    const [createReport] = useMutation(CREATE_REPORT)
    const [updateReport] = useMutation(UPDATE_REPORT)
    const [encryptReport] = useMutation<EncryptReportMutation>(ENCRYPT_REPORT, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.REPORT_WRITE,
            },
        },
    })
    const [password, setPassword] = useState<string | null>(null)

    const [isCronActive, setIsCronActive] = useState<boolean>(false)
    const [cronValue, setCronValue] = useState<string>("00 8 * * 1,2,3,4,5")
    const [enabled, setEnabled] = useState<boolean>(false)
    const authContext = useContext(AuthContext)

    const {data: report} = useGetOne<Sequent_Backend_Report>(
        "sequent_backend_report",
        {id: reportId},
        {enabled: isEditReport}
    )

    useEffect(() => {
        if (undefined === reportEncryptionPolicy && report?.encryption_policy) {
            setReportEncryptionPolicy(report.encryption_policy as any)
        }
    }, [report?.encryption_policy, reportEncryptionPolicy])

    const handleSubmit = async (values: any) => {
        let formValues = {
            ...values,
        }
        if ("confirmPassword" in formValues) {
            delete formValues.confirmPassword
        }
        if ("password" in formValues) {
            delete formValues.password
        }

        const formData: Partial<Sequent_Backend_Report> = {
            ...formValues,
            encryption_policy: reportEncryptionPolicy,
            tenant_id: tenantId,
            election_event_id: electionEventId,
            cron_config: isCronActive
                ? {
                      is_active: true,
                      cron_expression: cronValue,
                      email_recipients: values.cron_config.email_recipients,
                      executer_username: authContext.username,
                  }
                : null,
        }
        let hasPassword = !!password
        try {
            if (isEditReport && reportId) {
                await updateReport({
                    variables: {
                        id: reportId,
                        set: formData,
                    },
                })
                if (hasPassword) {
                    await encryptReport({
                        variables: {
                            electionEventId: electionEventId,
                            reportId: reportId,
                            password: password,
                        },
                        onCompleted: async (data) => {
                            if (data.encrypt_report?.error_msg) {
                                notify(data.encrypt_report.error_msg, {type: "error"})
                            }
                        },
                        onError: (error) => {
                            console.log(error)
                            notify(t("reportsScreen.messages.createError"), {type: "error"})
                        },
                    })
                }
                notify(t(`reportsScreen.messages.updateSuccess`), {type: "success"})
            } else {
                const {data: reportData} = await createReport({
                    variables: {
                        object: formData,
                    },
                })
                notify(t(`reportsScreen.messages.createSuccess`), {type: "success"})
                if (reportData && hasPassword) {
                    await encryptReport({
                        variables: {
                            reportId: reportData.insert_sequent_backend_report.returning[0].id,
                            electionEventId: electionEventId,
                            password: password,
                        },
                        onCompleted: (data) => {
                            if (data.encrypt_report?.error_msg) {
                                notify(data.encrypt_report.error_msg, {type: "error"})
                            } else {
                                notify(t("reportsScreen.messages.encryptSuccess"), {
                                    type: "success",
                                })
                            }
                        },
                        onError: (error) => {
                            notify(t("reportsScreen.messages.encryptError"), {type: "error"})
                        },
                    })
                }
            }

            if (close) {
                close()
            }
        } catch (error) {
            notify(t(`reportsScreen.messages.submitError`), {type: "error"})
        }
    }

    return (
        <Create hasEdit={isEditReport}>
            <SimpleForm
                record={isEditReport ? report : undefined}
                onSubmit={handleSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton alwaysEnable={enabled} />
                    </Toolbar>
                }
            >
                <FormContent
                    tenantId={tenantId}
                    electionEventId={electionEventId}
                    isEditReport={isEditReport}
                    report={report}
                    doCronActive={(value) => setIsCronActive(value)}
                    cronValue={cronValue}
                    setCronValue={setCronValue}
                    setEnabled={setEnabled}
                />
                <PasswordComponent
                    report={report}
                    setPassword={setPassword}
                    reportEncryptionPolicy={reportEncryptionPolicy}
                    setReportEncryptionPolicy={setReportEncryptionPolicy}
                />
            </SimpleForm>
        </Create>
    )
}

interface EmailRecipientsInputProps extends InputProps {
    label?: string
    placeholder?: string
}

interface EmailRecipientsInputProps extends InputProps {
    label?: string
    placeholder?: string
}

const EmailRecipientsInput: React.FC<EmailRecipientsInputProps> = (props) => {
    const {
        field, // Contains value and onChange
        fieldState, // Contains error and touched
        isRequired,
        id,
    } = useInput(props)

    return (
        <Autocomplete
            multiple
            freeSolo
            options={[] as string[]}
            value={field.value || []}
            onChange={(event: any, newValue: string[]) => {
                field.onChange(newValue)
            }}
            fullWidth={true}
            renderTags={(value: string[], getTagProps) =>
                value.map((option: string, index: number) => (
                    <Chip label={option} {...getTagProps({index})} key={index} />
                ))
            }
            renderInput={(params) => (
                <TextField
                    {...params}
                    variant="outlined"
                    label={props.label}
                    placeholder={props.placeholder}
                    error={fieldState.invalid}
                    helperText={fieldState.error?.message}
                    required={isRequired}
                    id={id}
                />
            )}
        />
    )
}

// Add 'onChange' to the props interface
interface ReportTypeInputProps extends InputProps {
    label?: string
    choices: {id: any; name: string}[]
    onChange?: (newValue: any) => void // Add this line
}

interface PermissionLabelsInputProps extends InputProps {
    label?: string
    choices?: Choice[]
    setChoices?: React.Dispatch<React.SetStateAction<Choice[]>>
}

const PermissionLabelsInput: React.FC<PermissionLabelsInputProps> = (props) => {
    const {field, fieldState, isRequired, id} = useInput({
        ...props,
        source: "permission_label",
    })

    // Initialize local choices state if not provided in props
    const [internalChoices, setInternalChoices] = useState<Choice[]>(props.choices || [])
    const choices = props.choices || internalChoices

    // Update form value when choices change
    const handleChange = (newValue: string[]) => {
        field.onChange(newValue)
        // Also update choices if needed
        if (props.setChoices) {
            props.setChoices(newValue.map((label) => ({id: label, name: label})))
        }
    }

    // Handle creating new entries
    const handleCreate = (newValue: string) => {
        if (newValue && !choices.find((choice: Choice) => choice.id === newValue)) {
            const newChoice = {id: newValue, name: newValue}
            const updatedChoices = [...choices, newChoice]

            // Update local state if no external state management provided
            if (!props.setChoices) {
                setInternalChoices(updatedChoices)
            } else {
                props.setChoices(updatedChoices)
            }

            // Ensure the field value includes the new item
            const currentValue = field.value || []
            if (!currentValue.includes(newValue)) {
                field.onChange([...currentValue, newValue])
            }
        }
        return {id: newValue, name: newValue}
    }

    return (
        <CustomAutocompleteArrayInput
            label={props.label ?? ""}
            defaultValue={field.value || []}
            onChange={handleChange}
            onCreate={handleCreate}
            choices={choices}
        />
    )
}

const ReportTypeInput: React.FC<ReportTypeInputProps> = (props) => {
    const {field, fieldState, isRequired, id} = useInput(props)

    const {choices, label, onChange} = props // Destructure 'onChange'

    return (
        <Autocomplete
            fullWidth={true}
            options={choices}
            getOptionLabel={(option) => option.name}
            onChange={(event: any, newValue: {id: string; name: string} | null) => {
                field.onChange(newValue ? newValue.id : null)
                if (onChange) {
                    onChange(newValue ? newValue.id : null) // Call the passed onChange prop
                }
            }}
            value={choices.find((choice) => choice.id === field.value) || null}
            renderInput={(params) => (
                <TextField
                    {...params}
                    label={label ?? ""}
                    variant="outlined"
                    required={isRequired}
                    error={fieldState.invalid}
                    helperText={fieldState.error?.message}
                    id={id}
                />
            )}
        />
    )
}

const FormContent: React.FC<CreateReportProps> = ({
    tenantId,
    electionEventId,
    isEditReport,
    report,
    doCronActive,
    cronValue,
    setCronValue,
    setEnabled,
}) => {
    const {t} = useTranslation()

    const [reportType, setReportType] = useState<ETemplateType | undefined>(undefined)
    const [electionId, setElectionId] = useState<string | null | undefined>(undefined)
    const [templateAlias, setTemplateAlias] = useState<string | null | undefined>(undefined)
    const [isCronActive, setIsCronActive] = useState<boolean>(false)

    const {setValue, register} = useFormContext()

    useEffect(() => {
        register("cron_config.cron_expression")
    }, [register])

    useEffect(() => {
        console.log("report changed")
        setIsCronActive(report?.cron_config?.is_active || false)
        setCronValue?.(report?.cron_config?.cron_expression || "")
        setReportType(report?.report_type ? (report.report_type as ETemplateType) : undefined)
        setTemplateAlias(report?.template_alias || undefined)

        setValue("template_alias", report?.template_alias || undefined)
        setValue(
            "report_type",
            report?.report_type ? (report.report_type as ETemplateType) : undefined
        )
        setValue("cron_config.email_recipients", report?.cron_config?.email_recipients || [])
        setValue("permission_label", report?.permission_label || [])
    }, [report, setValue, setCronValue])

    useEffect(() => {
        doCronActive?.(isCronActive)
    }, [isCronActive, doCronActive])

    const reportTypeChoices = Object.values(EReportType)
        .sort()
        .map((reportType) => ({
            id: reportType,
            name: t(`template.type.${reportType}`),
        }))

    const electionPolicy = useMemo((): EReportElectionPolicy => {
        if (!reportType) {
            return EReportElectionPolicy.ELECTION_ALLOWED
        }
        return reportTypeConfig[reportType].electionPolicy ?? EReportElectionPolicy.ELECTION_ALLOWED
    }, [reportType])

    const isTemplateRequired = useMemo((): boolean => {
        if (!reportType) {
            return false
        }
        return reportTypeConfig[reportType].templateRequired ?? false
    }, [reportType])

    const canGenerateReportScheduled = useMemo((): boolean => {
        if (!reportType) {
            return false
        }
        return reportTypeConfig[reportType].actions.includes(ReportActions.GENERATE_SCHEDULED)
    }, [reportType])

    useEffect(() => {
        if (reportType) {
            setTemplateAlias(null)
            setValue("template_alias", null)
            setValue("report_type", reportType)
        }
    }, [reportType, setValue])

    useEffect(() => {
        if (report?.election_id && electionPolicy === EReportElectionPolicy.ELECTION_NOT_ALLOWED) {
            setElectionId(null)
            setValue("election_id", null)
        }
    }, [electionPolicy])

    useEffect(() => {
        // Reset the isCronActive state when the report type changes
        if (!canGenerateReportScheduled) {
            setIsCronActive(false)
        } else {
            setIsCronActive(report?.cron_config?.is_active ?? false)
        }
    }, [reportType, canGenerateReportScheduled, report])

    const handleCronToggle = (event: React.ChangeEvent<HTMLInputElement>) => {
        setIsCronActive(event.target.checked)
    }

    const handleReportTypeChange = (newValue: ETemplateType | null) => {
        setReportType(newValue || undefined)
        setTemplateAlias(null)
        setValue("template_alias", null)
        setValue("report_type", newValue)
    }

    const [permissionLabels, setPermissionLabels] = useState<string[]>([])

    const [permissionLabelChoices, setPermissionLabelChoices] = useState<Choice[]>([])

    useEffect(() => {
        if (report?.permission_label) {
            const choices = (report.permission_label as string[]).map((label) => ({
                id: label,
                name: label,
            }))
            setPermissionLabelChoices(choices)
        }
    }, [report?.permission_label])

    const handlePermissionLabelRemoved = (value: string[]) => {
        if (value?.length < permissionLabels?.length) {
            setValue("permission_label", value)
        }
    }

    const handlePermissionLabelChanged = (value: string[]) => {
        setValue("permission_label", value)
        setPermissionLabels(value)
    }

    const handlePermissionLabelAdded = (value: string[]) => {
        setValue("permission_label", value)
    }

    return (
        <>
            <Typography variant="h4">
                {isEditReport ? t("reportsScreen.edit.title") : t("reportsScreen.create.title")}
            </Typography>
            <Typography variant="body2">
                {isEditReport
                    ? t("reportsScreen.edit.subtitle")
                    : t("reportsScreen.create.subtitle")}
            </Typography>
            <ReportTypeInput
                source="report_type"
                label={String(t("template.form.type"))}
                choices={reportTypeChoices}
                isRequired={true}
                onChange={handleReportTypeChange}
            />
            <SelectElection
                tenantId={tenantId}
                electionEventId={electionEventId}
                label={String(t("reportsScreen.fields.electionId"))}
                onSelectElection={(electionId) => setElectionId(electionId)}
                source="election_id"
                value={electionId}
                isRequired={electionPolicy === EReportElectionPolicy.ELECTION_REQUIRED}
                disabled={electionPolicy === EReportElectionPolicy.ELECTION_NOT_ALLOWED}
            />
            <SelectTemplate
                tenantId={tenantId}
                templateType={
                    reportType ? reportTypeConfig[reportType]?.associatedTemplateType : undefined
                }
                source={"template_alias"}
                label={String(t("reportsScreen.fields.template"))}
                onSelectTemplate={(template) => {
                    console.log("Selected templateId:", template.alias)
                    setTemplateAlias(template.alias)
                }}
                value={templateAlias}
                isRequired={isTemplateRequired}
            />

            <PermissionLabelsInput
                label={String(t("usersAndRolesScreen.users.fields.permissionLabel"))}
                choices={permissionLabelChoices}
                setChoices={setPermissionLabelChoices}
                source="permission_label"
            />

            {canGenerateReportScheduled && (
                <BooleanInput
                    source="cron_config.is_active"
                    label={String(t("reportsScreen.fields.repeatable"))}
                    onChange={handleCronToggle}
                />
            )}
            {isCronActive && (
                <>
                    <Cron
                        value={cronValue ?? ""}
                        setValue={(newValue: string) => {
                            console.log(`new cron config: ${newValue}`)
                            setValue("cron_config.cron_expression", newValue, {
                                shouldDirty: true,
                                shouldTouch: true,
                            })
                            setCronValue?.(newValue)
                            if (newValue !== cronValue) {
                                setEnabled?.(true)
                            }
                        }}
                    />
                    <EmailRecipientsInput
                        source="cron_config.email_recipients"
                        label={String(t("reportsScreen.fields.emailRecipients"))}
                        placeholder={t("reportsScreen.fields.emailRecipientsPlaceholder")}
                        isRequired={false}
                    />
                </>
            )}
        </>
    )
}
