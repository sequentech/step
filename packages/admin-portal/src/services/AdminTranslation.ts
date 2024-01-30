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
        navigation: {
            no_results: "לא נמצאו תוצאות",
            no_more_results: "מספר הדף %{page} חורג מהגבולות. נסה את הדף הקודם.",
            page_out_of_boundaries: "מספר הדף %{page} מחוץ לגבולות",
            page_out_from_end: "לא ניתן להמשיך אחרי הדף האחרון",
            page_out_from_begin: "לא ניתן לחזור לפני הדף הראשון",
            page_range_info: "%{offsetBegin}-%{offsetEnd} מתוך %{total}",
            partial_page_range_info: "%{offsetBegin}-%{offsetEnd} מתוך מעל %{offsetEnd}",
            current_page: "דף %{page} נוכחי",
            page: "עבור לדף %{page}",
            first: "עבור לדף הראשון",
            last: "עבור לדף האחרון",
            next: "עבור לדף הבא",
            previous: "עבור לדף הקודם",
            page_rows_per_page: "שורות לעמוד:",
            skip_nav: "דלג לתוכן",
        },
        sort: {
            sort_by: "מיין לפי %{field} %{order}",
            ASC: "סדר עולה",
            DESC: "סדר יורד",
        },
    },
    resources: {
        user: {
            fields: {
                title: "משתמשים ותפקידים",
                subtitle: "תצורה כללית",
                mobile_number: "טלפון נייד",
                email_verified: "אימייל מאומת",
                email: "דואר",
                enabled: "מופעל",
                first_name: "שם פרטי",
                last_name: "שם משפחה",
                username: "שם משתמש",
                actions: "פעולות",
            },
        },
        sequent_backend_area: {
            fields: {
                name: "שם",
                description: "תיאור",
                actions: "פעולות",
            },
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
    "en", // Default locale
    [
        {locale: "en", name: "English"},
        {locale: "fr", name: "Français"},
        {locale: "he", name: "Hebrew"},
    ] // Locales list
)
