// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const spanishTranslation: TranslationType = {
    translations: {
        404: {
            title: "Página no encontrada",
            subtitle: "La página que busca no existe",
        },
        homeScreen: {
            step1: "Paso 1: Importe su papeleta electoral.",
            description1:
                "Para continuar, por favor importe los datos de las papeletas encriptadas proporcionados en el Portal de Votación:",
            importBallotHelpDialog: {
                title: "Información: Importe su papeleta electoral",
                ok: "OK",
                content:
                    "Para continuar, por favor importe los datos de las papeletas encriptadas proporcionados en el Portal de Votación.",
            },
            step2: "Paso 2: Inserte su ID de papeleta.",
            description2:
                "Por favor ingrese el ID de la papeleta proporcionado en el Portal de Votación:",
            ballotIdHelpDialog: {
                title: "Información: Su ID de papeleta",
                ok: "OK",
                content:
                    "Por favor ingrese el ID de la papeleta proporcionado en el Portal de Votación.",
            },
            startButton: "Seleccione fichero",
            dragDropOption: "O arrastre el fichero aquí",
            importErrorDescription:
                "Hubo un problema al importar el voto auditable. ¿Eligió el archivo correcto?",
            importErrorMoreInfo: "Más información",
            importErrorTitle: "Error",
            ciphertextErrorTitle: "La verificación del voto ha fallado",
            ciphertextErrorDescription:
                "El texto cifrado de este voto auditable no es el cifrado del texto en claro y la aleatoriedad que contiene. No se puede confiar en el voto.",
            useSampleLink: "Use voto de ejemplo",
            nextButton: "Continuar",
            ballotIdLabel: "ID de papeleta",
            ballotIdPlaceholder: "Escriba aquí su ID de papeleta",
            fileUploaded: "Cargado",
            receiptTitle: "Recibo de transparencia",
            receiptDescription:
                "Esta elección publica un recibo de cada papeleta emitida. Compruebe el suyo: que está en el registro público, sin cambios e igual para todos.",
            receiptButton: "Comprobar un recibo",
        },
        confirmationScreen: {
            ballotIdTitle: "ID de papeleta",
            ballotIdDescription:
                "A continuación, el sistema muestra el ID de la papeleta descodificada y el generado por el verificador.",
            ballotIdError: "No coincide con el ID de papeleta decodificado.",
            decodedBallotId: "Id de papeleta decodificado",
            decodedBallotIdHelpDialog: {
                title: "Información: Id de papeleta decodificado",
                ok: "OK",
                content:
                    "Este es el ID de papeleta extraído del fichero de la Papeleta Auditable descodificada que proporcionaste.",
            },
            yourBallotId: "La Id de papeleta que proporcionaste",
            userBallotIdHelpDialog: {
                title: "Información: La Id de papeleta que proporcionaste",
                ok: "OK",
                content:
                    "Esta es la Id de la papeleta que escribiste en el anterior paso y que recogiste de la Cabina de Votación.",
            },
            backButton: "Atrás",
            printButton: "Imprimir",
            finishButton: "Verificado",
            verifySelectionsTitle: "Verifique sus selecciones en la papeleta",
            verifySelectionsDescription:
                "Las siguientes selecciones de la papeleta han sido descodificadas de la papeleta que importó. Por favor, revíselas y asegúrese de que coincidan con las selecciones que hizo en el Portal de Votación. Si sus selecciones no coinciden, por favor, contacte con las autoridades electorales...",
            verifySelectionsHelpDialog: {
                title: "Información: Verifique sus selecciones en la papeleta",
                ok: "OK",
                content:
                    "Las siguientes selecciones de la papeleta han sido descodificadas de la papeleta que importó. Por favor, revíselas y asegúrese de que coincidan con las selecciones que hizo en el Portal de Votación. Si sus selecciones no coinciden, por favor, contacte con las autoridades electorales...",
            },
            markedInvalid: "Voto explícitamente marcado inválido",
            points_one: "({{count}} Punto)",
            points_many: "({{count}} Puntos)",
            points_other: "({{count}} Puntos)",
            contestNotFound: "Pregunta no encontrada: {{contestId}}",
            declineToVote: "Se abstuvo de votar",
            blankBallot: "Papeleta en blanco",
            acclamationDescription:
                "Esta votación se ha resuelto por aclamación. Sus candidaturas resultan elegidas sin votación, por lo que no se puede seleccionar ninguna opción ni se registra ningún voto.",
        },
        receiptScreen: {
            title: "Comprobar un recibo de transparencia",
            description:
                "Un recibo de transparencia muestra que su papeleta está en el registro público de la elección, sin cambios e igual para todos. Esta página lo comprueba en su navegador, con la política de confianza que publicó la autoridad electoral.",
            loading: "Cargando el comprobador de recibos…",
            loadFailed:
                "No se ha podido cargar el comprobador de recibos. Recargue la página para volver a intentarlo.",
            receipt: {
                title: "Su recibo",
                description:
                    "Abra el archivo del recibo que guardó al votar, o pegue el recibo, su enlace o su referencia (el texto de su código QR).",
                fileFormat: "Los archivos de recibo terminan en .vitrine-receipt",
                pasteLabel: "Recibo, enlace o referencia",
                pastePlaceholder: "Péguelo aquí",
                open: "Abrir",
                another: "Comprobar otro recibo",
                problems: {
                    "empty": "Pegue primero un recibo, su enlace o su referencia.",
                    "too-large": "Es demasiado grande para ser un recibo.",
                    "bad-link":
                        "Este enlace no contiene un recibo que se pueda leer. Abra el archivo del recibo.",
                    "bad-reference":
                        "Esta referencia está incompleta o dañada. Cópiela de nuevo o abra el archivo del recibo.",
                    "bad-chain": "Este recibo está dañado: no se puede leer.",
                    "not-https":
                        "El enlace indica que la política está en una dirección que no es segura (https), así que no se usa.",
                    "wrong-box":
                        "Esto es una política de confianza, no un recibo. Abra primero el recibo.",
                    "unrecognised":
                        "Esto no es un recibo, ni el enlace o la referencia de un recibo.",
                    "unreadable": "No se ha podido leer el archivo.",
                },
            },
            policy: {
                title: "Política de confianza",
                description:
                    "Los recibos se comprueban con la política de confianza que publicó la autoridad electoral. Indíquela aquí, con su texto o su dirección https, tal como figura en el anuncio oficial de la elección.",
                fromLink:
                    "El enlace del recibo indica dónde está su política de confianza. Antes de fijarla en este dispositivo, compare sus palabras y su ID con el anuncio oficial de la elección.",
                inputLabel: "Política de confianza o su dirección",
                inputPlaceholder: "Pegue la política o su dirección https",
                load: "Cargar la política",
                openFile: "Abrir un archivo de política",
                loading: "Cargando la política de confianza…",
                pin: "Coinciden: fijar esta política",
                reject: "No coinciden",
                rejected:
                    "No use esta política. Avise a la autoridad electoral e indique la política del anuncio oficial de la elección.",
                pinned: "Comprobado con la política de confianza que fijó en este dispositivo.",
                unpin: "Olvidar esta política",
                draft: "Esto es un borrador de la política, no la política sellada, así que no se puede fijar. Sus palabras de borrador:",
                invalid: "Esto no es una política de confianza válida.",
                otherPolicy: "Esta política de confianza es válida, pero el recibo es de otra.",
                ambiguous:
                    "Más de una política fijada coincide con esta referencia, así que no se usa. Abra el archivo del recibo.",
                problems: {
                    "empty": "Pegue primero la política de confianza o su dirección.",
                    "too-large": "Es demasiado grande para ser una política de confianza.",
                    "not-https": "Solo se usan direcciones que empiezan por https.",
                    "wrong-box": "Esto es un recibo, no una política de confianza.",
                    "unrecognised": "Esto no es una política de confianza ni su dirección.",
                    "unreadable": "No se ha podido leer el archivo.",
                },
                fetchProblems: {
                    "not-https": "Solo se usan direcciones que empiezan por https.",
                    "unreachable":
                        "No se ha podido cargar la política de confianza. Compruebe la dirección o pegue la política.",
                    "status":
                        "El servidor no ha devuelto la política de confianza (estado {{status}}).",
                    "too-large": "Es demasiado grande para ser una política de confianza.",
                    "not-text": "La dirección no ha devuelto una política de confianza.",
                },
            },
            ballot: {
                title: "Su papeleta",
                description:
                    "El recibo solo se muestra como suyo para la papeleta que emitió. Escriba el ID de papeleta que le mostró el portal de votación.",
                ballotId: "ID de papeleta",
                ballotIdPlaceholder: "Escriba su ID de papeleta",
                election: "ID de la elección",
                electionPlaceholder: "Elija la elección",
                event: "ID del evento electoral",
                check: "Comprobar el recibo",
                change: "Cambiar el ID de papeleta",
                invalid:
                    "Revise el ID de papeleta: tiene 64 caracteres, dígitos y letras de la a a la f.",
                lookedFor: "ID de papeleta buscado:",
            },
            result: {
                title: "Resultado",
                checking: "Comprobando…",
                rounds: {
                    log: "Comprobado con los registros de la elección. En la próxima media hora, esta página también consultará las réplicas de los registros, los hubs de anclaje y los monitores, en un momento aleatorio, para que no puedan saber cuándo abrió su recibo. Mantenga la página abierta o compruébelo todo ahora.",
                    full: "Comprobado con los registros de la elección, sus réplicas, los hubs de anclaje y los monitores.",
                    offline:
                        "Los registros de esta elección son privados, así que el recibo se ha comprobado con lo que contiene.",
                },
                checkEverything: "Comprobarlo todo ahora",
                checkAgain: "Volver a comprobar",
                retry: "Reintentar",
                problems: {
                    "policy-mismatch": "Esta referencia es de otra política de confianza.",
                    "unknown-log":
                        "La política de confianza no tiene ningún registro con este nombre.",
                    "quiet-period":
                        "El registro no sirve pruebas durante su periodo de silencio. Vuelva a intentarlo después de {{time}}.",
                    "not-found":
                        "El registro aún no tiene esta entrada. Si votó hace poco, vuelva a intentarlo más tarde.",
                    "entry-mismatch":
                        "La entrada del registro en esta posición no es la de la referencia.",
                    "private-reference":
                        "Los registros de esta elección son privados, así que aquí no se puede comprobar una referencia. Abra el archivo del recibo.",
                    "failed": "No se ha podido comprobar el recibo. Vuelva a intentarlo.",
                },
                evidenceSaved:
                    "Se ha guardado el archivo del recibo. Contiene los objetos firmados que se contradicen: un auditor puede comprobarlo, y crear con él el archivo de pruebas, con el verificador de Vitrine.",
                advice: {
                    failed: "Una comprobación ha fallado: hay datos firmados que se contradicen entre sí o con la política de confianza. Guarde las pruebas y entréguelas, con el archivo del recibo, a la autoridad electoral y a los observadores que nombra la política de confianza.",
                    late: "Algo que ya debería estar aún no está. Puede que solo sea lentitud: vuelva a comprobarlo más tarde y, si sigue con retraso, avise a la autoridad electoral. Guarde el archivo del recibo.",
                },
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
            explicit: {
                notAllowed:
                    "Voto marcado explícitamente como inválido pero la pregunta no lo permite",
            },
        },
    },
}

export default spanishTranslation
