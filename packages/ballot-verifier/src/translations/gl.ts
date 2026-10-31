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
            step2: "Paso 2: Introduce o teu ID de seguimento",
            description2:
                "Pega o ID de seguimento completo do Portal de Votación: o botón de copiar xunto ao teu ID de seguimento cópiao.",
            ballotIdHelpDialog: {
                title: "Información: O teu ID de seguimento",
                ok: "Aceptar",
                content:
                    "Pega o ID de seguimento completo proporcionado no Portal de Votación. O botón de copiar que hai xunto a el cópiao.",
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
            verificationErrorTitle: "A verificación da papeleta fallou",
            unpublishedStyleErrorDescription:
                "Esta papeleta auditábel non corresponde a ningunha papeleta publicada para este evento electoral, polo que non se pode verificar.",
            unavailableStyleErrorDescription:
                "Non se puideron cargar as papeletas publicadas para este evento electoral, polo que esta papeleta auditábel aínda non se pode verificar. Por favor, ténteo de novo.",
            styleErrorDescription:
                "O estilo de papeleta desta papeleta auditábel non é o publicado para a elección: a súa chave pública ou os seus concursos e candidaturas difiren, ou non está publicado. Non se pode confiar na papeleta.",
            legacyBallotWarning:
                "Esta papeleta auditábel usa un formato anterior cuxo ID de seguimento non cobre o estilo de papeleta. O seu ID non pode confirmar que se cifrou coa chave publicada da elección.",
            useSampleLink: "Usar unha papeleta de exemplo",
            nextButton: "Seguinte",
            fileUploaded: "Cargado",
            receiptTitle: "Recibo de transparencia",
            receiptDescription:
                "Esta elección publica un recibo de cada papeleta emitida. Comproba o teu: que está no rexistro público, sen cambios e igual para todo o mundo.",
            receiptButton: "Comprobar un recibo",
        },
        confirmationScreen: {
            ballotIdTitle: "ID de seguimento",
            ballotIdDescription:
                "Abaixo, o sistema mostra o ID de seguimento da papeleta descifrada e o que proporcionaches.",
            ballotIdError: "Non coincide co ID de seguimento descifrado",
            decodedBallotId: "ID de seguimento descifrado",
            decodedBallotIdHelpDialog: {
                title: "Información: ID de seguimento descifrado",
                ok: "Aceptar",
                content:
                    "Este é o ID de seguimento lido a partir do ficheiro de papeleta auditábel que proporcionaches.",
            },
            yourBallotId: "O ID de seguimento que proporcionaches",
            userBallotIdHelpDialog: {
                title: "Información: O ID de seguimento que proporcionaches",
                ok: "Aceptar",
                content:
                    "Este é o ID de seguimento que introduciches no paso anterior e que recolliches no posto de votación.",
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
                    "O recibo só se mostra como teu para a papeleta que emitiches. Pega o ID de seguimento completo que che mostrou o portal de votación.",
                election: "ID da elección",
                electionPlaceholder: "Escolle a elección",
                event: "ID do evento electoral",
                check: "Comprobar o recibo",
                change: "Cambiar o ID de seguimento",
                lookedFor: "ID de seguimento buscado",
                fullId: "ID de seguimento completo",
                stage: "Paso da papeleta",
                stages: {received: "Recibida", cast: "Emitida", audited: "Auditada"},
            },
            result: {
                title: "Resultado",
                rounds: {
                    log: "Comprobado cos rexistros da elección. Na próxima media hora, esta páxina tamén consulta as réplicas dos rexistros, os hubs de ancoraxe e os monitores, nun momento aleatorio, para que non poidan saber cando o comprobaches.",
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
            status: {
                checking: {
                    head: "Comprobando…",
                    sub: "O teu navegador está a comprobar que a túa papeleta está no rexistro público.",
                },
                recorded: {
                    head: "No rexistro público",
                    sub: "O teu navegador atopou a túa papeleta no rexistro público. A continuación confírmana as testemuñas.",
                },
                witnessed: {
                    head: "Confirmado por testemuñas",
                    sub: "Testemuñas independentes confirmaron o rexistro. A continuación consúltanse os hubs públicos, na próxima media hora.",
                    subLater:
                        "Testemuñas independentes confirmaron o rexistro. As ancoraxes públicas chegan en menos de 10 minutos; non fai falta esperar.",
                },
                verified: {
                    head: "Verificada",
                    sub: "A túa papeleta está no rexistro público, confirmada por testemuñas independentes e ancorada publicamente.",
                },
                late: {
                    head: "Tarda máis do esperado",
                    sub: "A túa papeleta está no rexistro público e confirmada por testemuñas, pero as súas ancoraxes públicas van con atraso.",
                    subOther: "Algúns dos pasos de abaixo tardan máis do esperado.",
                },
                problem: {
                    head: "O rexistro non coincide",
                    sub: "A entrada n.º {{index}} do rexistro público non é a deste recibo.",
                    subOther:
                        "O rexistro público, este recibo e o teu ID de seguimento non coinciden entre si.",
                },
                damaged: {
                    head: "Non se puido comprobar",
                    sub: "Algúns datos parecen danados: o recibo ou a resposta do servidor. Isto non demostra que ninguén fixese trampa.",
                },
                evidence:
                    "O teu navegador gardou as probas: este recibo e a entrada asinada do rexistro. Envíaas á autoridade electoral ou a un observador; calquera pode comprobalas.",
                saveEvidence: "Gardar as probas",
                howToReport: "Como denuncialo",
                captionChecking:
                    "O teu navegador está a comprobar o rexistro público, con código que calquera pode inspeccionar.",
                captionChecked:
                    "Comprobado polo teu navegador, con código que calquera pode inspeccionar.",
                captionProblem:
                    "Detectado polo teu navegador, con código que calquera pode inspeccionar.",
                rows: {
                    record: "No rexistro público",
                    witnesses: "Confirmado por testemuñas",
                    anchors: "Ancorado publicamente",
                    recordDone: "Entrada n.º {{index}}. O teu navegador comprobou a súa proba.",
                    recordDoneNoIndex: "O teu navegador comprobou a súa proba.",
                    witnessesDone:
                        "{{signed}} de {{total}} testemuñas independentes asinárona; a política esixe {{needed}}.",
                    witnessesDoneAny: "Testemuñas independentes asinárona.",
                    hubs_one: "En {{count}} hub público",
                    hubs_many: "En {{count}} hubs públicos",
                    hubs_other: "En {{count}} hubs públicos",
                    browser: ", e onde comproban os navegadores",
                    standIn: ", mediante un hub suplente polo de agora",
                    unused: "A política desta elección non o usa.",
                    checking: "O teu navegador está a comprobar…",
                    anchorsActive: "Consúltase na próxima media hora, nun momento aleatorio.",
                    anchorsPending: "Aínda non está ancorada. Volve comprobalo en uns minutos.",
                    waiting: "A agardar polo paso anterior.",
                    lateAnchors: "Os hubs deberían telo ancorado ás {{time}}.",
                    lateOther: "Xa debería estar feito.",
                    recordFailed: "A entrada n.º {{index}} do rexistro difire do teu recibo.",
                    failed: "Non coincide co teu recibo e co teu ID de seguimento.",
                    damaged: "Non se puido comprobar: os datos parecen danados.",
                    skipped: "Non comprobado, porque o rexistro non coincide.",
                    skippedDamaged:
                        "Non comprobado, porque un paso anterior non se puido comprobar.",
                    statusChecking: "Comprobando…",
                    withinSeconds: "En segundos",
                    within10: "En 10 min",
                    within30: "En 30 min",
                    statusLate: "Con atraso",
                    statusMismatch: "Non coincide",
                    statusDamaged: "Danado",
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
        ballotIdField: {
            label: "ID de seguimento",
            placeholder: "Pega o teu ID de seguimento completo",
            help: "Os espazos son opcionais.",
            shortError:
                "Ese é o ID curto. Pega o ID de seguimento completo: a comprobación compárao enteiro.",
            formatError:
                "Revisa o ID de seguimento: ten 64 caracteres, díxitos e as letras da a á f.",
        },
    },
}

export default galegoTranslation
