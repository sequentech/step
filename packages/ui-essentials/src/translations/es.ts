// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const spanishTranslation: TranslationType = {
    translations: {
        language: "Español",
        welcome: "Let's start: Import auditable ballot..",
        breadcrumbSteps: {
            select: "Seleccionar un Verificador",
            import: "Importar Datos",
            verify: "Verificar",
            finish: "Terminar",
        },
        electionEventBreadcrumbSteps: {
            created: "Creado",
            keys: "Claves",
            publish: "Publicar",
            started: "Iniciado",
            ended: "Finalizado",
            results: "Resultados",
        },
        candidate: {
            moreInformationLink: "Más información",
            writeInsPlaceholder: "Teclee aquí el candidato por escrito",
            blankVote: "Voto en blanco",
            preferential: {
                position: "Posición",
                none: "Ninguna",
                ordinals: {
                    first: "º",
                    second: "º",
                    third: "º",
                    other: "º",
                },
            },
        },
        homeScreen: {
            title: "Verificador de Voto Sequent",
            description1:
                "El verificador de voto se usa cuando el votante elige auditar la boleta en la cabina de votación. La verificación debe tomar de 1 a 2 minutos.",
            description2:
                "El verificador de voto le permite al votante asegurarse de que el voto cifrado capture correctamente las selecciones realizadas en la cabina de votación. Permitir realizar esta verificación se denomina verificabilidad de transmisión según lo previsto y evita errores y actividades maliciosas durante el cifrado del voto.",
            descriptionMore: "Más información",
            startButton: "Selecciona fichero",
            dragDropOption: "O arrastre el fichero aquí",
            importErrorDescription:
                "Hubo un problema al importar el voto auditable. ¿Elegiste el archivo correcto?",
            importErrorMoreInfo: "Más información",
            importErrorTitle: "Error",
            useSampleText: "¿No tiene un voto verificable?",
            useSampleLink: "Use un voto verificable de ejemplo",
        },
        confirmationScreen: {
            title: "Verificador de Voto Sequent",
            topDescription1:
                "En base a la información del voto auditable importado, calculamos que:",
            topDescription2: "Si este ID de voto es mostrado en la Cabina de Votación:",
            bottomDescription1:
                "Su voto fue cifrado correctamente. Ahora puede cerrar esta ventana y volver a la Cabina de Votación.",
            bottomDescription2:
                "Si no coinciden, haga clic aquí para obtener más información sobre los posibles motivos y las acciones que puede tomar.",
            ballotChoicesDescription: "Y sus selecciones de voto son:",
            helpAndFaq: "Ayuda y Preguntas Frecuentes",
            backButton: "Atrás",
            markedInvalid: "Voto explícitamente marcado inválido",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Estado",
                content:
                    "El panel de estado te da información sobre las verificaciones realizadas.",
                ok: "OK",
            },
        },
        footer: {
            poweredBy: "Funciona con <1></1>",
        },
        errors: {
            encoding: {
                notEnoughChoices: "No hay suficientes opciones para decodificar",
                writeInChoiceOutOfRange: "Opción de voto escrita fuera de rango: {{index}}",
                writeInNotEndInZero: "Opción de voto escrita no finaliza en 0",
                bytesToUtf8Conversion:
                    "Error convirtiendo bytes de opción de voto escrita a cadena UTF-8: {{errorMessage}}",
                ballotTooLarge: "Voto más grande de lo esperado",
            },
            implicit: {
                selectedMax:
                    "El número de opciones seleccionadas {{numSelected}} es mayor que el máximo {{max}}",
                selectedMin:
                    "El número de opciones seleccionadas {{numSelected}} es menor que el máximo {{min}}",
            },
            explicit: {
                notAllowed:
                    "Voto marcado explícitamente como inválido pero la pregunta no lo permite",
            },
        },
        ballotHash: "Su Localizador de Voto: {{ballotId}}",
        version: {
            header: "Versión:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Cerrar sesión",
            modal: {
                title: "¿Estás seguro de que quieres cerrar sesión?",
                content:
                    "Está a punto de cerrar esta aplicación. Esta acción no se puede deshacer.",
                ok: "OK",
                close: "Cerrar",
            },
        },
        stories: {
            openDialog: "Abrir Diálogo",
        },
        dragNDrop: {
            firstLine: "Arrastrar y soltar ficheros o",
            browse: "Cargar fichero",
            format: "Formatos soportados: txt",
        },
        selectElection: {
            electionWebsite: "Sitio web electoral",
            countdown:
                "La elección comienza en {{years}} años, {{months}} meses, {{weeks}} semanas, {{days}} días, {{hours}} horas, {{minutes}} minutos, {{seconds}} segundos",
            openElection: "Abierta",
            closedElection: "Cerrada",
            voted: "Votado",
            notVoted: "No votado",
            resultsButton: "Resultados de Votación",
            voteButton: "Haga click para Votar",
            openDate: "Abierta: ",
            closeDate: "Cerrada: ",
            ballotLocator: "Localiza tu voto",
        },
        header: {
            profile: "Perfil",
            welcome: "Bienvenido,<br><span>{{name}}</span>",
            session: {
                title: "Su sesión está a punto de expirar.",
                timeLeft: "Le quedan {{time}} para emitir su voto.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutos y {{time}} segundos",
                timeLeftSeconds: "{{timeLeft}} segundos",
            },
        },
    },
}

export default spanishTranslation
