// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const tagalogTranslation: TranslationType = {
    translations: {
        common: {
            goBack: "Bumalik",
        },
        breadcrumbSteps: {
            electionList: "Listahan ng mga Balota",
            ballot: "Balota",
            review: "Suriin",
            confirmation: "Kumpirmasyon",
            audit: "Audit",
        },
        votingScreen: {
            backButton: "Bumalik",
            reviewButton: "Susunod",
            clearButton: "Burahin ang mga napili",
            ballotHelpDialog: {
                title: "Impormasyon: Screen ng Balota",
                content:
                    "Ipinapakita sa screen na ito ang mga paligsahan na maaari mong botohan. Maaari kang pumili sa pamamagitan ng pag-activate ng checkbox sa kanang bahagi ng Kandidato/Sagot. Upang i-reset ang iyong mga pagpili, i-click ang “<b>Burahin ang pagpili</b>” na button, upang pumunta sa susunod na hakbang, i-click ang “<b>Susunod</b>” na button sa ibaba.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Di-wasto o blangkong boto",
                content:
                    "Ang ilan sa iyong mga sagot ay magreresulta sa pagkaka-invalidate o pagkakaroon ng blangko sa balota sa isa o higit pang mga tanong.",
                ok: "Bumalik at suriin",
                continue: "Ipagpatuloy",
                cancel: "Kanselahin",
            },
            warningDialog: {
                title: "Suriin ang iyong balota",
                content:
                    "Ang iyong balota ay naglalaman ng mga pagpili na maaaring mangailangan ng iyong pansin (tulad ng pagpili ng mas kaunting opsyon kaysa sa pinapayagan). Ang iyong balota ay wasto at biblangin ayon sa iyong isinumite.",
                ok: "Bumalik at suriin",
                continue: "Magpatuloy",
                cancel: "Kanselahin",
            },
        },
        startScreen: {
            startButton: "Simulan ang Pagboto",
            instructionsTitle: "Mga Tagubilin",
            instructionsDescription: "Sundin ang mga hakbang na ito upang ikaw ay makaboto:",
            step1Title: "1. Piliin ang iyong mga sagot",
            step1Description:
                "Sagutin ang mga tanong ng halalan isa-isa habang ipinapakita ang mga ito. Maaari mong i-edit ang iyong balota hanggang handa ka nang magpatuloy.",
            step2Title: "2. Suriin ang iyong balota",
            step2Description:
                "Kapag ikaw ay kontento na sa iyong mga napili, aming i-eencript ang iyong balota at ipapakita sa iyo ang huling pagsusuri ng iyong mga napili. Makakatanggap ka rin ng natatanging tracker ID para sa iyong balota.",
            step3Title: "3. I-submit ang iyong balota",
            step3Description:
                "I-submit ang iyong balota: Sa wakas, maaari mo nang i-submit ang iyong balota upang ito ay tamang maitala. Maaari mo ring piliing i-audit at tiyakin na ang iyong balota ay tama ang pagkakalista at pagkaka-encrypt.",
        },
        reviewScreen: {
            title: "Suriin ang iyong balota",
            description:
                "Upang baguhin ng iyong mga napili, i-click ang “<b>I-edit ang balota</b>” na button, upang kumpirmahin ang iyong mga napili, i-click ang “<b>I-submit ang iyong balota</b>” na button sa ibaba, at upang i-audit ang iyong balota i-click ang “<b>I-audit ang balota</b>” na button sa ibaba.",
            descriptionNoAudit:
                "Upang baguhin ng iyong mga napili, i-click ang “<b>I-edit ang balota</b>” na button, upang kumpirmahin ang iyong mga napili, i-click ang “<b>I-submit ang iyong balota</b>” na button sa ibaba.",
            backButton: "I-edit ang balota",
            castBallotButton: "I-submit ang iyong balota",
            auditButton: "I-audit ang balota",
            reviewScreenHelpDialog: {
                title: "Impormasyon: Screen ng Pagsusuri",
                content:
                    "Maaari mo ditong suriin ang iyong mga napili bago i-submit ang iyong balota.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Hindi pa na-submit ang boto",
                content:
                    "<p>Ito ang iyong Ballot Tracker ID, ngunit <b>hindi pa nai-submit ang iyong boto</b>. Kung susubukan mong i-track ang balota, hindi mo ito mahahanap.</p><p>Ang dahilan kung bakit ipinapakita namin ang Ballot Tracker ID sa yugtong ito ay upang masuri mo ang pagiging tama ng naka-encrypt na balota bago ito i-submit.</p>",
                ok: "Tinatanggap ko na HINDI pa nai-susubmit ang aking boto",
                cancel: "Kanselahin",
            },
            auditBallotHelpDialog: {
                title: "Nais mo bang i-audit ang balota?",
                content:
                    "<p>Pakitandaan na ang pag-audit ng iyong balota ay magpapawalang-bisa dito, at kakailanganin mong magsimulang muli sa proseso ng pagboto. Sa proseso ng audit, maari mong tiyakin na ang iyong balota ay tamang naka-encode, ngunit ito ay nangangailangan ng mga advanced na teknikal na hakbang. Inirerekomenda naming magpatuloy ka lang kung ikaw ay kumpiyansa sa iyong kakayahang teknikal. Kung nais mo lamang i-submit ang iyong balota, i-click ang <u>Kanselahin</u> upang bumalik sa screen ng pagsusuri ng balota.</b></p>",
                ok: "Oo, nais kong IBASURA ang aking balota upang i-audit ito",
                cancel: "Kanselahin",
            },
            confirmCastVoteDialog: {
                title: "Sigurado ka bang nais mong i-submit ang iyong boto?",
                content: "Ang iyong boto ay hindi na maaring baguhin kung ikaw ang magkumpirma.",
                ok: "Oo, nais kong I-SUBMIT ang aking boto",
                cancel: "Kanselahin",
            },
            error: {
                NETWORK_ERROR:
                    "Nagkaroon ng problema sa network. Pakisubukan ulit mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                UNABLE_TO_FETCH_DATA:
                    "Nagkaroon ng problema sa pagkuha ng data. Pakisubukan ulit mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                LOAD_ELECTION_EVENT:
                    "Hindi ma-load ang kaganapan ng eleksyon. Pakisubukan ulit mamaya.",
                CAST_VOTE:
                    "Nagkaroon ng hindi inaasahang error habang bumoboto. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_CheckStatusFailed:
                    "Hindi pinapayagan ng halalan ang pagboto. Ang halalan ay maaaring sarado, naka-archive, o maaari kang bumoto sa labas ng itinakdang panahon.",
                CAST_VOTE_AreaNotFound:
                    "Nagkaroon ng error habang bumoboto: Hindi natagpuan ang lugar. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_InternalServerError:
                    "Nagkaroon ng internal na error habang bumoboto. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_QueueError:
                    "Nagkaroon ng problema sa pagproseso ng iyong boto. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_Unauthorized:
                    "Hindi ka awtorisadong bumoto. Pakikontak ang suporta para sa tulong.",
                CAST_VOTE_ElectionEventNotFound:
                    "Hindi matagpuan ang kaganapang elektoral. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_ElectoralLogNotFound:
                    "Hindi matagpuan ang iyong tala ng pagboto. Pakikontak ang suporta para sa tulong.",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "Nagkaroon ng error habang sinusuri ang iyong status sa pagboto. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_GetClientCredentialsFailed:
                    "Nabigo sa pag-verify ng iyong mga kredensyal. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_GetAreaIdFailed:
                    "Nagkaroon ng error sa pag-verify ng iyong lugar ng pagboto. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_GetTransactionFailed:
                    "Nagkaroon ng error sa pagproseso ng iyong boto. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_DeserializeBallotFailed:
                    "Nagkaroon ng error sa pagbasa ng iyong balota. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_DeserializeContestsFailed:
                    "Nagkaroon ng error sa pagbasa ng iyong mga napili. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_PokValidationFailed:
                    "Nabigo sa pag-validate ng iyong boto. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_UuidParseFailed:
                    "Nagkaroon ng error sa pagproseso ng iyong kahilingan. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_unexpected:
                    "Nagkaroon ng hindi kilalang error habang bumoboto. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CAST_VOTE_timeout:
                    "May error sa timeout sa pagboto. Pakisubukang muli mamaya o makipag-ugnayan sa support para sa tulong.",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "Lumampas ka na sa limitasyon ng muling pagboto. Pakisubukang muli mamaya o makipag-ugnayan sa support para sa tulong.",
                CAST_VOTE_CheckRevotesFailed:
                    "Lumampas ka na sa pinapayagang bilang ng muling pagboto. Pakisubukang muli mamaya o makipag-ugnayan sa support para sa tulong.",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "Nakaboto ka na sa ibang lugar. Pakisubukang muli mamaya o makipag-ugnayan sa support para sa tulong.",
                CAST_VOTE_UnknownError:
                    "Nagkaroon ng hindi kilalang error habang bumoboto. Pakisubukang muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                NO_BALLOT_SELECTION:
                    "Walang estado ng pagpili para sa eleksyon na ito. Pakitiyak na tama ang iyong pagpili o makipag-ugnayan sa helpdesk.",
                NO_BALLOT_STYLE: "Walang estilo ng balota. Pakikontak ang helpdesk.",
                NO_AUDITABLE_BALLOT: "Walang balota na maaring suriin. Pakikontak ang helpdesk.",
                INCONSISTENT_HASH:
                    "Nagkaroon ng error kaugnay sa proseso ng pag-hash ng balota. BallotId: {{ballotId}} ay hindi tugma sa sinusuring Hash ng Balota: {{auditableBallotHash}}. Pakireport itong isyu sa helpdesk.",
                ELECTION_EVENT_NOT_OPEN:
                    "Ang kaganapan ng eleksyon ay sarado na. Pakikontak ang helpdesk.",
                PARSE_ERROR:
                    "Nagkaroon ng error sa pag-parse ng balota. Pakisubukan ulit mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "Nagkaroon ng error sa pag-deserialize ng sinusuring balota. Pakisubukan ulit mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                DESERIALIZE_HASHABLE_ERROR:
                    "Nagkaroon ng error sa pag-deserialize ng hashable na balota. Pakisubukan ulit mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                CONVERT_ERROR:
                    "Nagkaroon ng error sa pag-convert ng balota. Pakisubukan ulit mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                SERIALIZE_ERROR:
                    "Nagkaroon ng error sa pag-serialize ng balota. Pakisubukan ulit mamaya o makipag-ugnayan sa helodesk para sa tulong.",
                UNKNOWN_ERROR:
                    "Nagkaroon ng error. Pakisubukan ulit mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                REAUTH_FAILED:
                    "Nabigo ang authentication. Pakisubukan muli o makipag-ugnayan sa helpdesk para sa tulong.",
                SESSION_EXPIRED:
                    "Ang iyong session ay nag-expire na. Pakisubukan muli mula sa simula.",
                CAST_VOTE_BallotIdMismatch: "Hindi tumutugma ang ballot ID sa ibinotong boto.",
                SESSION_STORAGE_ERROR:
                    "Hindi magamit ang session storage. Pakisubukang muli o makipag-ugnayan sa support.",
                PARSE_BALLOT_DATA_ERROR:
                    "Nagkaroon ng error sa pag-parse ng data ng balota. Pakisubukang muli mamaya o makipag-ugnayan sa support para sa tulong.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Ang data ng balota ay hindi wasto. Pakisubukang muli mamaya o makipag-ugnayan sa support para sa tulong.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "May error sa timeout sa pagkuha ng data. Pakisubukang muli mamaya o makipag-ugnayan sa support para sa tulong.",
                TO_HASHABLE_BALLOT_ERROR:
                    "May error sa pag-convert sa hashable na balota. Pakisubukang muli mamaya o makipag-ugnayan sa support para sa tulong.",
                INTERNAL_ERROR:
                    "Nagkaroon ng internal error habang nagboboto. Pakisubukang muli mamaya o makipag-ugnayan sa support para sa tulong.",
            },
        },
        confirmationScreen: {
            title: "Ang iyong boto ay nai-submit na",
            description:
                "Ang code ng kumpirmasyon sa ibaba ay nagpapatunay na <b>ang iyong balota ay matagumpay na nai-submit</b>. Maaari mong gamitin ang code na ito upang tiyakin na ang iyong balota ay nabilang.",
            ballotId: "ID ng Balota",
            printButton: "I-print",
            finishButton: "Tapos na",
            verifyCastTitle: "Tiyakin na ang iyong balota ay nai-submit",
            verifyCastDescription:
                "Maaari mong tiyakin na ang iyong balota ay nai-submit nang tama anumang oras gamit ang sumusunod na QR code:",
            confirmationHelpDialog: {
                title: "Impormasyon: Screen ng Kumpirmasyon",
                content:
                    "Ipinapakita ng screen na ito na ang iyong boto ay matagumpay na nai-submit. Ang impormasyong sa pahinang ito ay nagtitiyak na ang balota ay nai-submit sa ballot box, ang prosesong ito ay maaaring isagawa anumang oras sa panahon ng pagboto at pagkatapos na maisara ang Eleksyon.",
                ok: "OK",
            },
            demoPrintDialog: {
                title: "Pagpi-print ng balota",
                content: "Ang pagpi-print ay hindi pinapayagan sa demo mode",
                ok: "OK",
            },
            demoBallotUrlDialog: {
                title: "ID ng Balota",
                content: "Hindi maaaring gamitin ang code, hindi pinapayagan sa demo mode.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Impormasyon: ID ng Balota",
                content:
                    "Ang ID ng Balota ay isang code na maari mong gamitin upang hanapin ang iyong balota sa ballot box, ang ID na ito ay natatangi at hindi naglalaman ng impormasyon tungkol sa iyong mga napili.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "Impormasyon: Ballot ID",
                content:
                    "<p>Ang Ballot ID ay isang code na maari mong gamitin upang hanapin ang iyong balota sa ballot box, ang ID na ito ay natatangi at hindi naglalaman ng impormasyon tungkol sa iyong mga napili.</p><p><b>Paalala:</b> Ang voting booth na ito ay para sa layuning demonstrasyon lamang. Ang iyong boto ay HINDI pa naipapasa.</p>",
                ok: "OK",
            },
            errorDialogPrintBallotReceipt: {
                title: "Error",
                content: "Nagkaroon ng error, pakisubukan muli",
                ok: "OK",
            },
            demoQRText: "Ang ballot tracker ay hindi gumagana sa demo mode",
        },
        auditScreen: {
            printButton: "I-print",
            restartButton: "Simulan ang Pagboto",
            title: "Suriin ang Iyong Balota",
            description:
                "Upang i-verify ang iyong balota, mangyaring sundin ang mga hakbang sa ibaba:",
            step1Title: "1. I-download o kopyahin ang sumusunod na impormasyon",
            step1Description:
                "Ang iyong <b>Ballot ID</b> na makikita sa taas ng screen at ang iyong naka-encrypt na balota sa ibaba:",
            step1HelpDialog: {
                title: "Kopyahin ang Naka-encrypt na Balota",
                content:
                    "Maaari mong i-download o kopyahin ang iyong naka-encrypt na balota upang suriin ang balota at tiyakin na ang naka-encrypt na nilalaman ay naglalaman ng iyong mga pinili.",
                ok: "OK",
            },
            downloadButton: "I-download",
            step2Title: "2. I-verify ang iyong balota",
            step2Description:
                "<VerifierLink>Access sa ballot verifier</VerifierLink>, isang bagong tab ang magbubukas sa iyong browser.",
            step2HelpDialog: {
                title: "Tutorial sa Pagsusuri ng Balota",
                content:
                    "Upang suriin ang iyong balota, kailangan mong sundin ang mga hakbang na ipinapakita sa tutorial. Kasama rito ang pag-download ng isang desktop application na ginagamit upang i-verify ang naka-encrypt na balota nang hiwalay mula sa website.",
                ok: "OK",
            },
            bottomWarning:
                "Para sa mga dahilan ng seguridad, kapag sinusuri mo ang iyong balota, kailangan itong ipawalang-bisa. Upang magpatuloy sa proseso ng pagboto, kailangan mong i-click ang ‘<b>Simulan ang Pagboto</b>’ sa ibaba.",
        },
        electionSelectionScreen: {
            title: "Listahan ng Balota",
            description: "Pumili ng Balota na nais mong botohan",
            chooserHelpDialog: {
                title: "Impormasyon: Listahan ng Balota",
                content:
                    "Maligayang pagdating sa Voting Booth, ipinapakita ng screen na ito ang listahan ng mga Balota na maaari mong botohan. Ang mga balota na nakalista dito ay maaaring nakabukas para sa pagboto, naka-schedule, o sarado. Magkakaroon ka lamang ng access sa balota kung nakabukas ang panahon ng pagboto.",
                ok: "OK",
            },
            noResults: "Walang mga balota sa ngayon.",
            demoDialog: {
                title: "Demo Voting Booth",
                content:
                    "Ikaw ay nasa isang demo voting booth. <strong>Ang iyong boto ay HINDI mai-susubmit.</strong> Ang voting booth na ito ay para lamang sa layunin ng demonstrasyon.",
                ok: "Tinatanggap ko na ang aking boto ay hindi mai-susubmit",
            },
            errors: {
                noVotingArea:
                    "Walang nakatalagang lugar ng halalan para sa botante. Mangyaring subukan muli mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                networkError:
                    "Nagkaroon ng problema sa network. Mangyaring subukan muli mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                unableToFetchData:
                    "Nagkaroon ng problema sa pagkuha ng data. Mangyaring subukan muli mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                noElectionEvent:
                    "Walang kaganapan ng halalan. Mangyaring subukan muli mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                ballotStylesEmlError:
                    "Nagkaroon ng error sa pag-publish ng estilo ng balota. Mangyaring subukan muli mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                obtainingElectionFromID:
                    "Nagkaroon ng error sa pagkuha ng mga halalan na nauugnay sa mga sumusunod na election IDs: {{electionIds}}. Mangyaring subukan muli mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
            },
            alerts: {
                noElections:
                    "Walang mga halalan na maaari mong botohan. Ito ay maaring dahil walang naka-ugnay na paligsahan sa lugar. Mangyaring subukan muli mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                electionEventNotPublished:
                    "Ang kaganapan ng halalan ay hindi pa nailathala. Mangyaring subukan muli mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
            },
        },
        errors: {
            encoding: {
                notEnoughChoices: "Hindi sapat ang mga pagpipilian para ma-decode",
                writeInChoiceOutOfRange: "Write-in na napili ay wala sa saklaw: {{index}}",
                writeInNotEndInZero: "Ang Write-in ay hindi nagtatapos sa 0",
                writeInCharsExceeded:
                    "Ang Write-in ay lumampas ng {{numCharsExceeded}} sa maximum na bilang ng mga karakter. Kailangang ayusin.",
                bytesToUtf8Conversion:
                    "Error sa pag-convert ng write-in mula bytes patungong UTF-8 na string: {{errorMessage}}",
                ballotTooLarge: "Ang balota ay mas malaki kaysa sa inaasahan",
            },
            implicit: {
                selectedMax:
                    "Overvote: Ang bilang ng mga napili {{numSelected}} ay higit sa maximum na {{max}}",
                selectedMin:
                    "Ang bilang ng mga napili {{numSelected}} ay mas mababa sa minimum na {{min}}",
                maxSelectionsPerType:
                    "Ang bilang ng mga napili {{numSelected}} para sa listahan {{type}} ay higit sa maximum na {{max}}",
                underVote:
                    "Undervote: Ang bilang ng mga napili {{numSelected}} ay mas mababa sa maximum na {{max}}",
                overVoteDisabled:
                    "Naabot na ang maximum: Napili mo na ang maximum na {{numSelected}} na mga opsyon. Upang baguhin ang iyong pagpili, mangyaring alisin muna ang isa pang opsyon.",
                blankVote: "Blank Vote: Walang pinili",
            },
            explicit: {
                notAllowed:
                    "Ang balota ay tahasang minarkahan upang mapawalang-bisa ngunit hindi ito pinapayagan ng tanong",
                alert: "An seleksyon na minarkahan ibibilang na bakong balidong boto.",
            },
            page: {
                oopsWithStatus: "Oops! {{status}}",
                oopsWithoutStatus: "Oops! Hindi inaasahang Error",
                somethingWrong: "May nangyaring hindi tama.",
            },
        },
        materials: {
            common: {
                label: "Mga Pangsuportang Materyales",
                back: "Bumalik sa Listahan ng mga Balota",
                close: "Isara",
                preview: "Silipin",
            },
        },
        ballotLocator: {
            title: "Hanapin ang Iyong Balota",
            titleResult: "Resulta ng Iyong Paghahanap ng Balota",
            description: "Patunayan na ang iyong balota ay tama na naipasa",
            locate: "Hanapin ang Iyong Balota",
            locateAgain: "Hanapin ang Isa Pang Balota",
            found: "Ang iyong ballot ID {{ballotId}} ay natagpuan",
            notFound: "Ang iyong ballot ID {{ballotId}} ay hindi natagpuan",
            contentDesc: "Ito ang nilalaman ng iyong balota: ",
            wrongFormatBallotId: "Mali ang format para sa Ballot ID",
            steps: {
                lookup: "Hanapin ang Iyong Balota",
                result: "Resulta",
            },
            titleHelpDialog: {
                title: "Impormasyon: Screen ng Locator ng Balota",
                content:
                    "Sa screen na ito maaring hanapin ng botante ang kaniyang boto gamit ang Ballot ID upang matagpuan ito. Sa ganitong pamamaraan, maaring suriin kung ang balota ay nai-submit nang tama at kung ang naitalang balota ay tumutugma sa encrypted na balota na kanilang ipinadala.",
                ok: "OK",
            },
        },
    },
}

export default tagalogTranslation
