// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationType} from "./en"

const galegoTranslation: TranslationType = {
    translations: {
        404: {
            title: "Páxina non atopada",
            subtitle: "A páxina que estás buscando non existe",
        },
        homeScreen: {
            step1: "Paso 1: Importa a túa papeleta",
            description1:
                "Para continuar, importa os datos da papeleta cifrada proporcionados no Portal de Votación:",
            importBallotHelpDialog: {
                title: "Información: Importa a túa papeleta",
                ok: "Aceptar",
                content:
                    "Para continuar, importa os datos da papeleta cifrada proporcionados no Portal de Votación.",
            },
            step2: "Paso 2: Introduce o teu ID de papeleta",
            description2: "Escribe o ID da papeleta proporcionado no Portal de Votación:",
            ballotIdHelpDialog: {
                title: "Información: O teu ID de papeleta",
                ok: "Aceptar",
                content: "Escribe o ID da papeleta proporcionado no Portal de Votación.",
            },
            startButton: "Seleccionar arquivo",
            dragDropOption: "Ou arrástrao e déixao aquí",
            importErrorDescription:
                "Houbo un problema ao importar a papeleta auditábel. Escolléchelo arquivo correcto?",
            importErrorMoreInfo: "Máis información",
            importErrorTitle: "Erro",
            ciphertextErrorTitle: "A verificación da papeleta fallou",
            ciphertextErrorDescription:
                "O texto cifrado desta papeleta auditábel non é o cifrado do texto en claro e a aleatoriedade que contén. Non se pode confiar na papeleta.",
            useSampleLink: "Usar unha papeleta de exemplo",
            nextButton: "Seguinte",
            ballotIdLabel: "ID de papeleta",
            ballotIdPlaceholder: "Escribe o teu ID de papeleta",
            fileUploaded: "Cargado",
            receiptTitle: "Recibo de transparencia",
            receiptDescription:
                "Esta elección publica un recibo de cada papeleta emitida. Comproba o teu: que está no rexistro público, sen cambios e igual para todo o mundo.",
            receiptButton: "Comprobar un recibo",
        },
        confirmationScreen: {
            ballotIdTitle: "ID de papeleta",
            ballotIdDescription:
                "Abaixo o sistema mostra o ID da papeleta descifrada e o xerado polo verificador",
            ballotIdError: "Non coincide co ID da papeleta descifrada",
            decodedBallotId: "ID de papeleta descifrada",
            decodedBallotIdHelpDialog: {
                title: "Información: ID de papeleta descifrada",
                ok: "Aceptar",
                content:
                    "Este é o ID da papeleta lido a partir do ficheiro de papeleta auditábel que proporcionaches.",
            },
            yourBallotId: "O ID de papeleta que proporcionaches",
            userBallotIdHelpDialog: {
                title: "Información: O ID de papeleta que proporcionaches",
                ok: "Aceptar",
                content:
                    "Este é o ID de papeleta que introduciches no paso anterior e que recolliches no posto de votación.",
            },
            backButton: "Volver",
            printButton: "Imprimir",
            finishButton: "Verificado",
            verifySelectionsTitle: "Verifica as túas seleccións de papeleta",
            verifySelectionsDescription:
                "As seguintes seleccións de papeleta foron descifradas da papeleta que importaches. Revísaas e asegúrate de que coinciden coas que fixeches no Portal de Votación. Se as túas seleccións non coinciden, contacta coas autoridades electorais...",
            verifySelectionsHelpDialog: {
                title: "Información: Verifica as túas seleccións de papeleta",
                ok: "Aceptar",
                content:
                    "As seguintes seleccións de papeleta foron descifradas da papeleta que importaches. Revísaas e asegúrate de que coinciden coas que fixeches no Portal de Votación. Se as túas seleccións non coinciden, contacta coas autoridades electorais...",
            },
            markedInvalid: "Papeleta marcada explícitamente como inválida",
            points_one: "({{count}} Punto)",
            points_many: "({{count}} Puntos)",
            points_other: "({{count}} Puntos)",
            contestNotFound: "Pregunta non atopada: {{contestId}}",
            declineToVote: "Decidiu non votar",
            blankBallot: "Papeleta en branco",
            acclamationDescription:
                "Esta pregunta resolveuse por aclamación. As súas candidaturas resultan elixidas sen votación, polo que non se pode seleccionar ningunha opción nin se rexistra ningún voto.",
        },
        receiptScreen: {
            title: "Comprobar un recibo de transparencia",
            description:
                "Un recibo de transparencia mostra que a túa papeleta está no rexistro público da elección, sen cambios e igual para todo o mundo. Esta páxina compróbao no teu navegador, coa política de confianza que publicou a autoridade electoral.",
            loading: "Cargando o comprobador de recibos…",
            loadFailed:
                "Non se puido cargar o comprobador de recibos. Recarga a páxina para tentalo de novo.",
            receipt: {
                title: "O teu recibo",
                description:
                    "Abre o ficheiro do recibo que gardaches ao votar, ou pega o recibo, a súa ligazón ou a súa referencia (o texto do seu código QR).",
                fileFormat: "Os ficheiros de recibo rematan en .vitrine-receipt",
                pasteLabel: "Recibo, ligazón ou referencia",
                pastePlaceholder: "Pégao aquí",
                open: "Abrir",
                another: "Comprobar outro recibo",
                problems: {
                    "empty": "Pega primeiro un recibo, a súa ligazón ou a súa referencia.",
                    "too-large": "É grande de máis para ser un recibo.",
                    "bad-link":
                        "Esta ligazón non contén un recibo que se poida ler. Abre o ficheiro do recibo.",
                    "bad-reference":
                        "Esta referencia está incompleta ou danada. Cópiaa de novo ou abre o ficheiro do recibo.",
                    "bad-chain": "Este recibo está danado: non se pode ler.",
                    "not-https":
                        "A ligazón indica que a política está nun enderezo que non é seguro (https), así que non se usa.",
                    "wrong-box":
                        "Isto é unha política de confianza, non un recibo. Abre primeiro o recibo.",
                    "unrecognised":
                        "Isto non é un recibo, nin a ligazón ou a referencia dun recibo.",
                    "unreadable": "Non se puido ler o ficheiro.",
                },
            },
            policy: {
                title: "Política de confianza",
                description:
                    "Os recibos compróbanse coa política de confianza que publicou a autoridade electoral. Indícaa aquí, co seu texto ou o seu enderezo https, tal como figura no anuncio oficial da elección.",
                fromLink:
                    "A ligazón do recibo indica onde está a súa política de confianza. Antes de fixala neste dispositivo, compara as súas palabras e o seu ID co anuncio oficial da elección.",
                inputLabel: "Política de confianza ou o seu enderezo",
                inputPlaceholder: "Pega a política ou o seu enderezo https",
                load: "Cargar a política",
                openFile: "Abrir un ficheiro de política",
                loading: "Cargando a política de confianza…",
                pin: "Coinciden: fixar esta política",
                reject: "Non coinciden",
                rejected:
                    "Non uses esta política. Avisa á autoridade electoral e indica a política do anuncio oficial da elección.",
                pinned: "Comprobado coa política de confianza que fixaches neste dispositivo.",
                unpin: "Esquecer esta política",
                draft: "Isto é un borrador da política, non a política selada, así que non se pode fixar. As súas palabras de borrador:",
                invalid: "Isto non é unha política de confianza válida.",
                otherPolicy: "Esta política de confianza é válida, pero o recibo é doutra.",
                ambiguous:
                    "Máis dunha política fixada coincide con esta referencia, así que non se usa. Abre o ficheiro do recibo.",
                problems: {
                    "empty": "Pega primeiro a política de confianza ou o seu enderezo.",
                    "too-large": "É grande de máis para ser unha política de confianza.",
                    "not-https": "Só se usan enderezos que comezan por https.",
                    "wrong-box": "Isto é un recibo, non unha política de confianza.",
                    "unrecognised": "Isto non é unha política de confianza nin o seu enderezo.",
                    "unreadable": "Non se puido ler o ficheiro.",
                },
                fetchProblems: {
                    "not-https": "Só se usan enderezos que comezan por https.",
                    "unreachable":
                        "Non se puido cargar a política de confianza. Comproba o enderezo ou pega a política.",
                    "status":
                        "O servidor non devolveu a política de confianza (estado {{status}}).",
                    "too-large": "É grande de máis para ser unha política de confianza.",
                    "not-text": "O enderezo non devolveu unha política de confianza.",
                },
            },
            ballot: {
                title: "A túa papeleta",
                description:
                    "O recibo só se mostra como teu para a papeleta que emitiches. Escribe o ID de papeleta que che mostrou o portal de votación.",
                ballotId: "ID de papeleta",
                ballotIdPlaceholder: "Escribe o teu ID de papeleta",
                election: "ID da elección",
                electionPlaceholder: "Escolle a elección",
                event: "ID do evento electoral",
                check: "Comprobar o recibo",
                change: "Cambiar o ID de papeleta",
                invalid: "Revisa o ID de papeleta: ten 64 caracteres, díxitos e letras da a á f.",
                lookedFor: "ID de papeleta buscado:",
            },
            result: {
                title: "Resultado",
                checking: "Comprobando…",
                rounds: {
                    log: "Comprobado cos rexistros da elección. Na próxima media hora, esta páxina tamén consulta as réplicas dos rexistros, os hubs de ancoraxe e os monitores, nun momento aleatorio, para que non poidan saber cando abriches o teu recibo. Mantén a páxina aberta ou compróbao todo agora.",
                    full: "Comprobado cos rexistros da elección, as súas réplicas, os hubs de ancoraxe e os monitores.",
                    offline:
                        "Os rexistros desta elección son privados, así que o recibo comprobouse co que contén.",
                },
                checkEverything: "Comprobalo todo agora",
                checkAgain: "Comprobar de novo",
                retry: "Tentar de novo",
                problems: {
                    "policy-mismatch": "Esta referencia é doutra política de confianza.",
                    "unknown-log": "A política de confianza non ten ningún rexistro con este nome.",
                    "quiet-period":
                        "O rexistro non serve probas durante o seu período de silencio. Téntao de novo despois de {{time}}.",
                    "not-found":
                        "O rexistro aínda non ten esta entrada. Se votaches hai pouco, téntao de novo máis tarde.",
                    "entry-mismatch": "A entrada do rexistro nesta posición non é a da referencia.",
                    "private-reference":
                        "Os rexistros desta elección son privados, así que aquí non se pode comprobar unha referencia. Abre o ficheiro do recibo.",
                    "failed": "Non se puido comprobar o recibo. Téntao de novo.",
                },
                evidenceSaved:
                    "Gardouse o ficheiro do recibo. Contén os obxectos asinados que se contradín: un auditor pode comprobalo, e crear con el o ficheiro de probas, co verificador de Vitrine.",
                advice: {
                    failed: "Unha comprobación fallou: hai datos asinados que se contradín entre si ou coa política de confianza. Garda as probas e entrégaas, co ficheiro do recibo, á autoridade electoral e aos observadores que nomea a política de confianza.",
                    late: "Algo que xa debería estar aínda non está. Pode que só sexa lentitude: compróbao de novo máis tarde e, se segue con atraso, avisa á autoridade electoral. Garda o ficheiro do recibo.",
                },
            },
        },
        footer: {
            poweredBy: "Impulsado por <1></1>",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Non hai suficientes opcións para descodificar",
                writeInChoiceOutOfRange: "A opción escrita está fóra de rango: {{index}}",
                writeInNotEndInZero: "A opción escrita non remata en 0",
                bytesToUtf8Conversion:
                    "Erro ao converter a opción escrita de bytes a cadea UTF-8: {{errorMessage}}",
                ballotTooLarge: "A papeleta é máis grande do esperado",
            },
            explicit: {
                notAllowed: "A papeleta está marcada como inválida pero a pregunta non o permite",
            },
        },
    },
}

export default galegoTranslation
