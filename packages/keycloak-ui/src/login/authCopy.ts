// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// New layout copy is separate from server-owned authentication messages. Keep
// the fallback language explicit for locales outside the Sequent theme's set.
type AuthCopy = {
    adminPortal: string
    votingPortal: string
    adminEyebrow: string
    votingEyebrow: string
    poweredBy: string
}

const translations: Record<string, AuthCopy> = {
    en: {
        adminPortal: "Admin Portal",
        votingPortal: "Voting Portal",
        adminEyebrow: "Your election workspace",
        votingEyebrow: "Your voting portal",
        poweredBy: "Powered by Sequent Tech Inc",
    },
    es: {
        adminPortal: "Portal de Administración",
        votingPortal: "Portal de Votación",
        adminEyebrow: "Su espacio electoral",
        votingEyebrow: "Su portal de votación",
        poweredBy: "Desarrollado por Sequent Tech Inc",
    },
    ca: {
        adminPortal: "Portal d'administració",
        votingPortal: "Portal de votació",
        adminEyebrow: "El vostre espai electoral",
        votingEyebrow: "El vostre portal de votació",
        poweredBy: "Desenvolupat per Sequent Tech Inc",
    },
    eu: {
        adminPortal: "Administrazio-ataria",
        votingPortal: "Bozketa-ataria",
        adminEyebrow: "Zure hauteskunde-eremua",
        votingEyebrow: "Zure bozketa-ataria",
        poweredBy: "Sequent Tech Inc-ek garatua",
    },
    gl: {
        adminPortal: "Portal de Administración",
        votingPortal: "Portal de Votación",
        adminEyebrow: "O seu espazo electoral",
        votingEyebrow: "O seu portal de votación",
        poweredBy: "Desenvolvido por Sequent Tech Inc",
    },
    tl: {
        adminPortal: "Portal ng Administrasyon",
        votingPortal: "Portal ng Pagboto",
        adminEyebrow: "Ang iyong lugar para sa halalan",
        votingEyebrow: "Ang iyong portal sa pagboto",
        poweredBy: "Hatid ng Sequent Tech Inc",
    },
    fr: {
        adminPortal: "Portail d’administration",
        votingPortal: "Portail de vote",
        adminEyebrow: "Votre espace électoral",
        votingEyebrow: "Votre portail de vote",
        poweredBy: "Propulsé par Sequent Tech Inc",
    },
    nl: {
        adminPortal: "Beheerportaal",
        votingPortal: "Stemportaal",
        adminEyebrow: "Uw verkiezingsomgeving",
        votingEyebrow: "Uw stemportaal",
        poweredBy: "Mogelijk gemaakt door Sequent Tech Inc",
    },
}

export function getAuthCopy(languageTag: string): AuthCopy & {languageTag: string} {
    const language = languageTag.split("-")[0].toLowerCase()
    const translated = Object.prototype.hasOwnProperty.call(translations, language)
    return {
        ...translations[translated ? language : "en"],
        languageTag: translated ? languageTag : "en",
    }
}
