// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {Fragment, useCallback, useEffect, useMemo, useState, type ReactNode} from "react"
import {kcSanitize} from "keycloakify/lib/kcSanitize"
import type {Attribute} from "keycloakify/login/KcContext"
import Checkbox from "@mui/material/Checkbox"
import FormControlLabel from "@mui/material/FormControlLabel"
import Radio from "@mui/material/Radio"
import TextField from "@mui/material/TextField"
import type {I18n} from "../i18n"
import type {KcContext} from "../KcContext"
import {ScriptKind, loadScript} from "./assets"
import PhoneField from "./PhoneField"
import {
    annotation,
    fieldToggles,
    filteredOptions,
    htmlAttributes,
    initialValues,
    inputTypeOf,
    optionsOf,
    type FieldToggle,
} from "./profile"

type Register = Extract<KcContext, {pageId: "register.ftl"}>

enum InputType {
    Textarea = "textarea",
    Select = "select",
    Multiselect = "multiselect",
    Radios = "select-radiobuttons",
    Checkboxes = "multiselect-checkboxes",
    Phone = "tel",
    Date = "date",
}

const MAX_DATE = "9999-12-31"

export type ProfileForm = {
    kcContext: Register
    i18n: I18n
    attributes: Attribute[]
    // Prefilled from a read-only login hint: posted, but not editable.
    locked: string[]
    values: Record<string, string[]>
    toggles: Record<string, FieldToggle>
    // The text each filtered select's options have to contain, by its name.
    filters: Record<string, string>
    setValues: (name: string, values: string[]) => void
    setFilter: (name: string, filter: string) => void
}

// A locked field is sent empty.
function withoutLocked(
    attributes: Attribute[],
    values: Record<string, string[]>
): Record<string, string[]> {
    const toggles = fieldToggles(attributes, values)
    return Object.fromEntries(
        Object.entries(values).map(([name, value]) => [
            name,
            toggles[name]?.readOnly === true ? [] : value,
        ])
    )
}

export function useProfileForm(params: {
    kcContext: Register
    i18n: I18n
    attributes: Attribute[]
    locked: string[]
}): ProfileForm {
    const {kcContext, i18n, attributes, locked} = params
    const [values, setAllValues] = useState(() =>
        withoutLocked(
            attributes,
            Object.fromEntries(
                attributes.flatMap((attribute) => {
                    const initial = initialValues(attribute)
                    return annotation(attribute, "confirm") === undefined
                        ? [[attribute.name, initial]]
                        : [
                              [attribute.name, initial],
                              [`${attribute.name}-confirm`, initial],
                          ]
                })
            )
        )
    )
    const [filters, setFilters] = useState<Record<string, string>>({})
    const toggles = useMemo(() => fieldToggles(attributes, values), [attributes, values])
    const setValues = useCallback(
        (name: string, next: string[]) =>
            setAllValues((current) => withoutLocked(attributes, {...current, [name]: next})),
        [attributes]
    )
    const setFilter = useCallback(
        (name: string, filter: string) => {
            setFilters((current) => ({...current, [name]: filter}))
            const target = attributes.find((attribute) => attribute.name === name)
            if (target === undefined) return
            const [first] = filteredOptions(optionsOf(target), filter)
            setValues(name, first === undefined ? [] : [first])
        },
        [attributes, setValues]
    )
    return {kcContext, i18n, attributes, locked, values, toggles, filters, setValues, setFilter}
}

function optionLabel(attribute: Attribute, option: string, i18n: I18n): string {
    const labels = attribute.annotations.inputOptionLabels
    if (labels !== undefined) return i18n.advancedMsgStr(labels[option] ?? option)
    const prefix = attribute.annotations.inputOptionLabelsI18nPrefix
    return prefix === undefined ? option : i18n.advancedMsgStr(`${prefix}.${option}`)
}

export enum HelperPlacement {
    Before = "BEFORE",
    After = "AFTER",
}

export function HelperText(props: {
    name: string
    placement: HelperPlacement
    text?: string
    i18n: I18n
}) {
    const {name, placement, text, i18n} = props
    if (text === undefined || text === "") return null
    const after = placement === HelperPlacement.After
    return (
        <div
            id={helperId(name, placement)}
            className={after ? "auth-helper after" : "auth-helper"}
            aria-live="polite"
        >
            {i18n.advancedMsg(text)}
        </div>
    )
}

export function helperId(name: string, placement: HelperPlacement): string {
    return `form-help-text-${placement === HelperPlacement.After ? "after" : "before"}-${name}`
}

export function FieldError({id, html}: {id: string; html: string}) {
    return (
        <span
            id={id}
            className="auth-field-error"
            aria-live="polite"
            dangerouslySetInnerHTML={{__html: kcSanitize(html)}}
        />
    )
}

export function RequiredMark() {
    return (
        <span className="auth-required" aria-hidden="true">
            {" *"}
        </span>
    )
}

function numberOf(value: string | number | undefined): number | undefined {
    return value === undefined ? undefined : Number(value)
}

