// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const tagalogTranslation = {
    translations: {
        common: {
            goBack: "Bumalik",
        },
        breadcrumbSteps: {
            electionList: "Listahan ng Balota",
            ballot: "Balota",
            review: "Suriin",
            confirmation: "Kumpirmasyon",
            audit: "Audit"
        },
        votingScreen: {
            backButton: "Bumalik",
            reviewButton: "Susunod",
            clearButton: "Burahin ang pagpili",
            ballotHelpDialog: {
                title: "Impormasyon: Screen ng Balota",
                content:
                    "Ipinapakita ng screen na ito ang mga paligsahan na maaari mong pagboto. Maaari mong piliin ang iyong mga pagpipilian sa pamamagitan ng pag-activate ng checkbox sa kanang bahagi ng Kandidato/Sagot. Upang i-reset ang iyong mga pagpili, i-click ang “<b>Burahin ang pagpili</b>” na button, upang pumunta sa susunod na hakbang, i-click ang “<b>Susunod</b>” na button sa ibaba.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Di-wastong o blangko na boto",
                content:
                    "Ang ilan sa iyong mga sagot ay magreresulta sa pagkaka-invalidate o pagkakaroon ng blangko sa balota sa isa o higit pang mga tanong.",
                ok: "Bumalik at suriin",
                continue: "Ipagpatuloy",
                cancel: "Kanselahin",
            },
        },
        startScreen: {
            startButton: "Simulan ang Pagboto",
            instructionsTitle: "Mga Tagubilin",
            instructionsDescription: "Sundin ang mga hakbang na ito upang ihulog ang iyong balota:",
            step1Title: "1. Piliin ang iyong mga opsyon",
            step1Description:
                "Piliin ang iyong mga napiling kandidato at sagutin ang mga tanong sa Balota isa-isa habang lumalabas ang mga ito. Maaari mong i-edit ang iyong balota hanggang handa ka nang magpatuloy.",
            step2Title: "2. Suriin ang iyong balota",
            step2Description:
                "Kapag ikaw ay kontento na sa iyong mga pagpili, aming i-eencript ang iyong balota at ipapakita sa iyo ang huling pagsusuri ng iyong mga napili. Makakatanggap ka rin ng natatanging tracker ID para sa iyong balota.",
            step3Title: "3. Ihulog ang iyong balota",
            step3Description:
                "Ihulog ang iyong balota: Sa wakas, maaari mong ihulog ang iyong balota upang ito ay tamang maitala. Maaari mo ring piliing i-audit at tiyaking na ang iyong balota ay tama ang pagkakahulog at pagkaka-encrypt.",
        },
        reviewScreen: {
            title: "Suriin ang iyong balota",
            description:
                "Upang magbago ng iyong mga pagpili, i-click ang “<b>I-edit ang balota</b>” na button, upang kumpirmahin ang iyong mga pagpili, i-click ang “<b>Ihulog ang iyong balota</b>” na button sa ibaba, at upang i-audit ang iyong balota i-click ang “<b>I-audit ang balota</b>” na button sa ibaba. Pakitandaan na kapag naihulog mo na ang iyong balota, ikaw ay nakaboto na at hindi ka na bibigyan ng panibagong balota para sa Balota na ito.",
            descriptionNoAudit:
                "Upang magbago ng iyong mga pagpili, i-click ang “<b>I-edit ang balota</b>” na button, upang kumpirmahin ang iyong mga pagpili, i-click ang “<b>Ihulog ang iyong balota</b>” na button sa ibaba. Pakitandaan na kapag naihulog mo na ang iyong balota, ikaw ay nakaboto na at hindi ka na bibigyan ng panibagong balota para sa Balota na ito.",
            backButton: "I-edit ang balota",
            castBallotButton: "Ihulog ang iyong balota",
            auditButton: "I-audit ang balota",
            reviewScreenHelpDialog: {
                title: "Impormasyon: Screen ng Pagsusuri",
                content:
                    "Ipinapakita ng screen na ito na maaari mong suriin ang iyong mga pagpili bago ihulog ang iyong balota.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Hindi pa naihulog ang boto",
                content:
                    "<p>Ito ang iyong Ballot Tracker ID, ngunit <b>hindi pa naihulog ang iyong boto</b>. Kung susubukan mong subaybayan ang balota, hindi mo ito matatagpuan.</p><p>Ang dahilan kung bakit ipinapakita namin ang Ballot Tracker ID sa yugtong ito ay upang pahintulutan kang i-audit ang kawastuhan ng naka-encrypt na balota bago ito ihulog.</p>",
                ok: "Tinatanggap ko na HINDI pa naihulog ang aking boto",
                cancel: "Kanselahin",
            },
            auditBallotHelpDialog: {
                title: "Nais mo bang i-audit ang balota?",
                content:
                    "<p>Pakitandaan na ang pag-audit ng iyong balota ay magpapawalang-bisa dito, na kakailanganin mong magsimulang muli sa proseso ng pagboto. Ang proseso ng audit ay nagpapahintulot sa iyo na tiyakin na ang iyong balota ay tamang naka-encode, ngunit ito ay nangangailangan ng advanced na mga hakbang teknikal. Inirerekomenda naming magpatuloy ka lang kung ikaw ay kumpiyansa sa iyong kakayahan sa teknikal. Kung nais mo lamang ihulog ang iyong balota, i-click ang <u>Kanselahin</u> upang bumalik sa screen ng pagsusuri ng balota.</b></p>",
                ok: "Oo, nais kong ITAPON ang aking balota upang i-audit ito",
                cancel: "Kanselahin",
            },
            confirmCastVoteDialog: {
                title: "Sigurado ka bang nais mong ihulog ang iyong boto?",
                content: "Ang iyong boto ay hindi na mababago sa oras na ito ay makumpirma.",
                ok: "Oo, nais kong IHULOG ang aking boto",
                cancel: "Kanselahin",
            },
            error: {
                NETWORK_ERROR:
                    "Nagkaroon ng problema sa network. Pakisubukan ulit mamaya o makipag-ugnayan sa suporta para sa tulong.",
                UNABLE_TO_FETCH_DATA:
                    "Nagkaroon ng problema sa pagkuha ng data. Pakisubukan ulit mamaya o makipag-ugnayan sa suporta para sa tulong.",
                LOAD_ELECTION_EVENT: "Hindi ma-load ang kaganapan ng eleksyon. Pakisubukan ulit mamaya.",
                CAST_VOTE:
                    "Nagkaroon ng error sa graphQL habang ihinuhulog ang boto. Pakisubukan ulit mamaya o makipag-ugnayan sa suporta para sa tulong.",
                NO_BALLOT_SELECTION:
                    "Ang estado ng pagpili para sa eleksyon na ito ay hindi naroroon. Pakitiyak na tama ang iyong pagpili o makipag-ugnayan sa suporta.",
                NO_BALLOT_STYLE: "Ang estilo ng balota ay hindi magagamit. Pakikontak ang suporta.",
                NO_AUDITABLE_BALLOT: "Walang magagamit na audit na balota. Pakikontak ang suporta.",
                INCONSISTENT_HASH:
                    "Nagkaroon ng error kaugnay sa proseso ng pag-hash ng balota. BallotId: {{ballotId}} ay hindi tugma sa audit na Hash ng Balota: {{auditableBallotHash}}. Pakiulat ang isyung ito sa suporta.",
                ELECTION_EVENT_NOT_OPEN: "Ang kaganapan ng eleksyon ay sarado na. Pakikontak ang suporta.",
                PARSE_ERROR:
                    "Nagkaroon ng error sa pag-parse ng balota. Pakisubukan ulit mamaya o makipag-ugnayan sa suporta para sa tulong.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "Nagkaroon ng error sa deserialization ng audit na balota. Pakisubukan ulit mamaya o makipag-ugnayan sa suporta para sa tulong.",
                DESERIALIZE_HASHABLE_ERROR:
                    "Nagkaroon ng error sa deserialization ng hashable na balota. Pakisubukan ulit mamaya o makipag-ugnayan sa suporta para sa tulong.",
                CONVERT_ERROR:
                    "Nagkaroon ng error sa pag-convert ng balota. Pakisubukan ulit mamaya o makipag-ugnayan sa suporta para sa tulong.",
                SERIALIZE_ERROR:
                    "Nagkaroon ng error sa serialization ng balota. Pakisubukan ulit mamaya o makipag-ugnayan sa suporta para sa tulong.",
                UNKNOWN_ERROR:
                    "Nagkaroon ng error. Pakisubukan ulit mamaya o makipag-ugnayan sa suporta para sa tulong.",
            },
        },
        confirmationScreen: {
            title: "Ang iyong boto ay naihulog na",
            description:
                "Ang code ng kumpirmasyon sa ibaba ay nagpapatunay na <b>ang iyong balota ay matagumpay na naihulog</b>. Maaari mong gamitin ang code na ito upang tiyakin na ang iyong balota ay nabibilang.",
            ballotId: "ID ng Balota",
            printButton: "I-print",
            finishButton: "Tapos na",
            verifyCastTitle: "Tiyakin na ang iyong balota ay naihulog",
            verifyCastDescription: "Maaari mong tiyakin na ang iyong balota ay naihulog nang tama anumang oras gamit ang sumusunod na QR code:",
            confirmationHelpDialog: {
                title: "Impormasyon: Screen ng Kumpirmasyon",
                content: "Ipinapakita ng screen na ito na ang iyong boto ay matagumpay na naihulog. Ang impormasyong ibinigay sa pahinang ito ay nagpapahintulot sa iyo na tiyakin na ang balota ay nailagay sa ballot box, ang prosesong ito ay maaaring isagawa anumang oras sa panahon ng pagboto at pagkatapos na maisara ang balota.",
                ok: "OK"
            },
            demoPrintDialog: {
                title: "Pagpi-print ng balota",
                content: "Ang pagpi-print ay hindi pinapayagan sa demo mode",
                ok: "OK"
            },
            demoBallotUrlDialog: {
                title: "ID ng Balota",
                content: "Hindi maaaring gamitin ang code, hindi pinapayagan sa demo mode.",
                ok: "OK"
            },
            ballotIdHelpDialog: {
                title: "Impormasyon: ID ng Balota",
                content: "Ang ID ng Balota ay isang code na nagpapahintulot sa iyo na hanapin ang iyong balota sa ballot box, ang ID na ito ay natatangi at hindi naglalaman ng impormasyon tungkol sa iyong mga pinili.",
                ok: "OK"
            },
            ballotIdDemoHelpDialog: {
                title: "Impormasyon: Ballot ID",
                content: "<p>Ang Ballot ID ay isang code na nagpapahintulot sa iyo na hanapin ang iyong balota sa ballot box, ang ID na ito ay natatangi at hindi naglalaman ng impormasyon tungkol sa iyong mga pagpipilian.</p><p><b>Paalala:</b> Ang voting booth na ito ay para sa layuning demonstrasyon lamang. Ang iyong boto ay HINDI pa naipapasa.</p>",
                ok: "OK"
            },
            errorDialogPrintVoteReceipt: {
                title: "Error",
                content: "Nagkaroon ng error, pakisubukan muli",
                ok: "OK"
            },
            demoQRText: "Ang ballot tracker ay hindi pinapagana sa demo mode"
        },
        auditScreen: {
            printButton: "I-print",
            restartButton: "Magsimula ng Pagboto",
            title: "Suriin ang Iyong Balota",
            description: "Upang i-verify ang iyong balota, mangyaring sundin ang mga hakbang sa ibaba:",
            step1Title: "1. I-download o kopyahin ang sumusunod na impormasyon",
            step1Description:
                "Ang iyong <b>Ballot ID</b> na lumilitaw sa tuktok ng screen at ang iyong naka-encrypt na balota sa ibaba:",
            step1HelpDialog: {
                title: "Kopyahin ang Naka-encrypt na Balota",
                content:
                    "Maaari mong i-download o kopyahin ang iyong naka-encrypt na balota upang suriin ang balota at tiyakin na ang naka-encrypt na nilalaman ay naglalaman ng iyong mga pinili.",
                ok: "OK",
            },
            downloadButton: "I-download",
            step2Title: "2. I-verify ang iyong balota",
            step2Description:
                '<a class="link" href="{{linkToBallotVerifier}}" target="_blank">Access sa ballot verifier</a>, isang bagong tab ang magbubukas sa iyong browser.',
            step2HelpDialog: {
                title: "Tutorial sa Pagsusuri ng Balota",
                content:
                    "Upang suriin ang iyong balota, kailangan mong sundin ang mga hakbang na ipinapakita sa tutorial. Kasama rito ang pag-download ng isang desktop application na ginagamit upang i-verify ang naka-encrypt na balota nang hiwalay mula sa website.",
                ok: "OK",
            },
            bottomWarning:
                "Para sa mga dahilan ng seguridad, kapag sinusuri mo ang iyong balota, kailangan itong masira. Upang magpatuloy sa proseso ng pagboto, kailangan mong i-click ang ‘<b>Magsimula ng Pagboto</b>’ sa ibaba.",
        },
        electionSelectionScreen: {
            title: "Listahan ng Balota",
            description: "Pumili ng Balota na nais mong iboto",
            chooserHelpDialog: {
                title: "Impormasyon: Listahan ng Balota",
                content:
                    "Maligayang pagdating sa Voting Booth, ipinapakita ng screen na ito ang listahan ng mga Balota na maaari mong iboto. Ang mga balota na nakalista dito ay maaaring bukas para sa pagboto, naka-schedule, o sarado. Magkakaroon ka lamang ng access sa balota kung bukas ang panahon ng pagboto.",
                ok: "OK",
            },
            noResults: "Walang mga balota sa ngayon.",
            demoDialog: {
                title: "Demo Voting Booth",
                content:
                    "Pumasok ka sa isang demo voting booth. <strong>Ang iyong boto ay HINDI maibibigay.</strong> Ang voting booth na ito ay para lamang sa layunin ng demonstrasyon.",
                ok: "Tinatanggap ko na ang aking boto ay hindi maibibigay",
            },
            errors: {
                noVotingArea:
                    "Walang nakatalagang lugar ng halalan para sa botante. Mangyaring subukan muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                networkError:
                    "Nagkaroon ng problema sa network. Mangyaring subukan muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                unableToFetchData:
                    "Nagkaroon ng problema sa pagkuha ng data. Mangyaring subukan muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                noElectionEvent:
                    "Walang halalan na kaganapan. Mangyaring subukan muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                ballotStylesEmlError:
                    "Nagkaroon ng error sa pag-publish ng estilo ng balota. Mangyaring subukan muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                obtainingElectionFromID:
                    "Nagkaroon ng error sa pagkuha ng mga halalan na nauugnay sa mga sumusunod na election IDs: {{electionIds}}. Mangyaring subukan muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
            },
            alerts: {
                noElections:
                    "Walang mga halalan na maaari mong iboto. Maaaring dahil ito sa kakulangan ng contest sa lugar. Mangyaring subukan muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
                electionEventNotPublished:
                    "Ang kaganapan ng halalan ay hindi pa nailathala. Mangyaring subukan muli mamaya o makipag-ugnayan sa suporta para sa tulong.",
            }
        },
        errors: {
            encoding: {
                notEnoughChoices: "Hindi sapat ang mga pagpipilian para ma-decode",
                writeInChoiceOutOfRange: "Write-in na pagpipilian ay wala sa saklaw: {{index}}",
                writeInNotEndInZero: "Ang Write-in ay hindi nagtatapos sa 0",
                writeInCharsExceeded: "Ang Write-in ay lumampas ng {{numCharsExceeded}} sa maximum na bilang ng mga karakter. Nangangailangan ng pag-aayos.",
                bytesToUtf8Conversion: "Error sa pag-convert ng write-in mula bytes patungong UTF-8 na string: {{errorMessage}}",
                ballotTooLarge: "Ang balota ay mas malaki kaysa sa inaasahan",
            },
            implicit: {
                selectedMax: "Overvote: Ang bilang ng mga napiling pagpipilian {{numSelected}} ay higit sa maximum na {{max}}",
                selectedMin: "Ang bilang ng mga napiling pagpipilian {{numSelected}} ay mas mababa sa minimum na {{min}}",
                maxSelectionsPerType: "Ang bilang ng mga napiling pagpipilian {{numSelected}} para sa listahan {{type}} ay higit sa maximum na {{max}}",
                underVote: "Undervote: Ang bilang ng mga napiling pagpipilian {{numSelected}} ay mas mababa sa maximum na {{max}}",
                blankVote: "Blank Vote: 0 pagpipilian ang pinili",
            },
            explicit: {
                notAllowed: "Ang balota ay tahasang minarkahan bilang hindi wasto ngunit hindi ito pinapayagan ng tanong",
            },
            page: {
                oopsWithStatus: "Ay naku! {{status}}",
                oopsWithoutStatus: "Ay naku! Hindi inaasahang Error",
                somethingWrong: "May nangyaring hindi tama.",
            }
        },
        materials: {
            common: {
                label: "Mga Materyales sa Suporta",
                back: "Bumalik sa Listahan ng mga Balota",
                close: "Isara",
                preview: "prebiyu",
            },
        },
        ballotLocator: {
            title: "Hanapin ang Iyong Balota",
            titleResult: "Resulta ng Iyong Paghahanap ng Balota",
            description: "Patunayan na ang iyong balota ay tama na naipasa",
            locate: "Hanapin ang Iyong Balota",
            locateAgain: "Hanapin ang Isa Pang Balota",
            found: "Ang iyong balota ID {{ballotId}} ay natagpuan",
            notFound: "Ang iyong balota ID {{ballotId}} ay hindi natagpuan",
            contentDesc: "Ito ang nilalaman ng iyong balota: ",
            wrongFormatBallotId: "Mali ang format para sa Balota ID",
            steps: {
                lookup: "Hanapin ang Iyong Balota",
                result: "Resulta",
            },
            titleHelpDialog: {
                title: "Impormasyon: Screen ng Locator ng Balota",
                content:
                    "Ang screen na ito ay nagpapahintulot sa botante na hanapin ang kanilang boto gamit ang Ballot ID upang ma-retrieve ito. Ang pamamaraang ito ay nagpapahintulot na suriin kung tama ang pag-cast ng kanilang balota at kung ang naitalang balota ay tumutugma sa encrypted na balota na kanilang ipinadala.",
                ok: "OK",
            }
        },
    },
}

export type TranslationType = typeof tagalogTranslation

export default tagalogTranslation
