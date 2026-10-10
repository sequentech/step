// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useTranslation} from "react-i18next"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Box,
    Button,
    Checkbox,
    FormControlLabel,
    FormGroup,
    MenuItem,
    TextField,
    Typography,
} from "@mui/material"
import AddIcon from "@mui/icons-material/Add"
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {JsonEditor} from "json-edit-react"
import {EMessagePurpose, EPhoneFormat, MESSAGE_PURPOSES} from "@/types/messaging"
import {
    EHttpConfigProblem,
    EHttpSection,
    HTTP_API_EXAMPLE,
    HTTP_PLACEHOLDERS,
    HTTP_SECTIONS,
    HTTP_SECTION_SKELETONS,
    IHttpApiForm,
    IHttpConfigProblem,
    httpFormProblems,
    withHttpExample,
} from "./httpApiSender"

interface IHttpProblemsProps {
    problems: IHttpConfigProblem[] | undefined
    /** Whether a request that was not written yet is already a problem to show. */
    submitted: boolean
}

/** What is wrong with a part of the configuration, one line per problem. */
export const HttpProblems: React.FC<IHttpProblemsProps> = ({problems, submitted}) => {
    const {t} = useTranslation()
    const shown = (problems ?? []).filter(
        ({problem}) => submitted || problem !== EHttpConfigProblem.MISSING_URL
    )
    if (!shown.length) {
        return null
    }
    return (
        <Box sx={{display: "flex", flexDirection: "column"}}>
            {shown.map(({problem, path}) => (
                <Typography
                    key={`${problem}-${path}`}
                    variant="caption"
                    role="alert"
                    sx={{color: "error.main"}}
                >
                    {t(`messagingAccounts.http.problem.${problem}`, {
                        path: path || t("messagingAccounts.http.thisSection"),
                    })}
                </Typography>
            ))}
        </Box>
    )
}

interface IHttpJsonSectionProps {
    /** `SEND`, or one of the optional sections. */
    part: EHttpSection | "SEND"
    value: unknown
    problems: IHttpConfigProblem[] | undefined
    submitted: boolean
    disabled: boolean
    onChange: (value: unknown) => void
}

/** One part of the provider's description, edited as JSON, with the problems of its shape. */
export const HttpJsonSection: React.FC<IHttpJsonSectionProps> = ({
    part,
    value,
    problems,
    submitted,
    disabled,
    onChange,
}) => {
    const {t} = useTranslation()
    const title = t(`messagingAccounts.http.section.${part}`)
    const skeleton = part === "SEND" ? null : HTTP_SECTION_SKELETONS[part]
    const optional = skeleton !== null
    const configured = value !== null && value !== undefined
    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 1}} data-http-section={part}>
            <Box
                sx={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    gap: 2,
                }}
            >
                <Typography variant="body2" component="h4" sx={{fontWeight: 500}}>
                    {title}
                </Typography>
                {optional && !disabled && configured && (
                    <Button
                        size="small"
                        startIcon={<DeleteOutlineIcon />}
                        onClick={() => onChange(null)}
                    >
                        {t("messagingAccounts.http.remove", {section: title})}
                    </Button>
                )}
                {optional && !disabled && !configured && (
                    <Button
                        size="small"
                        startIcon={<AddIcon />}
                        onClick={() => onChange(JSON.parse(JSON.stringify(skeleton)))}
                    >
                        {t("messagingAccounts.http.add", {section: title})}
                    </Button>
                )}
            </Box>
            <Typography variant="caption" color="text.secondary">
                {t(`messagingAccounts.http.sectionHelp.${part}`)}
            </Typography>
            {configured ? (
                <JsonEditor
                    data={value as object}
                    setData={onChange}
                    rootName={part.toLowerCase()}
                    viewOnly={disabled}
                    collapse={3}
                    maxWidth="100%"
                />
            ) : (
                <Typography variant="caption" color="text.secondary">
                    {t("messagingAccounts.http.notConfigured")}
                </Typography>
            )}
            <HttpProblems problems={problems} submitted={submitted} />
        </Box>
    )
}

