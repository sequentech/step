// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useMemo, useState} from "react"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import {ArrowIcon, UserIcon} from "../icons"
import {CredentialFieldPosition, RegistrationFormMode, type SequentRegistration} from "../KcContext"
import CredentialFields from "../enrollment/CredentialFields"
import type {EnrollmentPageProps} from "../enrollment/pageProps"
import ProfileFields, {useProfileForm} from "../enrollment/ProfileFields"
import {credentialPlacement, visibleAttributes} from "../enrollment/profile"
import {EnrollmentStep, enrollmentFrame} from "../scanovate/text"

const KEYCLOAK_REGISTRATION: SequentRegistration = {
    credentialFieldPosition: CredentialFieldPosition.Last,
    hiddenAttributes: [],
    lockedAttributes: [],
}

// The form voters enroll with: the realm's User Profile and, when the flow
// asks for one, a password. The form that signs voters in (LOGIN mode), CAPTCHA
// and terms acceptance keep sequent-theme's register.ftl.
export default function Register(props: EnrollmentPageProps<"register.ftl">) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {url, realm, profile, passwordRequired, messagesPerField} = kcContext
    const registration = kcContext.sequentRegistration ?? KEYCLOAK_REGISTRATION
    const [submitting, setSubmitting] = useState(false)
    const attributes = useMemo(
        () =>
            visibleAttributes(
                Object.values(profile.attributesByName),
                registration.hiddenAttributes
            ),
        [profile.attributesByName, registration.hiddenAttributes]
    )
    const form = useProfileForm({
        kcContext,
        i18n,
        attributes,
        locked: registration.lockedAttributes,
    })
    const placement = credentialPlacement({
        attributes,
        emailAsUsername: realm.registrationEmailAsUsername,
        passwordRequired,
        position: registration.credentialFieldPosition,
    })
    // Keycloak's own registration form asks for the password once and is no step of an enrollment.
    const enrolling = registration.formMode === RegistrationFormMode.Registration
    const credentials = passwordRequired && (
        <CredentialFields
            kcContext={kcContext}
            i18n={i18n}
            source={placement.source}
            creating={enrolling}
            autoFocus={placement.first}
        />
    )

    return (
        <Template
            kcContext={kcContext}
            i18n={i18n}
            doUseDefaultCss={doUseDefaultCss}
            classes={classes}
            {...(enrolling && kcContext.themeName === "sequent-ui-voting"
                ? enrollmentFrame(kcContext, i18n, EnrollmentStep.Details)
                : {})}
            symbol={<UserIcon />}
            headerNode={i18n.msgStr("registerTitle")}
            titleLang={i18n.currentLanguage.languageTag}
            displayMessage={messagesPerField.exists("global")}
            displayInfo={enrolling}
            infoNode={<a href={url.loginUrl}>{i18n.msg("backToLogin")}</a>}
        >
            <Box
                component="form"
                id="kc-register-form"
                className="auth-form"
                action={url.registrationAction}
                method="post"
                onSubmit={() => setSubmitting(true)}
            >
                <p className="auth-required-note">
                    <span aria-hidden="true">* </span>
                    {i18n.msgStr("requiredFields")}
                </p>
                {placement.first && credentials}
                <ProfileFields
                    form={form}
                    afterField={(attribute) =>
                        !placement.first && attribute.name === placement.anchor && credentials
                    }
                />
                <div id="kc-form-buttons" className="auth-actions">
                    <Button
                        type="submit"
                        variant="contained"
                        fullWidth
                        className="auth-submit"
                        endIcon={<ArrowIcon />}
                        disabled={submitting}
                    >
                        {i18n.msgStr("doRegister")}
                    </Button>
                </div>
            </Box>
        </Template>
    )
}
