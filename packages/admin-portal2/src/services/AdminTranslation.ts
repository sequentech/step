// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import polyglotI18nProvider from "ra-i18n-polyglot"
import englishMessages from "ra-language-english"

const spanishMessages = {
    ...englishMessages,
    ra: {
        ...englishMessages.ra,
        action: {
            ...englishMessages.ra.action,
            add_filter: "Añadir filtro",
            select_columns: "Columnas",
            save: "Guardar",
            confirm: "Confirmar",
            create: "Crear",
            create_item: "Crear %{item}",
            delete: "Borrar",
            edit: "Editar",
            export: "Exportar",
            list: "Lista",
            refresh: "Refrescar",
        },
    },
}

export const adminI18nProvider = polyglotI18nProvider(
    (locale) => {
        return locale === "es" ? spanishMessages : englishMessages
    },
    "en" // Default locale
)
