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
            poweredBy: "Funciona con <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "No hay suficientes opciones para decodificar",
                writeInChoiceOutOfRange: "Opción de voto escrita fuera de rango: {{index}}",
                writeInNotEndInZero: "Opción de voto escrita no finaliza en 0",
                writeInCharsExceeded_one: "Acorte la escritura libre en {{count}} carácter.",
                writeInCharsExceeded_many: "Acorte la escritura libre en {{count}} caracteres.",
                writeInCharsExceeded_other: "Acorte la escritura libre en {{count}} caracteres.",
                bytesToUtf8Conversion:
                    "Error convirtiendo bytes de opción de voto escrita a cadena UTF-8: {{errorMessage}}",
                ballotTooLarge: "Voto más grande de lo esperado",
            },
            implicit: {
                selectedMax_one: "Desmarque {{count}} candidato.",
                selectedMax_many: "Desmarque {{count}} candidatos.",
                selectedMax_other: "Desmarque {{count}} candidatos.",
                selectedMin_one: "Seleccione {{count}} candidato más.",
                selectedMin_many: "Seleccione {{count}} candidatos más.",
                selectedMin_other: "Seleccione {{count}} candidatos más.",
                maxSelectionsPerType_one: "Desmarque {{count}} candidato de {{type}}.",
                maxSelectionsPerType_many: "Desmarque {{count}} candidatos de {{type}}.",
                maxSelectionsPerType_other: "Desmarque {{count}} candidatos de {{type}}.",
                underVote_one: "Seleccione hasta {{count}} candidato más.",
                underVote_many: "Seleccione hasta {{count}} candidatos más.",
                underVote_other: "Seleccione hasta {{count}} candidatos más.",
                overVoteDisabled_one:
                    "Ha seleccionado el máximo de {{count}} candidato. Desmárquelo para elegir otro.",
                overVoteDisabled_many:
                    "Ha seleccionado el máximo de {{count}} candidatos. Desmarque uno para elegir otro.",
                overVoteDisabled_other:
                    "Ha seleccionado el máximo de {{count}} candidatos. Desmarque uno para elegir otro.",
                blankVote: "No ha seleccionado ningún candidato.",
                preferenceOrderWithGaps:
                    "¡Voto inválido! El orden de preferencia tiene uno o más huecos.",
                duplicatedPosition:
                    "¡Voto inválido! La misma posición fue seleccionada para dos o más candidatos.",
            },
            explicit: {
                notAllowed:
                    "Voto marcado explícitamente como inválido pero la pregunta no lo permite",
                alert: "La selección marcada será considerada voto inválido.",
            },
            configuration: {
                multipleExplicitInvalidCandidates:
                    "Configuración de voto inválida: el concurso define {{count}} candidatos explícitamente inválidos, pero solo se permite uno.",
                multipleExplicitBlankCandidates:
                    "Configuración de voto inválida: el concurso define {{count}} candidatos de voto en blanco explícito, pero solo se permite uno.",
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
            welcome: "Bienvenido/a,<br><span>{{name}}</span>",
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