/** The placeholders a request may hold, and what each becomes. */
export const HttpPlaceholderReference: React.FC = () => {
    const {t} = useTranslation()
    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 1}}>
            <Typography variant="caption" color="text.secondary">
                {t("messagingAccounts.http.placeholders.help")}
            </Typography>
            <Box component="dl" sx={{margin: 0, display: "grid", rowGap: 0.5}}>
                {HTTP_PLACEHOLDERS.map(({placeholder, id}) => (
                    <Box
                        key={id}
                        sx={{display: "grid", gridTemplateColumns: "190px 1fr", columnGap: 1}}
                    >
                        <Typography
                            component="dt"
                            variant="caption"
                            sx={{fontFamily: "monospace", overflowWrap: "anywhere"}}
                        >
                            {placeholder}
                        </Typography>
                        <Typography component="dd" variant="caption" sx={{margin: 0}}>
                            {t(`messagingAccounts.http.placeholder.${id}`)}
                        </Typography>
                    </Box>
                ))}
            </Box>
        </Box>
    )
}

export interface IHttpApiSenderEditorProps {
    values: IHttpApiForm
    /** Whether saving was tried, so a request not written yet is shown as a problem. */
    submitted: boolean
    disabled: boolean
    onChange: (values: IHttpApiForm) => void
}