type InputProps = {
    attribute: Attribute
    name: string
    form: ProfileForm
    required: boolean
    invalid: boolean
    describedBy?: string
}

function ChoiceInputs({attribute, name, form, required, invalid, describedBy}: InputProps) {
    const {i18n, values, setValues} = form
    const radio = inputTypeOf(attribute) === InputType.Radios
    const selected = values[name] ?? []
    const locked = form.locked.includes(attribute.name)
    const Control = radio ? Radio : Checkbox
    return (
        <div
            role={radio ? "radiogroup" : "group"}
            className="auth-choices"
            aria-labelledby={`${name}-label`}
            aria-describedby={describedBy}
        >
            {optionsOf(attribute).map((option, index) => (
                <FormControlLabel
                    key={option}
                    label={optionLabel(attribute, option, i18n)}
                    control={
                        <Control
                            id={`${name}-${option}`}
                            name={name}
                            value={option}
                            checked={selected.includes(option)}
                            disabled={attribute.readOnly || locked}
                            slotProps={{
                                input: {
                                    // On the input: the option's label takes no mark of its
                                    // own. Any checked option satisfies a required group.
                                    required:
                                        required &&
                                        (radio || (index === 0 && selected.length === 0)),
                                    "aria-invalid": invalid || undefined,
                                    "aria-required": !radio && required ? true : undefined,
                                },
                            }}
                            onChange={(event) =>
                                setValues(
                                    name,
                                    radio
                                        ? [option]
                                        : event.target.checked
                                          ? [...selected, option]
                                          : selected.filter((value) => value !== option)
                                )
                            }
                        />
                    }
                />
            ))}
            {locked &&
                selected.map((value) => (
                    <input key={value} type="hidden" name={name} value={value} />
                ))}
        </div>
    )
}

function SelectInput({attribute, name, form, required, invalid, describedBy}: InputProps) {
    const {i18n, values, setValues, setFilter} = form
    const multiple = inputTypeOf(attribute) === InputType.Multiselect
    const selected = values[name] ?? []
    const locked = form.locked.includes(attribute.name)
    const filters = annotation(attribute, "filterSelectAttribute")
    return (
        <>
            <select
                id={name}
                name={name}
                className="auth-input"
                multiple={multiple}
                value={multiple ? selected : (selected[0] ?? "")}
                required={required}
                disabled={attribute.readOnly || locked || form.toggles[name]?.disabled === true}
                size={numberOf(attribute.annotations.inputTypeSize)}
                aria-invalid={invalid || undefined}
                aria-describedby={describedBy}
                onChange={(event) => {
                    const next = [...event.target.selectedOptions].map(({value}) => value)
                    setValues(name, next)
                    if (filters !== undefined) setFilter(filters, event.target.value)
                }}
            >
                {!multiple && <option value="" />}
                {filteredOptions(optionsOf(attribute), form.filters[name]).map((option) => (
                    <option key={option} value={option}>
                        {optionLabel(attribute, option, i18n)}
                    </option>
                ))}
            </select>
            {/* A disabled select posts nothing. */}
            {locked &&
                selected.map((value) => (
                    <input key={value} type="hidden" name={name} value={value} />
                ))}
        </>
    )
}

function TextInput(props: InputProps & {index: number; value: string}) {
    const {attribute, name, form, required, invalid, describedBy, index, value} = props
    const {annotations} = attribute
    const type = inputTypeOf(attribute)
    const id = index === 0 ? name : `${name}-${index}`
    const toggle = form.toggles[name] ?? {}
    const readOnly = form.locked.includes(attribute.name) || toggle.readOnly === true
    const disabled = attribute.readOnly || toggle.disabled === true
    const extra = useMemo(() => htmlAttributes(attribute), [attribute])
    // The realm's html-attribute annotations, style included, as it wrote them.
    const applyExtra = useCallback(
        (input: HTMLElement | null) => {
            if (input === null) return
            for (const [key, text] of Object.entries(extra)) input.setAttribute(key, text)
        },
        [extra]
    )
    const data = Object.fromEntries(
        Object.entries(attribute.html5DataAnnotations ?? {}).map(([key, text]) => [
            `data-${key}`,
            text,
        ])
    )
    if (type === InputType.Phone) {
        return (
            <PhoneField
                id={id}
                resources={form.kcContext.url.resourcesPath}
                defaultValue={value}
                pattern={annotations.inputTypePattern}
                placeholder={annotations.inputTypePlaceholder}
                required={required}
                readOnly={readOnly}
                disabled={disabled}
                invalid={invalid}
                describedBy={describedBy}
                inputRef={applyExtra}
            />
        )
    }
    const textarea = type === InputType.Textarea
    return (
        <TextField
            id={id}
            name={name}
            type={textarea ? undefined : type}
            multiline={textarea}
            rows={textarea ? (numberOf(annotations.inputTypeRows) ?? 4) : undefined}
            value={value}
            required={required}
            disabled={disabled}
            error={invalid}
            fullWidth
            inputRef={applyExtra}
            onChange={(event) => {
                const next = [...(form.values[name] ?? [])]
                next[index] = event.target.value
                form.setValues(name, next)
            }}
            slotProps={{
                htmlInput: {
                    readOnly,
                    "aria-describedby": describedBy,
                    placeholder: annotations.inputTypePlaceholder,
                    maxLength: numberOf(annotations.inputTypeMaxlength),
                    ...(textarea
                        ? {cols: numberOf(annotations.inputTypeCols)}
                        : {
                              pattern: annotations.inputTypePattern,
                              size: numberOf(annotations.inputTypeSize),
                              minLength: numberOf(annotations.inputTypeMinlength),
                              // A bare date input takes years of more than four digits.
                              max:
                                  annotations.inputTypeMax ??
                                  (type === InputType.Date ? MAX_DATE : undefined),
                              min: annotations.inputTypeMin,
                              step: annotations.inputTypeStep,
                          }),
                    ...data,
                },
            }}
        />
    )
}

