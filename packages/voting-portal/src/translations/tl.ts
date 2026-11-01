// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import englishTranslation, {TranslationType} from "./en"

const tagalogTranslation: TranslationType = {
    translations: {
        ballotPrivacy: {
            ...englishTranslation.translations.ballotPrivacy,
            title: "Lahat ng balota",
            subtitle:
                "Bawat balotang naipadala sa halalang ito, isang beses, sa huling hakbang nito. Nakalista ang mga botante sa ilalim ng alyas sa halip na pangalan.",
            own: "Nasa log ang iyong balota",
            copy: "Kopyahin ang Tracking ID",
            ballotIdHelp: "Tungkol sa iyong Tracking ID",
            stageTime: "{{stage}} {{time}}",
            noMatchHelp:
                "Tingnan kung may mali sa pag-type. Ang Tracking ID ay may gitling at hindi bababa sa 8 karakter, tulad ng FTBE-MHRX. Ang alyas ay may tutuldok, tulad ng K7QM:2XWD.",
            copied: "Nakopya ang Tracking ID",
            copyFailed: "Hindi makopya ang Tracking ID. Piliin ang code para makopya ito.",
            search: "Maghanap ng Tracking ID o alyas",
            placeholder: "FTBE-MHRX o K7QM:2XWD",
            matchesBoth: "{{tracking}} ang tugma ayon sa Tracking ID · {{pseudonym}} ayon sa alyas",
            ballotId: "Tracking ID",
            lastStep: "Huling hakbang",
            time: "Oras",
            trackingId: "Tracking ID",
            trackingIdHelp: "Kinukuwenta mula sa naka-encrypt na balota at sa susi ng botante.",
            noMatch: "Walang Tracking ID o alyas na nagsisimula sa {{prefix}}",
            noBallots: "Wala pang naipapadalang balota.",
            invalidPrefix:
                "Maglagay ng hindi bababa sa 8 karakter ng Tracking ID na may gitling, tulad ng FTBE-MHRX, o hanggang 8 ng alyas na may tutuldok, tulad ng K7QM:2XWD. Gumagamit ang mga code ng mga digit 0 hanggang 9 at malalaking titik, walang I, L, O at U.",
            stages: {
                received: {
                    label: "Natanggap",
                    title: "Natanggap ang balota",
                    help: "Pumirma ang balotang kahon ng resibo para sa balotang ito, at nangako ang pampublikong log na idagdag ito. Hindi ito binibilang maliban kung ihulog ito ng botante.",
                    castState: "Hindi pa naihuhulog",
                },
                cast: {
                    label: "Naihulog",
                    title: "Naihulog ang balota",
                    help: "Kinumpirma ng balotang kahon na inihulog ng botante ang balotang ito.",
                    castState: "Pumasa",
                },
                audited: {
                    label: "Na-audit",
                    title: "Na-audit ang balota",
                    help: "Binuksan ito ng botante para suriin. Ang na-audit na balota ay hindi kailanman inihuhulog o binibilang.",
                    castState: "Hindi kailanman naihulog",
                },
            },
            checkLabels: {
                ...englishTranslation.translations.ballotPrivacy.checkLabels,
                ballotId: "Tracking ID",
            },
            checkDescriptions: {
                ...englishTranslation.translations.ballotPrivacy.checkDescriptions,
                ballotId:
                    "Kinukuwenta ang Tracking ID mula sa naka-encrypt na balota at sa susi ng botante.",
            },
        },
        startup: {
            error: "Hindi masimulan ang portal ng pagboto. Pakisubukang muli.",
            retry: "Subukang muli",
        },
        contactChannel: {
            change: "Paraan ng pakikipag-ugnayan",
            updated: "Napalitan na ang iyong contact. Doon na ipapadala ang mga code at abiso.",
            cancelled: "Hindi napalitan ang iyong contact.",
            failed: "Hindi namin mabago kung paano ka namin kokontakin. Subukang muli mamaya o makipag-ugnayan sa support.",
            close: "Isara",
        },
        common: {
            goBack: "Bumalik",
            showMore: "Lakihan ang nakikita",
            showLess: "Bawasan ang nakikita",
        },
        audioInstructions: {
            screens: {
                "election-chooser":
                    "Ipinapakita ng pahinang ito ang mga halalang maaari mong botohan. Gamitin ang Tab key upang lumipat mula sa isang halalan papunta sa susunod. May button ang bawat halalan para simulan ang pagboto. Pindutin ang Enter sa button na iyon upang buksan ang balota nito.",
                "start":
                    "Ipinapaliwanag ng pahinang ito kung paano bumoto. Una, pipiliin mo ang iyong mga opsyon, pagkatapos ay susuriin mo ang mga ito, at saka mo ihuhulog ang iyong balota. Gamitin ang Tab key upang marating ang button na nagsisimula ng pagboto at pindutin ang Enter. Kung may ipinapakitang deklarasyon, lagyan muna ng tsek ang checkbox nito gamit ang Space bar.",
                "ballot":
                    "Ito ang iyong balota. Ang bawat tanong ay isang pangkat ng mga opsyon, at sinasabi ng tanong kung ilan ang maaari mong piliin. Gamitin ang Tab key upang lumipat sa mga opsyon at ang Space bar upang pumili o mag-alis ng pili. Kapag tapos ka na, pumunta sa button para magpatuloy, sa dulo ng pahina, at pindutin ang Enter upang suriin ang iyong mga pinili. May button din para burahin ang lahat ng iyong pinili. Walang maihuhulog hangga't hindi mo kinukumpirma sa pahina ng pagsusuri.",
                "review":
                    "Ipinapakita ng pahinang ito ang mga pinili mo. Suriin itong mabuti. Para may baguhin, gamitin ang button para i-edit ang iyong balota. Para bumoto, pumunta sa button para ihulog ang iyong balota at pindutin ang Enter. Nasa itaas ng pahina ang iyong Ballot ID. Maaari mo itong kopyahin upang mahanap ang iyong balota sa ibang pagkakataon.",
                "confirmation":
                    "Naihulog na ang iyong boto. Ipinapakita ng pahinang ito ang iyong Ballot ID, na magagamit mo upang tiyaking naitala ang iyong balota. Maaari mo itong kopyahin o i-print ang pahinang ito. Kapag tapos ka na, gamitin ang button sa dulo ng pahina upang tapusin.",
                "audit":
                    "Sa pahinang ito, maaari mong i-audit ang iyong balota sa halip na ihulog ito. Ipinapakita nito ang impormasyong masusuri mo gamit ang ballot verifier. Hindi naihuhulog ang balotang na-audit, kaya bumalik pagkatapos sa iyong balota upang bumoto.",
                "ballot-locator":
                    "Sa pahinang ito, matitiyak mong naitala ang isang balota. I-type o i-paste ang isang Ballot ID sa text field at pindutin ang Enter. Lalabas ang resulta sa ibaba ng field.",
                "support-materials":
                    "Ipinapakita ng pahinang ito ang mga dokumentong tutulong sa iyong bumoto. Gamitin ang Tab key upang lumipat sa mga ito at pindutin ang Enter sa button ng isang dokumento upang buksan ito. Gamitin ang button para bumalik sa listahan ng mga halalan.",
            },
        },
        a11y: {
            skipToContent: "Lumaktaw sa pangunahing nilalaman",
            helpAbout: "Tulong tungkol sa {{topic}}",
            previewMaterial: "I-preview ang {{title}}",
            ballotsTable: "Mga balota",
            ballotLocatorTabs: "Mga seksyon ng pagsubaybay sa balota",
            ballotIdLabel: "Tracking ID",
            votingProgress: "Progreso ng pagboto",
            stepOf: "Hakbang {{current}} ng {{total}}",
            selectUpTo_one: "Pumili ng hanggang {{count}} opsyon",
            selectUpTo_many: "Pumili ng hanggang {{count}} na opsyon",
            selectUpTo_other: "Pumili ng hanggang {{count}} na opsyon",
            selectExactly_one: "Pumili ng {{count}} opsyon",
            selectExactly_many: "Pumili ng {{count}} na opsyon",
            selectExactly_other: "Pumili ng {{count}} na opsyon",
            selectBetween: "Pumili sa pagitan ng {{min}} at {{max}} na opsyon",
        },
        candidatesList: {
            collapseToggle: "I-toggle ang listahan {{listTitle}}",
            showCandidates: "Ipakita ang mga kandidato",
            hideCandidates: "Itago ang mga kandidato",
            selectedCandidates_one: "{{count}} kandidatong napili",
            selectedCandidates_many: "{{count}} kandidatong napili",
            selectedCandidates_other: "{{count}} kandidatong napili",
            expandAll: "Palawakin lahat",
            collapseAll: "Bawasan lahat",
        },
        breadcrumbSteps: {
            electionList: "Mga Balota",
            ballot: "Balota",
            review: "Suriin",
            confirmation: "Kumpirmahin",
            audit: "Audit",
        },
        footer: {
            poweredBy: "Pinapagana ng <1></1>",
        },
        contest: {
            acclamation: {
                description:
                    "Ang paligsahang ito ay napagpasyahan sa pamamagitan ng aklamasyon. Ang mga kandidato nito ay nahalal nang walang botohan, kaya walang opsyong mapipili at walang botong naitatala.",
            },
        },
        votingScreen: {
            backButton: "Bumalik",
            reviewButton: "Susunod",
            clearButton: "Burahin ang mga napili",
            ballotHelpDialog: {
                title: "Tungkol sa screen ng balota",
                content:
                    "Ipinapakita sa screen na ito ang mga paligsahan na maaari mong botohan. I-activate ang checkbox sa kanan para pumili ng Kandidato/Sagot. I-reset sa “<b>Burahin ang mga napili</b>”, pumunta sa susunod sa “<b>Susunod</b>”.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Di-wasto o blangkong boto",
                content:
                    "Ang ilan sa iyong mga sagot ay magreresulta sa pagkaka-invalidate o pagkakaroon ng blangko sa balota sa isa o higit pang mga paligsahan.",
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
            blankBallotDialog: {
                title: "Wala kang napiling kandidato",
                content:
                    "Wala kang ginawang pagpili. Ang iyong balota ay isusumite bilang blangkong balota, na isang wasto at sinasadyang pagpili at ibibilang bilang ganoon.",
                continue: "Magpatuloy",
                cancel: "Kanselahin",
            },
        },
        startScreen: {
            startButton: "Simulan ang Pagboto",
            declineToVoteButton: "Tumangging bumoto",
            declineToVoteDialog: {
                title: "Kumpirmahin ang pagtangging bumoto",
                content:
                    "Sigurado ka bang nais mong tumangging bumoto?<br />Dadalhin ka diretso sa review at mase-save ang iyong status ng partisipasyon bilang <b>Tumangging bumoto</b>.",
                continue: "Tumangging bumoto",
                cancel: "Kanselahin",
            },
            instructionsTitle: "Paano bumoto",
            instructionsDescription: "Sundin ang mga hakbang na ito para bumoto:",
            step1Title: "1. Piliin ang iyong mga sagot",
            step1Description:
                "Pumili sa bawat paligsahan isa-isa. Maaari mong i-edit ang iyong balota hanggang handa ka.",
            step2Title: "2. Suriin ang iyong balota",
            step2Description:
                "Ie-encrypt namin ang iyong balota at ipapakita ang huling pagsusuri. Makakatanggap ka ng natatanging Tracking ID.",
            step3Title: "3. I-submit ang iyong balota",
            step3Description:
                "I-submit ang iyong balota upang ito ay tamang maitala, o suriin ito upang tiyakin na ito ay tama ang pagkaka-encrypt.",
        },
        reviewScreen: {
            reloaded: {
                title: "Hindi naihulog ang iyong balota",
                text: "Binura ng pag-reload ng page ang isahang-gamit na susi na naghuhulog nito. Suriin ang iyong mga pinili at ipadala muli ang iyong balota. Mananatili sa talaan ang ipinadala mong balota bilang natanggap at hindi kailanman bibilangin.",
                back: "Bumalik sa aking balota",
            },
            castRecovery: {
                uncertain:
                    "Hindi pa nakumpirma ang resulta ng pagboto. Maaari mong subukan muli ang parehong kahilingan o tingnan ang Tracking ID bago magsimulang muli.",
                checkBallot: "Tingnan ang balota",
            },
            acclamation: {
                title: "Napagpasyahan sa pamamagitan ng aklamasyon",
                helpDialog: {
                    title: "Impormasyon: Aklamasyon",
                    content:
                        "Ipinapakita sa screen na ito ang napagpasyahan sa pamamagitan ng aklamasyon. Dahil walang opsyong mapipili, walang balotang isusumite at walang maberipika pagkatapos.",
                    ok: "OK",
                },
                description:
                    "Suriin ang napagpasyahan sa pamamagitan ng aklamasyon sa halalang ito. Walang balotang isusumite.",
                finishButton: "Tapusin",
            },
            title: "Suriin ang iyong balota",
            description:
                "I-click ang “<b>I-edit ang balota</b>” para baguhin ang mga napili, “<b>I-submit ang balota</b>” para kumpirmahin, o “<b>Suriin ang balota</b>” para sa audit.",
            descriptionNoAudit:
                "I-click ang “<b>I-edit ang balota</b>” para baguhin ang mga napili, o “<b>I-submit ang balota</b>” para kumpirmahin.",
            backButton: "I-edit ang balota",
            castBallotButton: "I-submit ang balota",
            auditButton: "Suriin ang balota",
            copyBallotId: "Kopyahin ang Tracking ID",
            ballotIdCopied: "Nakopya ang Tracking ID",
            ballotIdCopyError: "Hindi makopya ang Tracking ID",
            reviewScreenHelpDialog: {
                title: "Tungkol sa screen ng pagsusuri",
                content:
                    "Maaari mo ditong suriin ang iyong mga napili bago i-submit ang iyong balota.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Hindi pa na-submit ang boto",
                content:
                    "<p>Ito ang iyong Tracking ID, ngunit <b>hindi pa nai-submit ang iyong boto</b>. Kung susubukan mong i-track ang balota, hindi mo ito mahahanap.</p><p>Ang dahilan kung bakit ipinapakita namin ang Tracking ID sa yugtong ito ay upang masuri mo ang pagiging tama ng naka-encrypt na balota bago ito i-submit.</p>",
                ok: "Naiintindihan ko na hindi pa nai-submit ang aking boto",
                cancel: "Kanselahin",
            },
            auditBallotHelpDialog: {
                title: "Nais mo bang i-audit ang balota?",
                content:
                    "<p>Pakitandaan na ang pag-audit ng iyong balota ay magpapawalang-bisa dito, at kakailanganin mong magsimulang muli sa proseso ng pagboto. Sa proseso ng audit, maari mong tiyakin na ang iyong balota ay tamang naka-encode, ngunit ito ay nangangailangan ng mga advanced na teknikal na hakbang. Inirerekomenda naming magpatuloy ka lang kung ikaw ay kumpiyansa sa iyong kakayahang teknikal. Kung nais mo lamang i-submit ang iyong balota, i-click ang <u>Kanselahin</u> upang bumalik sa screen ng pagsusuri ng balota.</p>",
                ok: "Oo, nais kong ibasura ang aking balota upang suriin ito",
                cancel: "Kanselahin",
            },
            confirmCastVoteDialog: {
                title: "Sigurado ka bang nais mong i-submit ang iyong boto?",
                content: "Pagkatapos mong kumpirmahin, ang iyong boto ay mai-susubmit.",
                ok: "Oo, nais kong i-submit ang aking boto",
                cancel: "Kanselahin",
            },
            confirmCastBlankBallotDialog: {
                title: "Sigurado ka bang nais mong magsumite ng blangkong balota?",
                content:
                    "Wala kang napiling kandidato. Pagkatapos mong kumpirmahin, ang iyong balota ay isusumite bilang blangko.",
                ok: "Oo, nais kong isumite ang aking blangkong balota",
                cancel: "Kanselahin",
            },
            unfilledContestsDialog: {
                title: "May mga pagpiling hindi napunan",
                content:
                    "Maaari mong iwanang blangko ang isang pagpili o pumili ng mas kaunting opsyon kaysa sa pinapayagan. Suriin ang sumusunod bago magpatuloy.",
                selected: "{{selected}} sa {{max}} ang napili",
                nothingSelected: "Walang napili",
                ok: "Magpatuloy sa mga pagpiling ito",
                cancel: "Suriin ang mga pinili",
            },
            error: {
                NETWORK_ERROR:
                    "Nagkaroon ng problema sa network. Pakisubukan ulit mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                UNABLE_TO_FETCH_DATA:
                    "Nagkaroon ng problema sa pagkuha ng data. Pakisubukan ulit mamaya o makipag-ugnayan sa helpdesk para sa tulong.",
                LOAD_ELECTION_EVENT:
                    "Hindi ma-load ang kaganapan ng halalan. Pakisubukan ulit mamaya.",
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
                    "Walang estado ng pagpili para sa halalan na ito. Pakitiyak na tama ang iyong pagpili o makipag-ugnayan sa helpdesk.",
                NO_BALLOT_STYLE: "Walang estilo ng balota. Pakikontak ang helpdesk.",
                NO_AUDITABLE_BALLOT: "Walang balota na maaring suriin. Pakikontak ang helpdesk.",
                INCONSISTENT_HASH:
                    "Nagkaroon ng error kaugnay sa proseso ng pag-hash ng balota. Ang Tracking ID: {{ballotId}} ay hindi tugma sa sinusuring Hash ng Balota: {{auditableBallotHash}}. Pakireport itong isyu sa helpdesk.",
                NOT_RECEIVED:
                    "Wala pa ang iyong balota sa pampublikong talaan, kaya hindi ito naipadala. Bumalik sa iyong balota at subukan muli.",
                ELECTION_EVENT_NOT_OPEN:
                    "Ang kaganapan ng halalan ay sarado na. Pakikontak ang helpdesk.",
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
                CAST_VOTE_BallotIdMismatch: "Hindi tumutugma ang Tracking ID sa ibinotong boto.",
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
            declineToVote: "Tumangging bumoto",
            blankBallot: "Blangkong balota",
        },
        confirmationScreen: {
            acclamation: {
                title: "Napagpasyahan sa pamamagitan ng aklamasyon",
                description:
                    "Lahat ng paligsahan sa halalang ito ay napagpasyahan sa pamamagitan ng aklamasyon, kaya walang balotang isinumite at walang Tracking ID.",
                helpDialog: {
                    title: "Impormasyon: Aklamasyon",
                    content:
                        "Lahat ng paligsahan sa halalang ito ay napagpasyahan sa pamamagitan ng aklamasyon: ang mga kandidato nito ay nahalal nang walang botohan. Dahil walang balotang isinumite, walang Tracking ID, resibo, o QR code na maaaring i-verify.",
                    ok: "OK",
                },
            },
            title: "Ang iyong boto ay nai-submit na",
            description:
                "Ang code ng kumpirmasyon sa ibaba ay nagpapatunay na <b>ang iyong balota ay matagumpay na nai-submit</b>. Maaari mong gamitin ang code na ito upang tiyakin na ang iyong balota ay nabilang.",
            blankBallot: {
                description:
                    "Ang iyong balota ay naisumite bilang blangko, na isang wasto at sinasadyang pagpili.",
            },
            ballotId: "Tracking ID",
            printButton: "I-print",
            finishButton: "Tapos na",
            remainingElectionsError:
                "Hindi namin masuri kung mayroon ka pang ibang eleksiyong maaaring botohan. Pakisubukang muli.",
            retryButton: "Subukang muli",
            verifyCastTitle: "Tiyakin na ang iyong balota ay nai-submit",
            verifyCastDescription:
                "Maaari mong tiyakin na ang iyong balota ay nai-submit nang tama anumang oras gamit ang sumusunod na QR code:",
            confirmationHelpDialog: {
                title: "Tungkol sa screen ng kumpirmasyon",
                content:
                    "Ipinapakita ng screen na ito na ang iyong boto ay matagumpay na nai-submit. Maaari mong tiyakin na ang balota ay nai-submit sa ballot box.",
                ok: "OK",
            },
            demoPrintDialog: {
                title: "Pagpi-print ng balota",
                content: "Ang pagpi-print ay hindi pinapayagan sa demo mode",
                ok: "OK",
            },
            demoBallotUrlDialog: {
                title: "Tracking ID",
                content: "Hindi maaaring gamitin ang code, hindi pinapayagan sa demo mode.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Tungkol sa Tracking ID",
                content:
                    "Ang Tracking ID ay isang code na maari mong gamitin upang hanapin ang iyong balota sa ballot box, ang ID na ito ay natatangi at hindi naglalaman ng impormasyon tungkol sa iyong mga napili.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "Tungkol sa Tracking ID",
                content:
                    "Ang Tracking ID ay isang code na maari mong gamitin upang hanapin ang iyong balota sa ballot box, ang ID na ito ay natatangi at hindi naglalaman ng impormasyon tungkol sa iyong mga napili.",
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
            confirmingAudit: "Kinukumpirmang hindi na maaaring isumite ang balotang ito…",
            auditUnavailable:
                "Hindi nasuri ang balota. Hindi pa ipinapakita ang nilalaman nito. Pakisubukang muli.",
            printButton: "I-print",
            restartButton: "Simulan ang Pagboto",
            title: "Suriin ang Iyong Balota",
            description: "Upang suriin ang iyong balota, sundin ang mga hakbang sa ibaba:",
            step1Title: "1. I-download o kopyahin ang sumusunod na impormasyon",
            step1Description:
                "Ang iyong <b>Tracking ID</b> na makikita sa taas ng screen at ang iyong naka-encrypt na balota sa ibaba:",
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
            title: "Mga Balota",
            description: "Pumili ng Balota na nais mong botohan",
            chooserHelpDialog: {
                title: "Tungkol sa listahan ng balota",
                content:
                    "Ipinapakita ng screen na ito ang listahan ng mga Balota na maaari mong botohan. Magkakaroon ka ng access kung nakabukas ang panahon ng pagboto.",
                ok: "OK",
            },
            noResults: "Walang mga balota sa ngayon.",
            resultsButton: "Tingnan ang mga resulta",
            demoDialog: {
                title: "Demo Voting Booth",
                content:
                    "Ikaw ay nasa isang demo voting booth. <strong>Ang iyong boto ay hindi mai-susubmit.</strong> Para lamang sa layunin ng demonstrasyon.",
                ok: "Naiintindihan ko na ang aking boto ay hindi mai-susubmit",
            },
            errors: {
                noVotingArea:
                    "Hindi ka nakalista bilang botante sa halalan na ito. Mangyaring makipag-ugnayan sa suporta.",
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
            materialsGate: {
                instructions:
                    "Kailangan mong basahin ang <MaterialsLink>{{materialsTitle}}</MaterialsLink> bago ka makaboto.",
            },
            votingClosedAt: "Nagsara ang botohan noong {{close}} ({{localClose}}).",
            votingClosedOn: "Nagsara ang botohan noong {{close}}.",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Hindi sapat ang mga pagpili para ma-decode",
                writeInChoiceOutOfRange: "Ang write-in na pagpili ay wala sa saklaw: {{index}}",
                writeInNotEndInZero: "Ang write-in ay hindi nagtatapos sa 0",
                bytesToUtf8Conversion:
                    "Error sa pag-convert ng write-in mula sa bytes patungong UTF-8 na string: {{errorMessage}}",
                ballotTooLarge: "Ang balota ay mas malaki kaysa sa inaasahan",
            },
            explicit: {
                notAllowed:
                    "Ang balota ay tahasang minarkahang hindi wasto ngunit hindi ito pinapayagan ng paligsahan",
                alert: "Ang pagpiling ito ay biblangin bilang isang hindi wastong boto",
            },
            page: {
                oopsWithStatus: "Oops! {{status}}",
                oopsWithoutStatus: "Oops! Hindi inaasahang Error",
                somethingWrong: "May nangyaring hindi tama.",
                invalidLoginHintParametersTitle: "Hindi wastong link sa pagboto",
                invalidLoginHintParametersMessage:
                    "Naglalaman ang link na ito ng hindi wastong impormasyon sa pag-login. Humiling ng bagong link at subukan muli.",
                certAuthFailedTitle: "Nabigo ang Pagpapatunay ng Sertipiko",
                certAuthFailedMessage:
                    "Hindi ma-verify ang iyong sertipiko. Pakitiyak na gumagamit ka ng wastong sertipiko ng botante at subukang muli.",
            },
        },
        materials: {
            common: {
                label: "Mga Pangsuportang Materyales",
                back: "Bumalik sa listahan ng mga balota",
                close: "Isara",
                preview: "Silipin",
                download: "I-download",
            },
            mandatory: {
                checkboxLabel: "Nabasa ko na ang Mga Pangsuportang Materyales",
                continueButton: "Magpatuloy",
                error: "Nagkaroon ng problema sa pagre-record ng iyong kumpirmasyon. Pakisubukan muli.",
            },
        },
        slates: {
            title: "Mga slate",
            description:
                "Ang slate ay isang pangkat ng mga kandidatong magkakasamang tumatakbo. Ipinapakita ng bawat kandidato ang slate na kinabibilangan nila.",
            independent: "Independiyente",
            contestMembers: "Mga kandidato ng {{slate}} para sa {{contest}}",
            noCandidate: "Walang kandidato",
            coverage: {
                full: "Buong slate",
                singleContest: "{{contest}} lamang",
                partial: "Bahagyang slate",
                candidates_one: "{{count}} kandidato",
                candidates_other: "{{count}} kandidato",
                offices_one: "{{count}} posisyon",
                offices_other: "{{count}} posisyon",
            },
            candidateList: {
                show: "Ipakita ang mga kandidato",
                hide: "Itago ang mga kandidato",
            },
            tabs: {
                label: "Mga paraan ng pagsagot sa iyong balota",
                slates: "Pumili ng slate",
                candidates: "Mga indibidwal na kandidato",
            },
            review: {
                title: "Ang iyong mga pinili",
                total: "Mga kandidatong napili: {{selected}} sa {{seats}}",
                slate: "{{slate}}: {{status}}",
                independent: "Mga independiyenteng kandidatong napili: {{count}}",
                note: "Itinatala ang iyong boto para sa bawat napiling kandidato. Ang slate ay hindi hiwalay na boto.",
                contestCount: "{{selected}} sa {{max}} ang napili",
                edit: "Baguhin",
                editLabel: "Baguhin ang {{contest}}",
            },
            selection: {
                all: "Napili ang lahat ng {{total}}",
                mixed: "Halo · {{selected}} sa {{total}} ang napili",
                partly: "Bahagyang napili · {{selected}} sa {{total}}",
                selected: "Napili",
                edit: "Baguhin ang mga pinili",
            },
            apply: {
                button: "Piliin ang slate na ito",
                buttonLabel: "Piliin ang slate na {{slate}}",
                chosen: "Napili ang {{slate}}. Mga napiling kandidato: {{candidates}}. Mga paligsahan: {{contests}}.",
                overMaximum:
                    "Hindi mapipili ang {{slate}}: mayroon itong {{candidates}} kandidato para sa {{contest}}, na {{max}} lamang ang pinapayagan. Maaari ka pa ring pumili ng mga kandidato nang isa-isa.",
                unavailable:
                    "Hindi mapipili ang {{slate}} sa balotang ito. Maaari ka pa ring pumili ng mga kandidato nang isa-isa.",
                replaceDialog: {
                    title: "Palitan ang iyong kasalukuyang mga pinili?",
                    content:
                        "Kapag pinili ang {{slate}}, mapapalitan ang iyong mga pinili sa mga paligsahan sa ibaba. Hindi magbabago ang iba mo pang pinili.",
                    removed: "Aalisin:",
                    added: "Pipiliin bilang kapalit:",
                    ok: "Palitan ang mga pinili",
                    cancel: "Panatilihin ang aking mga pinili",
                },
            },
        },
        ballotLocator: {
            title: "Pagsubaybay sa Balota",
            titleResult: "Pagsubaybay sa Balota",
            description: "Patunayan na ang iyong balota ay tama na naipasa",
            descriptionAudited:
                "Na-audit ang balotang ito, kaya hindi ito kailanman naipasa. Tiyaking ang mga napiling na-decode sa ibaba ay ang mga pinili mo.",
            descriptionAuditedBrief: "Na-audit ang balotang ito, kaya hindi ito kailanman naipasa.",
            locate: "Hanapin ang Iyong Balota",
            locateAgain: "Hanapin ang Isa Pang Balota",
            found: "Ang iyong Tracking ID {{ballotId}} ay natagpuan",
            notFound: "Ang iyong Tracking ID {{ballotId}} ay hindi natagpuan",
            contentDesc: "Ito ang nilalaman ng iyong balota: ",
            wrongFormatBallotId: "Mali ang format para sa Tracking ID",
            ambiguous: "Maraming balota mo ang nagsisimula sa {{ballotId}}. Pumili ng isa:",
            ambiguousMany:
                "Maraming balota mo ang nagsisimula sa {{ballotId}}. I-type ang mas marami pa sa Tracking ID.",
            ballotIdNotFoundAtFilter:
                "Hindi natagpuan. Tiyaking tama ang iyong Tracking ID at nabibilang ito sa gumagamit na ito.",
            filterByBallotId: "I-filter ayon sa Tracking ID",
            totalBallots: "Kumulang mga balota: {{total}}",
            idHelp: "Ang maikling Tracking ID, tulad ng FTBE-MHRX, o ang buo",
            foundShort:
                "Natagpuan sa pamamagitan ng maikling ID nito. Para matiyak na iyo ito, ihambing ang buong Tracking ID sa iyong resibo.",
            search: {
                title: "Suriin ang iyong balota sa pampublikong talaan",
                intro: "Ilagay ang Tracking ID mula sa iyong screen ng kumpirmasyon o resibo. Hinahanap ito ng iyong browser at ang browser mismo ang sumusuri sa mga patunay.",
                find: "Hanapin",
                invalid: "Ang Tracking ID ay may mga digit at mga letrang a hanggang f lamang.",
                chips: {
                    short: "<b>Maikli</b> FTBE-MHRX",
                    full: "<b>Buo</b> 16 na grupo ng apat",
                    spaces: "Opsyonal ang mga espasyo",
                },
                features: {
                    cast: {
                        title: "Mga naipasang balota",
                        text: "Tingnan na nasa pampublikong talaan ito, kinumpirma ng mga malayang saksi at naka-anchor sa publiko.",
                    },
                    audited: {
                        title: "Mga na-audit na balota",
                        text: "Tingnan ang mga napili rito, na-decrypt sa iyong browser, at tiyaking ito ang mga pinili mo.",
                    },
                    secret: {
                        title: "Nananatiling lihim ang iyong boto",
                        text: "Nananatiling naka-encrypt ang naipasang balota. Ipinapakita ng tracker na nabilang ito, hindi kailanman kung paano ka bumoto.",
                    },
                },
            },
            matches: {
                title: "{{total}} balota ang may parehong maikling ID na <b>{{shortId}}</b>. Piliin ang iyo, o ilagay ang buong Tracking ID.",
                cast: "Naipasa noong {{time}}",
                audited: "Na-audit noong {{time}}",
            },
            audit: {
                kicker: "Audit ng balota",
                title: "Tiyaking na-encrypt ang iyong balota ayon sa pinili mo",
                note: "Ito ang balotang na-audit mo, na-decode ng iyong browser. Ikaw lang ang nakaaalam ng pinili mo, kaya ikaw lang ang makagagawa ng pagsusuring ito.",
                steps: {
                    label: "Paano gumagana ang pagsusuri",
                    encrypted: {
                        title: "Na-encrypt sa iyong device",
                        text: "Ini-encrypt ng portal ang iyong mga pinili at binigyan ang balota ng Tracking ID nito, bago mo piniling i-audit ito.",
                    },
                    decoded: {
                        title: "Na-decode rito",
                        text: "Sa pag-audit, ipinahayag ng portal ang lihim nito. Na-decode ng iyong browser ang balota gamit iyon at inihambing ito sa pampublikong talaan.",
                    },
                    compare: {
                        title: "Ikaw ang naghahambing",
                        text: "Tingnan ang mga pinili sa ibaba. Ito ba ang mga pinili mo?",
                    },
                },
                choices: "Mga pinili na na-decode mula sa balotang ito",
                question: "Tugma ba ang mga ito sa pinili mo?",
                yes: "Oo, tugma",
                no: "Hindi, may kakaiba",
                change: "Baguhin ang sagot ko",
                match: {
                    title: "Na-encrypt ang iyong balota ayon sa pinili mo",
                    text: "Nagamit na ang balotang na-audit na ito: hindi ito kailanman naiboto at hindi kailanman bibilangin. Bumalik at bumoto. May bagong Tracking ID ang iyong boto, at puwede kang mag-audit muli bago iyon kung gusto mo.",
                    vote: "Bumalik at bumoto",
                },
                mismatch: {
                    title: "Huwag bumoto sa device na ito",
                    text: "Hindi ang pinili mo ang na-encrypt, kaya maaaring may nakialam sa device na ito o sa portal ng pagboto. Itinago ng iyong browser ang ebidensiya: ang naka-encrypt na balota mula sa pampublikong talaan, ang lihim na inihayag ng portal, at ang resulta ng pag-decode nito.",
                    report: "I-ulat ito sa awtoridad ng halalan",
                    reportNoContact: "I-ulat ito sa awtoridad ng halalan.",
                    fromPolicy:
                        "Mula sa nailathalang patakaran ng halalan, hindi mula sa pahinang ito.",
                    fromSettings: "Mula sa mga setting ng halalan.",
                    then: "Pagkatapos, bumoto mula sa ibang device.",
                    save: "I-save ang ebidensiya",
                    email: "I-email ang awtoridad",
                    emailSubject: "Hindi nagde-decode sa mga pinili ko ang na-audit kong balota",
                    emailBody:
                        "Nag-audit ako ng balota sa halalang ito at hindi ang mga pinili ko ang na-decode ng aking browser.\n\nTracking ID: {{id}}\n\nNai-save ko ang ebidensiya sa isang file at maipapadala ko ito.",
                },
                how: "Paano ito gumagana?",
                faq: {
                    why: {
                        question: "Bakit mag-audit?",
                        answer: "Naka-encrypt sa iyong device ang boto mo, kaya walang makababasa nito, kahit ikaw. Pinapayagan ka ng pag-audit na tiyaking ini-encrypt ng portal ang pinili mo bago ka bumoto nang totoo.",
                    },
                    catch: {
                        question: "Bakit nahuhuli nito ang pandaraya?",
                        answer: "Nag-e-encrypt ang portal bago mo piliing mag-audit, kaya hindi nito alam kung aling mga balota ang susuriin. Kung binago nito ang mga boto, mahuhuli ito ng bawat botanteng nag-audit.",
                    },
                    spent: {
                        question: "Bakit hindi mabibilang ang balotang ito?",
                        answer: "Kapag naihayag na ang lihim nito, ipinapakita ng balota kung paano ka bumoto. Para manatiling lihim ang boto mo, ang balotang na-audit ay hindi kailanman iboboto.",
                    },
                    differ: {
                        question: "Paano kung hindi tugma?",
                        answer: "Huwag bumoto sa device na iyon. I-save ang ebidensiya at i-ulat ito sa awtoridad ng halalan. Pagkatapos, bumoto mula sa ibang device.",
                    },
                    tip: "Para sa pinakamatibay na pagsusuri, buksan ang pahinang ito sa device na iba sa ginamit mong pagboto, tulad ng iyong telepono. Ang device na nagbago ng iyong balota ay maaari ring magbago ng ipinapakita nito sa iyo rito.",
                },
                failed: {
                    title: "Nabigo ang audit",
                    text: "Hindi ma-decrypt ang balota sa pampublikong talaan gamit ang susing inihayag ng portal. Huwag bumoto gamit ang device na ito; ipaalam sa awtoridad ng halalan.",
                },
                unchecked: {
                    title: "Hindi masuri ang audit",
                    text: "Hindi nasuri ng device na ito ang na-audit na balotang ito, kaya walang ipinapakitang napili. Subukan muli mamaya, o suriin ang nilalaman ng balota sa ballot verifier.",
                },
            },
            result: {
                receipt: "Resibo ng balota",
                idComment:
                    "Gamitin ang maikling ID o ang buo para muling mahanap ang balotang ito.",
                fullId: "Buong Tracking ID",
                copy: "Kopyahin ang buong Tracking ID",
                copied: "Nakopya ang Tracking ID",
                tag: {
                    found: "Natagpuan",
                    auditFailed: "Nabigo ang audit",
                },
                record: {
                    title: "Nilalaman ng iyong balota",
                    subtitle: "Naka-encrypt, gaya ng nasa pampublikong talaan",
                },
                recordEntry: "Entry sa talaan",
                save: "I-save ang resibo",
                sealLabel: "Katayuan ng balota: {{status}}",
                check: "Suriin ito nang hiwalay",
                caption: "Sinuri ng iyong browser, gamit ang code na maaaring suriin ng sinuman.",
                captionVerified:
                    "Napatunayan ng iyong browser, gamit ang code na maaaring suriin ng sinuman.",
                captionProblem:
                    "Natagpuan ng iyong browser, gamit ang code na maaaring suriin ng sinuman.",
                late: "Karaniwang nangangahulugan ito na mabagal o offline ang isang hub; hindi ito patunay ng pakikialam. Suriin muli mamaya, dito o sa ibang device.",
                problem:
                    "Inililista ng portal ang balotang ito bilang naipasa, ngunit walang entry para rito ang pampublikong talaan ng halalan. Itinabi ng iyong browser ang ebidensya: ang sagot ng portal at ang nilagdaang checkpoint ng talaan. Ipadala ito sa awtoridad ng halalan o sa isang tagamasid.",
                problemOther:
                    "Magkasalungat ang mga nilagdaang patunay para sa balotang ito. Maaaring pagkakamali o pag-atake ito. Itinabi ng iyong browser ang ebidensya. Ipadala ito sa awtoridad ng halalan o sa isang tagamasid.",
                saveEvidence: "I-save ang ebidensya",
                howToReport: "Paano ito iuulat",
                howToReportText:
                    "I-save ang file ng ebidensya at ipadala ito sa awtoridad ng halalan o sa isang malayang tagamasid. Masusuri nila ito sa sarili nilang mga computer. Wala itong laman tungkol sa kung paano ka bumoto.",
                noReceipt:
                    "Walang resibo ang device na ito para sa balotang ito. Suriin ito sa device na ginamit mo sa pagboto, o suriin ang file ng resibo sa ballot verifier.",
                receiptPending:
                    "May itinatabing resibo ang device na ito para sa balotang ito, ngunit hindi ito kumpleto o hindi ito nasuri ngayon. Subukan muli mamaya, o suriin ang file ng resibo sa ballot verifier.",
                status: {
                    checking: {
                        head: "Sinusuri…",
                        sub: "Hinahanap ng iyong browser ang balota sa pampublikong talaan.",
                    },
                    recorded: {
                        head: "Nasa pampublikong talaan",
                        sub: "Natagpuan ng iyong browser ang balota. Sinusuri ang mga saksi nito…",
                    },
                    witnessed: {
                        head: "Kinumpirma ng mga saksi",
                        sub: "Sinusuri ang mga pampublikong anchor…",
                        subAnchored_one:
                            "Naka-anchor sa mga pampublikong hub. Susunod ang kung saan sumusuri ang mga browser sa loob ng {{count}} minuto.",
                        subAnchored_other:
                            "Naka-anchor sa mga pampublikong hub. Susunod ang kung saan sumusuri ang mga browser sa loob ng {{count}} minuto.",
                    },
                    verified: {
                        head: "Napatunayan",
                        sub: "Nasa pampublikong talaan, kinumpirma ng mga malayang saksi at naka-anchor sa publiko.",
                    },
                    late: {
                        head: "Mas matagal kaysa inaasahan",
                        sub: "Nasa pampublikong talaan at kinumpirma ng mga saksi, ngunit huli ang mga pampublikong anchor nito.",
                    },
                    problem: {
                        head: "Wala sa pampublikong talaan",
                        sub: "Hindi nahanap ng iyong browser ang balotang ito sa pampublikong talaan ng halalan.",
                    },
                    otherProblem: {
                        head: "Nabigo ang isang pagsusuri",
                        sub: "Magkasalungat ang mga nilagdaang patunay para sa balotang ito. Tingnan ang hakbang na nabigo.",
                    },
                },
                steps: {
                    record: {
                        title: "Naidagdag sa pampublikong talaan",
                        done: "Entry #{{index}} sa talaan ng mga balota ng halalan. Sinuri ng iyong browser ang patunay nito.",
                        failed: "Walang entry sa talaan ng mga balota ng halalan na may ganitong Tracking ID.",
                        missing: "Nawawala",
                    },
                    witnesses: {
                        title: "Kinumpirma ng mga saksi",
                        done: "Lumagda ang mga malayang saksi sa talaang kasama ito.",
                        count: "{{signed}} sa {{total}} · {{needed}} ang kailangan",
                        countOf: "{{signed}} sa {{total}}",
                    },
                    hubs: {
                        title_one: "Naka-anchor sa {{count}} pampublikong hub",
                        title_other: "Naka-anchor sa {{count}} pampublikong hub",
                        titleAny: "Naka-anchor sa mga pampublikong hub",
                        done: "May hawak din ng talaang ito ang mga nakabahaging talaan: {{hubs}}.",
                    },
                    browser: {
                        title: "Naka-anchor kung saan sumusuri ang mga browser",
                        done: "Hawak ng isang pansamantalang hub hanggang ang mga browser na mismo ang sumuri ng mga talaan.",
                    },
                    checking: "Sinusuri ito ng iyong browser…",
                    scheduled_one: "Sa loob ng {{count}} minuto. Hindi kailangang maghintay.",
                    scheduled_other: "Sa loob ng {{count}} minuto. Hindi kailangang maghintay.",
                    by: "Hanggang {{time}}",
                    late: "Huli",
                    lateHubs: "Dapat itong na-anchor ng mga hub bago ang {{time}}.",
                    lateOther: "Dapat natapos ito bago ang {{time}}.",
                    waiting: "Naghihintay sa hakbang sa itaas.",
                    skipped: "Hindi sinuri.",
                    failedStep: "Magkasalungat ang mga nilagdaang patunay para sa hakbang na ito.",
                    failedTime: "Problema",
                },
            },
            checksAvailableUntil: "Maaari mong suriin ang iyong balota hanggang {{date}}.",
            checksEnded: "Natapos ang mga pagsusuri noong {{date}}.",
            castAt: "Naihulog noong {{date}}",
            steps: {
                lookup: "Hanapin ang Iyong Balota",
                result: "Resulta",
            },
            titleHelpDialog: {
                title: "Tungkol sa Pagsubaybay sa Balota",
                content:
                    "Ang Pagsubaybay sa Balota ay nagbibigay-daan sa iyo na ilagay ang Tracking ID upang hanapin ang iyong boto at tiyakin na ito ay tamang nairehistro.",
                ok: "OK",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Pagsubaybay sa Balota",
            },
            column: {
                statement_kind: "Uri",
                statement_timestamp: "Timestamp",
                ballot_id: "Tracking ID",
            },
        },
        vitrine: {
            ballotId: {
                label: "Tracking ID",
                full: "Buong Tracking ID",
                yours: "Ang Iyong Tracking ID:",
                copy: "Kopyahin ang buong Tracking ID",
                copied: "Nakopya ang Tracking ID",
                copyText: "Kopyahin",
                copiedText: "Nakopya",
                showFull: "Ipakita ang buong Tracking ID",
                selected: "Napili na. Pindutin ang Ctrl+C, o ⌘C sa Mac, para kopyahin ito.",
                copyByHand:
                    "Hindi ito nakopya ng iyong browser. Buksan ang buong Tracking ID para mapili ito.",
            },
            elsewhere: {
                title: "Hanapin sa iba",
                count_one: "{{count}} explorer, {{independent}} malaya",
                count_other: "{{count}} explorer, {{independent}} malaya",
                introBallot:
                    "Binabasa mismo ng mga explorer na pinapatakbo ng iba ang pampublikong talaan. Dapat mahanap ng bawat isa ang iyong balota, na may parehong mga pagsusuri, nang hindi dumadaan sa website na ito.",
                introEntry:
                    "Binabasa mismo ng mga explorer na pinapatakbo ng iba ang pampublikong talaan. Dapat ipakita ng bawat isa ang entry na ito gaya ng nasa dito.",
                foot: "Galing sa patakaran ng halalan ang listahan. Mabubuksan ng anumang Vitrine explorer ang parehong talaan: ibigay ang buong Tracking ID.",
                name: "Explorer ng {{name}}",
                operatorName: "Explorer ng halalan",
                independent: "Malaya",
                operator: "Operator",
                newTab: "Magbubukas sa bagong tab",
                who: {
                    monitor: "Isang monitor na binanggit sa patakaran ng halalan",
                    hub: "Isang anchor hub na binanggit sa patakaran ng halalan",
                    operator: "Pinapatakbo ng awtoridad ng halalan",
                },
            },
            status: {
                title: "Katayuan ng balota",
                audit: "I-audit ang balota",
                auditNote:
                    "Hindi sigurado kung tama ang pag-encrypt ng portal sa iyong mga pinili? I-audit ang balotang ito: ipinapakita ng portal kung paano ito na-encrypt para masuri mo. Hindi na kailanman maibabato ang na-audit na balota, kaya bumoto ka ulit pagkatapos.",
                cast: "Naiboto noong {{when}}",
                audited: "Na-audit noong {{when}}",
                ballotCast: "Naiboto na ang balota",
                aria: "Katayuan ng balota: {{state}}. Ipakita ang mga detalye",
                ariaCast:
                    "Naiboto na ang balota. Katayuan ng balota: {{state}}. Ipakita ang mga detalye",
                tooltip: "{{state}} · Ipakita ang mga detalye",
                tracker: "Buksan sa ballot tracker",
                save: "I-save ang resibo",
                ok: "OK",
                tag: {
                    cast: "Naiboto",
                    notCast: "Hindi naiboto",
                    auditable: "Maaaring i-audit",
                    audited: "Na-audit",
                },
                state: {
                    notcast: "Hindi pa naiboto",
                    checking: "Sinusuri…",
                    recorded: "Nasa pampublikong talaan",
                    witnessed: "Kinumpirma ng mga saksi",
                    verified: "Na-verify",
                    late: "Huli",
                    problem: "Problema",
                    auditable: "Maaaring i-audit",
                    auditfail: "Nabigo ang audit",
                },
                head: {
                    late: "Mas matagal kaysa inaasahan",
                    mismatch: "Hindi tugma ang talaan",
                    damaged: "Sira ang resibo",
                    noReceipt: "Walang resibo para sa iyong balota",
                    unchecked: "Hindi masuri ang iyong balota",
                },
                sub: {
                    notcast:
                        "Naka-encrypt ang balotang ito sa device na ito at wala pang naipapadala. Kapag naiboto mo na ito, susuriin ng iyong browser kung nakarating ito sa pampublikong talaan.",
                    checking:
                        "Sinusuri ng iyong browser kung nasa pampublikong talaan ang iyong balota.",
                    recorded:
                        "Nakita ng iyong browser ang iyong balota sa pampublikong talaan. Susunod itong kukumpirmahin ng mga saksi.",
                    witnessed:
                        "Kinumpirma ng mga independiyenteng saksi ang talaan. Susunod ang mga pampublikong angkla sa loob ng {{minutes}} minuto; hindi na kailangang maghintay.",
                    verified:
                        "Nasa pampublikong talaan ang iyong balota, kinumpirma ng mga independiyenteng saksi at pampublikong naka-angkla.",
                    late: "Nasa pampublikong talaan ang iyong balota at kinumpirma ng mga saksi, ngunit huli ang mga pampublikong angkla nito.",
                    lateStep:
                        "Mas matagal kaysa inaasahan ang isang hakbang. Hindi ito patunay ng pakikialam, at patuloy na sumusuri ang iyong browser.",
                    mismatch:
                        "Ang entry #{{index}} ng pampublikong talaan ay hindi ang nasa resibong ito.",
                    mismatchNoEntry:
                        "Ang entry ng pampublikong talaan ay hindi ang nasa resibong ito.",
                    damaged:
                        "Hindi mabasa o ma-verify ang bahagi ng resibo o ng talaan. Pagkakamali ito sa data, hindi patunay ng pakikialam. Naiboto ang iyong boto.",
                    noReceipt:
                        "Walang natanggap na resibo ang website na ito para sa iyong balota. Naiboto ang iyong boto, at naabisuhan na ang mga tagamasid ng halalan.",
                    unchecked: "Hindi nasuri ng website na ito ang resibo. Naiboto ang iyong boto.",
                    auditable:
                        "Na-audit mo ang balotang ito, kaya hindi ito kailanman naiboto. Nasa pampublikong talaan ito, at dine-decrypt ito ng ballot tracker para ipakita ang mga napili rito.",
                    auditfail:
                        "Hindi ma-decrypt ang balota sa pampublikong talaan gamit ang susing inihayag ng portal. Huwag bumoto gamit ang device na ito; ipaalam sa awtoridad ng halalan.",
                },
                step: {
                    record: {
                        title: "Nasa pampublikong talaan",
                        upcoming:
                            "Hinahanap ng iyong browser ang balota sa pampublikong talaan ng halalan.",
                    },
                    witnesses: {
                        title: "Kinumpirma ng mga saksi",
                        upcoming:
                            "Pinipirmahan ng mga independiyenteng saksi ang talaang kasama ito.",
                    },
                    anchors: {
                        title: "Pampublikong naka-angkla",
                        upcoming:
                            "Hawak ng mga pampublikong hub, at ng mga log na sinusuri ng mga browser, ang talaang iyon.",
                    },
                },
                row: {
                    waiting: "Naghihintay sa naunang hakbang.",
                    checking: "Sinusuri ng iyong browser…",
                    anchoring:
                        "Mga pampublikong hub, pagkatapos ay kung saan sumusuri ang mga browser: sa loob ng {{minutes}} minuto.",
                    recorded: "Nasuri ng iyong browser ang patunay nito.",
                    recordedEntry: "Entry #{{index}}. Nasuri ng iyong browser ang patunay nito.",
                    witnessed:
                        "Sapat na bilang ng mga independiyenteng saksi ng patakaran ({{count}}) ang pumirma sa talaang kasama ito.",
                    anchoredWaict:
                        "Sa mga pampublikong hub ({{count}}) at kung saan sumusuri ang mga browser.",
                    anchoredBootstrap:
                        "Sa mga pampublikong hub ({{count}}) at kung saan sumusuri ang mga browser, sa pamamagitan muna ng pamalit na hub.",
                    anchoredHubs: "Sa mga pampublikong hub ({{count}}).",
                    anchoredOneHub: "Naka-angkla sa 1 hub, pinapatakbo ng {{party}}.",
                    anchoredOneHubPlatform: "Naka-angkla sa 1 hub, pinapatakbo ng platform.",
                    lateNow: "Dapat tapos na ito ngayon.",
                    lateStep: "Dapat tapos na ito pagsapit ng {{time}}.",
                    lateAnchors: "Dapat naangkla na ito ng mga hub pagsapit ng {{time}}.",
                    mismatch: "Iba ang entry #{{index}} sa talaan sa iyong resibo.",
                    mismatchNoEntry: "Iba ang isang entry sa talaan sa iyong resibo.",
                    contradiction: "Nagkakasalungatan ang mga nilagdaang patunay nito.",
                    damaged: "Sira ang patunay nito at hindi ma-verify.",
                    noReceipt: "Walang natanggap na resibo ang website na ito.",
                    unchecked: "Hindi nasuri ng website na ito ang resibo.",
                    skipped: "Hindi nasuri, dahil hindi nakumpleto ang naunang hakbang.",
                    skippedMismatch: "Hindi nasuri, dahil hindi tugma ang talaan.",
                },
                word: {
                    withinSeconds: "Sa loob ng ilang segundo",
                    withinMinutes: "Sa loob ng {{minutes}} min",
                    checking: "Sinusuri…",
                    late: "Huli",
                    mismatch: "Hindi tugma",
                    damaged: "Sira",
                    noReceipt: "Walang resibo",
                    unchecked: "Hindi nasuri",
                    skipped: "—",
                },
                callout: {
                    late: "Kadalasan, ibig sabihin nito ay mabagal o offline ang isang hub; hindi ito patunay ng pakikialam.",
                    lateKept:
                        "Muling sinusuri ito ng device na ito sa bawat balik mo, at masusuri rin ito ng ballot tracker mula sa anumang device.",
                    lateTracker: "Masusuri rin ito ng ballot tracker mula sa anumang device.",
                    evidence:
                        "Itinago ng iyong browser ang ebidensya: ang resibong ito at ang nilagdaang entry ng talaan. Ipadala ito sa awtoridad ng halalan o sa isang tagamasid; sinuman ay makakasuri nito.",
                    saveEvidence: "I-save ang ebidensya",
                },
                caption: {
                    notcast:
                        "Walang susuriin hangga't hindi mo ito naiboboto. Pagkatapos, ipapakita ng check mark na ito kung hanggang saan na na-verify ang iyong balota.",
                    checking:
                        "Sinusuri ng iyong browser ang pampublikong talaan, gamit ang code na masisiyasat ng sinuman.",
                    problem:
                        "Natuklasan ng iyong browser, gamit ang code na masisiyasat ng sinuman.",
                    auditable:
                        "Na-decrypt ng iyong browser, gamit ang code na masisiyasat ng sinuman.",
                    verified:
                        "Na-verify ng iyong browser, gamit ang code na masisiyasat ng sinuman.",
                    checked: "Sinuri ng iyong browser, gamit ang code na masisiyasat ng sinuman.",
                    kept: "Itinatago ng device na ito ang resibo at muling sinusuri ito sa bawat pagbisita.",
                },
            },
            signers: {
                witnesses: {open: "Sino ang pumirma", close: "Itago kung sino ang pumirma"},
                hubs: {open: "Aling mga hub", close: "Itago kung aling mga hub"},
                runBy: "Pinapatakbo ng {{party}}",
                runByAlso: "Pinapatakbo ng {{party}}, na nagpapatakbo rin ng {{hubs}}",
                signed: "Pumirma {{time}}",
                signedNoTime: "Pumirma",
                notYet: "Hindi pa",
                late: "Huli",
                anchored: "Naka-angkla {{time}}",
                anchoredNoTime: "Naka-angkla",
                within: "Sa loob ng {{minutes}} min",
                check: "Suriin sa site nito",
                checkWitness: "Suriin sa site nito: saksing {{name}} (bubukas sa bagong tab)",
                checkHub: "Suriin sa site nito: hub na {{name}} (bubukas sa bagong tab)",
                linksNote:
                    "Binubuksan ng “Suriin sa site nito” ang sariling explorer ng pumirma, na doon sinusuri ang lagdang iyon.",
                standIn: {
                    name: "Kung saan sumusuri ang mga browser",
                    who: "Pamalit na hub, sa ngayon",
                },
                rule: {
                    witnesses:
                        "Hinihingi ng patakaran na pumirma ang {{needed}} sa {{total}} nitong saksi. {{signed}} ang pumirma.",
                    witnessesAlone:
                        "Hinihingi ng patakaran na pumirma ang {{needed}} sa {{total}} nitong saksi, at walang iisang panig na nagpapatakbo ng ganoon karami. {{signed}} ang pumirma.",
                    witnessesCount: "{{signed}} sa {{total}} saksi ng patakaran ang pumirma.",
                    hubs: "Dapat i-angkla ng lahat ng {{count}} hub ang talaan, at hindi bababa sa {{independent}} ang dapat patakbuhin ng iba maliban sa awtoridad ng halalan.",
                    hubsPlatform:
                        "Dapat i-angkla ng lahat ng {{count}} hub ang talaan, at hindi bababa sa {{independent}} ang dapat patakbuhin ng hindi awtoridad ng halalan ni ng platform.",
                    hubsAll: "Dapat i-angkla ng lahat ng {{count}} hub ang talaan.",
                    oneHub: "Isang hub lang ang pinangalanan ng patakaran, pinapatakbo ng {{party}}.",
                    oneHubPlatform:
                        "Isang hub lang ang pinangalanan ng patakaran, pinapatakbo ng platform.",
                },
            },
            updates: {
                title: "Kumuha ng mga update sa email tungkol sa balotang ito",
                description:
                    "Kapag nakumpirma na ito at kapag nabilang na, o kung may naitalang problema. Pumili ng alinmang hub: sinusuri ng bawat isa ang talaan nang mag-isa.",
                listLabel: "Mga hub sa patakaran ng halalang ito",
                runBy: "Pinapatakbo ng {{party}}",
                hubLabel:
                    "{{name}}, pinapatakbo ng {{party}}: kumuha ng mga update sa email sa sarili nitong site (bubukas sa bagong tab)",
                footnote:
                    "Ang mga hub na pinangalanan ng patakaran ng halalan, sa random na pagkakasunod-sunod sa bawat pagkakataon. Bawat isa ay bubukas sa sarili nitong site kasama ang iyong resibo sa link; walang ibinabahagi hanggang sa pumili ka roon kung ano ang susubaybayan.",
                footnoteNoReceipt:
                    "Ang mga hub na pinangalanan ng patakaran ng halalan, sa random na pagkakasunod-sunod sa bawat pagkakataon. Bawat isa ay bubukas sa sarili nitong site; walang ibinabahagi hanggang sa pumili ka roon kung ano ang susubaybayan.",
            },
            booth: {
                title: "Hindi pa maihahanda ang iyong balota",
                explanation:
                    "Bago i-encrypt ang iyong balota, tinitingnan ng website na ito kung ang mga setting ng halalan ay ang mga nakalathala sa pampublikong talaan ng halalan. Hindi pa na-encrypt o naipadala ang iyong balota, at nananatili rito ang iyong mga pinili.",
                pending:
                    "Hindi pa available ang mga nakalathalang setting. Pakisubukan muli mamaya-maya.",
                invalid:
                    "Hindi mabasa ang mga nakalathalang setting. Pakisubukan muli mamaya-maya.",
                failed: "Hindi tugma ang mga setting sa mga nakalathala. Naabisuhan na ang mga tagamasid ng halalan. Pakikontak ang suporta.",
                retry: "Subukan muli",
                close: "Isara",
            },
            record: {
                json: "JSON",
                text: "Teksto",
                fields_one: "{{count}} field",
                fields_other: "{{count}} field",
                items_one: "{{count}} item",
                items_other: "{{count}} item",
                size: {
                    bytes: "{{value}} B",
                    kilobytes: "{{value}} KB",
                },
                copy: "Kopyahin ang {{file}}",
                copied: "Nakopya",
                download: "I-download ang {{file}}",
            },
        },
    },
}

export default tagalogTranslation