/** A provider described by configuration: what it can do and the requests that reach it. */
export const HttpApiSenderEditor: React.FC<IHttpApiSenderEditorProps> = ({
    values,
    submitted,
    disabled,
    onChange,
}) => {
    const {t} = useTranslation()
    const problems = httpFormProblems(values)
    const pointerProblem = problems.MESSAGE_ID_POINTER?.[0]
    const hoursProblem = problems.CONVERSATION_WINDOW?.[0]
    const problemText = (problem: IHttpConfigProblem | undefined, field: string) =>
        problem ? t(`messagingAccounts.http.problem.${problem.problem}`, {path: field}) : undefined
    const hoursLabel = t("messagingAccounts.http.conversationWindow")
    const pointerLabel = t("messagingAccounts.http.messageIdPointer")
    const togglePurpose = (purpose: EMessagePurpose, required: boolean) =>
        onChange({
            ...values,
            templateRequiredFor: required
                ? [...values.templateRequiredFor, purpose]
                : values.templateRequiredFor.filter((entry) => entry !== purpose),
        })

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            <Box>
                <Typography variant="subtitle2" component="h3">
                    {t("messagingAccounts.http.title")}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                    {t("messagingAccounts.http.description")}
                </Typography>
            </Box>

            <TextField
                select
                label={t("messagingAccounts.http.phoneFormat")}
                value={values.phoneFormat}
                disabled={disabled}
                helperText={t("messagingAccounts.http.phoneFormatHelp")}
                onChange={(event) =>
                    onChange({...values, phoneFormat: event.target.value as EPhoneFormat})
                }
                fullWidth
            >
                {Object.values(EPhoneFormat).map((format) => (
                    <MenuItem key={format} value={format}>
                        {t(`messagingAccounts.http.phoneFormatOption.${format}`)}
                    </MenuItem>
                ))}
            </TextField>

            <Box>
                <Typography variant="body2" sx={{fontWeight: 500}} id="http-template-required">
                    {t("messagingAccounts.http.templateRequired")}
                </Typography>
                <FormGroup row aria-labelledby="http-template-required">
                    {MESSAGE_PURPOSES.map((purpose) => (
                        <FormControlLabel
                            key={purpose}
                            label={t(`messaging.purpose.${purpose}`)}
                            control={
                                <Checkbox
                                    checked={values.templateRequiredFor.includes(purpose)}
                                    disabled={disabled}
                                    onChange={(event) =>
                                        togglePurpose(purpose, event.target.checked)
                                    }
                                />
                            }
                        />
                    ))}
                </FormGroup>
                <Typography variant="caption" color="text.secondary">
                    {t("messagingAccounts.http.templateRequiredHelp")}
                </Typography>
            </Box>

            {MESSAGE_PURPOSES.map((purpose) => (
                <TextField
                    key={purpose}
                    label={t("messagingAccounts.http.approvedLanguages", {
                        purpose: t(`messaging.purpose.${purpose}`),
                    })}
                    value={values.approvedLanguages[purpose]}
                    disabled={disabled}
                    helperText={t("messagingAccounts.http.approvedLanguagesHelp")}
                    onChange={(event) =>
                        onChange({
                            ...values,
                            approvedLanguages: {
                                ...values.approvedLanguages,
                                [purpose]: event.target.value,
                            },
                        })
                    }
                    fullWidth
                />
            ))}

            <TextField
                label={hoursLabel}
                value={values.conversationWindowHours}
                disabled={disabled}
                error={!!hoursProblem}
                helperText={
                    problemText(hoursProblem, hoursLabel) ??
                    t("messagingAccounts.http.conversationWindowHelp")
                }
                onChange={(event) =>
                    onChange({...values, conversationWindowHours: event.target.value})
                }
                slotProps={{htmlInput: {inputMode: "numeric"}}}
                fullWidth
            />

            <HttpJsonSection
                part="SEND"
                value={values.send}
                problems={problems.SEND}
                submitted={submitted}
                disabled={disabled}
                onChange={(send) => onChange({...values, send})}
            />

            <TextField
                label={pointerLabel}
                value={values.messageIdPointer}
                disabled={disabled}
                error={!!pointerProblem}
                helperText={
                    problemText(pointerProblem, pointerLabel) ??
                    t("messagingAccounts.http.messageIdPointerHelp")
                }
                onChange={(event) => onChange({...values, messageIdPointer: event.target.value})}
                fullWidth
            />

            {HTTP_SECTIONS.map((section) => (
                <HttpJsonSection
                    key={section}
                    part={section}
                    value={values.sections[section]}
                    problems={problems[section]}
                    submitted={submitted}
                    disabled={disabled}
                    onChange={(value) =>
                        onChange({...values, sections: {...values.sections, [section]: value}})
                    }
                />
            ))}

            <Accordion disableGutters>
                <AccordionSummary
                    expandIcon={<ExpandMoreIcon />}
                    id="http-placeholders-header"
                    aria-controls="http-placeholders-content"
                >
                    <Typography variant="body2" sx={{fontWeight: 500}}>
                        {t("messagingAccounts.http.placeholders.title")}
                    </Typography>
                </AccordionSummary>
                <AccordionDetails id="http-placeholders-content">
                    <HttpPlaceholderReference />
                </AccordionDetails>
            </Accordion>

            <Accordion disableGutters>
                <AccordionSummary
                    expandIcon={<ExpandMoreIcon />}
                    id="http-example-header"
                    aria-controls="http-example-content"
                >
                    <Typography variant="body2" sx={{fontWeight: 500}}>
                        {t("messagingAccounts.http.example.title")}
                    </Typography>
                </AccordionSummary>
                <AccordionDetails
                    id="http-example-content"
                    sx={{display: "flex", flexDirection: "column", gap: 1}}
                >
                    <Typography variant="caption" color="text.secondary">
                        {t("messagingAccounts.http.example.description")}
                    </Typography>
                    <Box
                        component="pre"
                        tabIndex={0}
                        aria-label={t("messagingAccounts.http.example.title")}
                        sx={{
                            margin: 0,
                            padding: 1,
                            fontSize: 12,
                            overflowX: "auto",
                            border: 1,
                            borderColor: "divider",
                            borderRadius: 1,
                        }}
                    >
                        {JSON.stringify(HTTP_API_EXAMPLE, null, 2)}
                    </Box>
                    {!disabled && (
                        <Box>
                            <Button
                                variant="outlined"
                                onClick={() => onChange(withHttpExample(values))}
                            >
                                {t("messagingAccounts.http.example.use")}
                            </Button>
                        </Box>
                    )}
                </AccordionDetails>
            </Accordion>
        </Box>
    )
}
