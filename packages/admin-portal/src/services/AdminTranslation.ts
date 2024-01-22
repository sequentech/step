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

const hebrewMessages = {
    ...englishMessages,
    ra: {
        ...englishMessages.ra,
        action: {
            ...englishMessages.ra.action,
            add_filter: "הוסף סינון",
            select_columns: "עמודות",
            save: "שמור",
            confirm: "אישור",
            create: "יצירה",
            create_item: "יצירת %{item}",
            delete: "מחק",
            edit: "ערוך",
            export: "יצא",
            list: "רשימה",
            refresh: "רענן",
        },
    },
}

export const adminI18nProvider = polyglotI18nProvider(
    (locale) => {
        return locale === "es"
            ? spanishMessages
            : locale === "he"
            ? hebrewMessages
            : englishMessages
    },
    "en" // Default locale
)