function ProfileField({
    attribute,
    name,
    form,
}: {
    attribute: Attribute
    name: string
    form: ProfileForm
}) {
    const {kcContext, i18n} = form
    const {messagesPerField} = kcContext
    const {annotations} = attribute
    const type = inputTypeOf(attribute)
    const choices = type === InputType.Radios || type === InputType.Checkboxes
    const label = i18n.advancedMsgStr(
        name === attribute.name
            ? (attribute.displayName ?? "")
            : (annotation(attribute, "confirm") ?? "")
    )
    const invalid = messagesPerField.existsError(name)
    const before = annotations.inputHelperTextBefore ?? ""
    const after = annotations.inputHelperTextAfter ?? ""
    const describedBy =
        [
            before !== "" && helperId(name, HelperPlacement.Before),
            invalid && `input-error-${name}`,
            after !== "" && helperId(name, HelperPlacement.After),
        ]
            .filter((id) => id !== false)
            .join(" ") || undefined
    const required = form.toggles[name]?.required ?? attribute.required
    const input = {attribute, name, form, required, invalid, describedBy}
    const values = form.values[name] ?? []
    return (
        <div>
            {choices ? (
                <span id={`${name}-label`} className="auth-field-label">
                    {label}
                    {required && <RequiredMark />}
                </span>
            ) : (
                <label htmlFor={name} className="auth-field-label">
                    {label}
                    {required && <RequiredMark />}
                </label>
            )}
            <HelperText name={name} placement={HelperPlacement.Before} text={before} i18n={i18n} />
            {choices ? (
                <ChoiceInputs {...input} />
            ) : type === InputType.Select || type === InputType.Multiselect ? (
                <SelectInput {...input} />
            ) : attribute.multivalued === true && values.length > 0 ? (
                values.map((value, index) => (
                    <TextInput key={index} {...input} index={index} value={value} />
                ))
            ) : (
                <TextInput {...input} index={0} value={values[0] ?? ""} />
            )}
            {invalid && <FieldError id={`input-error-${name}`} html={messagesPerField.get(name)} />}
            <HelperText name={name} placement={HelperPlacement.After} text={after} i18n={i18n} />
        </div>
    )
}

function GroupHeader({attribute, i18n}: {attribute: Attribute; i18n: I18n}) {
    const {group} = attribute
    if (group === undefined || group.name === "") return null
    const header = group.displayHeader ?? ""
    const description = group.displayDescription ?? ""
    return (
        <div className="auth-group">
            <h2 id={`header-${group.name}`} className="auth-group-title">
                {header === "" ? group.name : i18n.advancedMsgStr(header)}
            </h2>
            {description !== "" && (
                <p id={`description-${group.name}`}>{i18n.advancedMsgStr(description)}</p>
            )}
        </div>
    )
}

// The realm's User Profile attributes, with sequent-theme's annotations.
export default function ProfileFields(props: {
    form: ProfileForm
    afterField: (attribute: Attribute) => ReactNode
}) {
    const {form, afterField} = props
    const {kcContext, i18n, attributes} = form
    const resources = kcContext.url.resourcesPath
    const scripts = Object.keys(kcContext.profile.html5DataAnnotations ?? {}).join(" ")
    // Keycloak's scripts for the data annotations look the fields up when they run.
    useEffect(() => {
        for (const script of scripts.split(" ").filter((name) => name !== "")) {
            void loadScript(`${resources}/js/${script}.js`, ScriptKind.Module).catch(
                () => undefined
            )
        }
    }, [resources, scripts])
    return (
        <>
            {attributes.map((attribute, index) => (
                <Fragment key={attribute.name}>
                    {attribute.group?.name !== attributes[index - 1]?.group?.name && (
                        <GroupHeader attribute={attribute} i18n={i18n} />
                    )}
                    {annotation(attribute, "hidden") !== "true" && (
                        <>
                            <ProfileField attribute={attribute} name={attribute.name} form={form} />
                            {annotation(attribute, "confirm") !== undefined && (
                                <ProfileField
                                    attribute={attribute}
                                    name={`${attribute.name}-confirm`}
                                    form={form}
                                />
                            )}
                        </>
                    )}
                    {afterField(attribute)}
                </Fragment>
            ))}
        </>
    )
}
