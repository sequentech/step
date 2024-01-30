// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const hebrewTranslation = {
    translations: {
        language: "עברית",
        welcome: "שלום <br/> <strong>עולם</strong>",
        breadcrumbSteps: {
            select: "בחר מאמת",
            import: "ייבא נתונים",
            verify: "אמת",
            finish: "סיום",
        },
        electionEventBreadcrumbSteps: {
            created: "נוצר",
            keys: "מפתחות",
            publish: "פרסם",
            started: "התחיל",
            ended: "הסתיים",
            results: "תוצאות",
        },
        candidate: {
            moreInformationLink: "מידע נוסף",
            writeInsPlaceholder: "הקלד מועמד כתיבה ידנית כאן",
        },
        homeScreen: {
            title: "בודק קלפי הצבעה של Sequent",
            description1:
                "בודק הקלפים משמש כאשר הבוחר בוחר לבדוק את הקלף בתיק בחדר ההצבעה. אמיתות הבדיקה אמורות לקחת 1-2 דקות.",
            description2:
                "בודק הקלפים מאפשר לבוחר לוודא שהקלף המוצפן תפס את הבחירות שביצע בחדר ההצבעה. האפשרות לבצע בדיקה זו נקראת אמיתות-כפי-שנכוונה ומונעת שגיאות ופעילות זדונית בעת הצפנת הקלף.",
            descriptionMore: "למדו עוד",
            startButton: "סייר קובץ",
            dragDropOption: "או גררו ושחררו כאן",
            importErrorDescription: "הייתה בעיה בייבוא הקלף הניתן לבדיקה. ?בחרת בקובץ הנכון",
            importErrorMoreInfo: "מידע נוסף",
            importErrorTitle: "שגיאה",
            useSampleText: "אין לך קלף ניתן לבדיקה?",
            useSampleLink: "השתמש בקלף ניתן לבדיקה מדגם",
        },
        confirmationScreen: {
            title: "בודק קלפי הצבעה של Sequent",
            topDescription1: "בהתאם למידע בקלף הניתן לבדיקה שיובא, חישבנו כי:",
            topDescription2: "אם זהו המספר הזה של הקלף בחדר ההצבעה:",
            bottomDescription1:
                "הקלף שלך הוצפן בצורה נכונה. ניתן כעת לסגור חלון זה ולחזור לחדר ההצבעה.",
            bottomDescription2:
                "אם הם אינם תואמים, לחץ כאן למידע נוסף על הסיבות האפשריות והפעולות שאתה יכול לבצע.",
            ballotChoicesDescription: "והבחירות שלך הן:",
            helpAndFaq: "עזרה ושאלות נפוצות",
            backButton: "חזרה",
            markedInvalid: "הקלף סומן באופן מפורש כלא חוקי",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "סטטוס",
                content: "לוח המצב נותן לך מידע אודות האמיתות שבוצעו.",
                ok: "אישור",
            },
        },
        poweredBy: "מופעל על ידי",
        errors: {
            encoding: {
                notEnoughChoices: "אין מספיק בחירות להצפנה",
                writeInChoiceOutOfRange: "בחירת כתיבה ידנית מחוץ לטווח: {{index}}",
                writeInNotEndInZero: "הכתיבה לא מסתיימת ב-0",
                bytesToUtf8Conversion:
                    "שגיאה בהמרת כתיבה ידנית מבתים למחרוזת UTF-8: {{errorMessage}}",
                ballotTooLarge: "הקלף גדול ממה שצפוי",
            },
            implicit: {
                selectedMax: "מספר הבחירות שנבחרו {{numSelected}} גדול מהמקסימום {{max}}",
                selectedMin: "מספר הבחירות שנבחרו {{numSelected}} קטן מהמינימום {{min}}",
            },
            explicit: {
                notAllowed: "הקלף סומן כלא חוקי מקודם אך השאלה אינה מאפשרת זאת",
            },
        },
        ballotHash: "מספר הקלף שלך: {{ballotId}}",
        version: {
            header: "גרסה:",
        },
        logout: {
            buttonText: "התנתק",
            modal: {
                title: "האם אתה בטוח שברצונך להתנתק?",
                content: "אתה עומד לסגור את היישום הזה. לא ניתן לבטל פעולה זו. ",
                ok: "אישור",
                close: "סגור",
            },
        },
        stories: {
            openDialog: "פתח דיאלוג",
        },
        dragNDrop: {
            firstLine: "גררו קבצים או",
            browse: "עיין",
            format: "פורמטים נתמכים: txt",
        },
        selectElection: {
            electionWebsite: "אתר הבחירות",
            openElection: "פתח",
            closedElection: "סגור",
            voted: "הצביע",
            notVoted: "לא הצביע",
            resultsButton: "תוצאות הבחירות",
            voteButton: "לחץ להצבעה",
            openDate: "פתיחה: ",
            closeDate: "סגירה: ",
            ballotLocator: "אתר את הקלף שלך",
        },
        header: {
            profile: "פרופיל",
        },
    },
}

export type TranslationType = typeof hebrewTranslation

export default hebrewTranslation
