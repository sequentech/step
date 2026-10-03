// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const dutchTranslation: TranslationType = {
    translations: {
        language: "Nederlands",
        welcome: "Hello <br/> <strong>World</strong>",
        breadcrumbSteps: {
            select: "Select a Verifier",
            import: "Import Data",
            verify: "Verify",
            finish: "Finish",
        },
        electionEventBreadcrumbSteps: {
            created: "Created",
            keys: "Keys",
            publish: "Publish",
            started: "Started",
            ended: "Ended",
            results: "Results",
        },
        a11y: {
            closeDialog: "Dialoogvenster sluiten",
            dismissMessage: "Bericht sluiten",
            ballotIdHelp: "Over uw stembiljet-ID",
            loading: "Laden",
            severity: {
                error: "Fout",
                warning: "Waarschuwing",
                success: "Gelukt",
                info: "Informatie",
            },
            selectList: "De hele lijst selecteren",
            preferenceLabel: "Voorkeur",
            writeInFor: "Naam van de geschreven kandidaat",
        },
        candidate: {
            moreInformationLink: "More information",
            writeInsPlaceholder: "Type write-in candidate here",
            blankVote: "Blank Vote",
            preferential: {
                position: "Positie",
                none: "Geen",
                ordinals: {
                    first: "e",
                    second: "e",
                    third: "e",
                    other: "e",
                },
            },
        },
        homeScreen: {
            title: "Sequent Ballot Verifier",
            description1:
                "The ballot verifier is used when the voter chooses to audit the ballot in the voting booth. The verification should take 1-2 minutes.",
            description2:
                "The ballot verifier allows the voter to ensure that the encrypted ballot correctly captures the selections made in the voting booth. Allowing to perform this check is called cast-as-intended verifiability and prevents errors and malicious activity during ballot encryption.",
            descriptionMore: "Learn more",
            startButton: "Browse file",
            dragDropOption: "Or drag and drop it here",
            importErrorDescription:
                "There was a problem importing the auditable ballot. Did you choose  the right file?",
            importErrorMoreInfo: "More info",
            importErrorTitle: "Error",
            useSampleText: "Don't have an auditable ballot?",
            useSampleLink: "Use a sample auditable ballot",
        },
        confirmationScreen: {
            title: "Sequent Ballot Verifier",
            topDescription1:
                "Based on the information in the imported Auditable Ballot, we calculated that:",
            topDescription2: "If this is the Ballot ID shown in the Voting Booth:",
            bottomDescription1:
                "Your ballot was encrypted correctly. You can now close this window and return to the Voting Booth.",
            bottomDescription2:
                "If they don't match, click here to learn more about the potential reasons and what actions you can take.",
            ballotChoicesDescription: "And your ballot choices are:",
            helpAndFaq: "Help & FAQ",
            backButton: "Back",
            markedInvalid: "Ballot explicitly marked invalid",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Status",
                content:
                    "The status panel gives you information about the  verifications performed.",
                ok: "OK",
            },
        },
        footer: {
            poweredBy: "Aangedreven door <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Niet genoeg keuzes om te decoderen",
                writeInChoiceOutOfRange: "In te vullen keuze buiten bereik: {{index}}",
                writeInNotEndInZero: "In te vullen tekst eindigt niet op 0",
                writeInCharsExceeded:
                    "In te vullen tekst overschrijdt maximum aantal tekens met {{numCharsExceeded}}. Moet worden aangepast.",
                bytesToUtf8Conversion:
                    "Fout bij het converteren van in te vullen tekst van bytes naar UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Stembiljet groter dan verwacht",
            },
            implicit: {
                selectedMax:
                    "Te veel stemmen: Aantal geselecteerde keuzes {{numSelected}} is meer dan het maximum {{max}}",
                selectedMin:
                    "Aantal geselecteerde keuzes {{numSelected}} is minder dan het minimum {{min}}",
                maxSelectionsPerType:
                    "Aantal geselecteerde keuzes {{numSelected}} voor lijst {{type}} is meer dan het maximum {{max}}",
                underVote:
                    "Te weinig stemmen: Aantal geselecteerde keuzes {{numSelected}} is minder dan het maximum {{max}}",
                overVoteDisabled:
                    "Maximum bereikt: U heeft het maximum aantal keuzes {{numSelected}} geselecteerd. Om uw selectie te wijzigen, deselecteer eerst een andere optie.",
                blankVote: "Blanco stem: 0 keuzes geselecteerd",
                preferenceOrderWithGaps:
                    "Ongeldige stem! De voorkeursvolgorde heeft een of meer hiaten.",
                duplicatedPosition:
                    "Ongeldige stem! Dezelfde positie is geselecteerd voor twee of meer kandidaten.",
            },
            explicit: {
                notAllowed:
                    "Stembiljet expliciet ongeldig gemarkeerd maar vraag staat dit niet toe",
                alert: "Gemarkeerde selectie wordt als ongeldige stem beschouwd.",
            },
            configuration: {
                multipleExplicitInvalidCandidates:
                    "Ongeldige stemconfiguratie: de verkiezing definieert {{count}} expliciet ongeldige kandidaten, maar er is er maar één toegestaan.",
                multipleExplicitBlankCandidates:
                    "Ongeldige stemconfiguratie: de verkiezing definieert {{count}} expliciete blanco kandidaten, maar er is er maar één toegestaan.",
                invalidSlateConfiguration:
                    "Ongeldige stemconfiguratie: de lijsten zijn niet geldig ({{reason}}).",
            },
        },
        ballotHash: "Your Ballot ID: {{ballotId}}",
        version: {
            header: "Version:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Logout",
            modal: {
                title: "Are you sure you want to logout?",
                content: "You are about to close this application. This action can not be undone. ",
                ok: "OK",
                close: "Close",
            },
        },
        stories: {
            openDialog: "Open Dialog",
        },
        dragNDrop: {
            firstLine: "Drag & drop files or",
            browse: "Browse",
            format: "Supported format: txt",
            importError: "Dit bestand kon niet worden geïmporteerd. Probeer het opnieuw.",
        },
        selectElection: {
            electionWebsite: "Ballot Website",
            countdown:
                "Election Begins in {{years}} years, {{months}} months, {{weeks}} weeks, {{days}} days, {{hours}} hours, {{minutes}} minutes, {{seconds}} seconds",
            openElection: "Open",
            closedElection: "Closed",
            voted: "Voted",
            notVoted: "Not voted",
            resultsButton: "Ballot Results",
            voteButton: "Click to Vote",
            openDate: "Open: ",
            closeDate: "Close: ",
            ballotLocator: "Locate your ballot",
        },
        header: {
            profile: "Profile",
            welcome: "Welcome,<br><span>{{name}}</span>",
            session: {
                title: "Your session is going to expire.",
                timeLeft: "You have {{time}} left to cast your vote.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutes and {{time}} seconds",
                timeLeftSeconds: "{{timeLeft}} seconds",
            },
        },
        problems: {
            noProblems: "Niets te melden. Dit zou worden geïmporteerd.",
            errors_one: "{{count}} fout",
            errors_other: "{{count}} fouten",
            errorsExplained: "Dit wordt pas geïmporteerd als elk van deze fouten is opgelost.",
            warnings_one: "{{count}} waarschuwing",
            warnings_other: "{{count}} waarschuwingen",
            warningsExplained:
                "Dit wordt geïmporteerd. Elk van deze punten is waarschijnlijk niet wat bedoeld was.",
            warningsStrict: "De strikte modus staat aan, dus deze houden de build tegen.",
            messages: {
                area: {
                    "cycle": {
                        lead: "Gebieden binnen elkaar",
                        text: "Gebieden binnen elkaar — gebied '{{area}}' maakt deel uit van een kring van bovenliggende gebieden, waardoor het Beheerportaal zou vastlopen.",
                    },
                    "duplicate-name": {
                        lead: "Twee gebieden met dezelfde naam",
                        text: "Twee gebieden met dezelfde naam — '{{name}}' is zowel '{{first}}' als '{{second}}'. De kiezers-CSV bepaalt een gebied op naam, dus kiezers komen terecht in het gebied dat de importeur het eerst vindt.",
                    },
                    "early-voting-unknown": {
                        lead: "Onbekende instelling voor vervroegd stemmen",
                        text: "Onbekende instelling voor vervroegd stemmen — '{{value}}' is niet geldig voor een gebied. Het platform accepteert {{allowed}}.",
                    },
                    "empty-ballot": {
                        lead: "Gebied met een leeg stembiljet",
                        text: "Gebied met een leeg stembiljet — gebied '{{area}}' stemt over geen enkele stemming, noch een eigen noch een geërfde van een gebied waarbinnen het ligt, dus de kiezers zouden een leeg stembiljet zien.",
                    },
                    "inside-itself": {
                        lead: "Gebied binnen zichzelf",
                        text: "Gebied binnen zichzelf — een gebied kan niet zijn eigen bovenliggende gebied zijn.",
                    },
                    "no-identifier": {
                        lead: "Gebied zonder identificatie",
                        text: "Gebied zonder identificatie — elk gebied heeft er een nodig.",
                    },
                    "no-name": {
                        lead: "Gebied zonder naam",
                        text: "Gebied zonder naam — de kiezers-CSV bepaalt het gebied van een kiezer op naam, niet op id, dus in een gebied zonder naam kan geen enkele kiezer worden geplaatst.",
                    },
                    "parent-missing": {
                        lead: "Bovenliggend gebied ontbreekt",
                        text: "Bovenliggend gebied ontbreekt — het gebied ligt binnen een bovenliggend gebied dat niet in dit bestand staat.",
                    },
                    "parent-unknown": {
                        lead: "Bovenliggend gebied onbekend",
                        text: "Bovenliggend gebied onbekend — niets heeft de identificatie '{{parent}}'.",
                    },
                },
                ballot: {
                    "no-elections": {
                        lead: "Geen verkiezingen",
                        text: "Geen verkiezingen — een verkiezingsevenement heeft er minstens één nodig.",
                    },
                },
                bundle: {
                    "duplicate-id": {
                        lead: "Identificatie twee keer gebruikt",
                        text: "Identificatie twee keer gebruikt — {{id}} in {{kind}} wordt ook gebruikt door {{previous}}.",
                    },
                    "event-no-encryption": {
                        lead: "Geen versleutelingsprotocol",
                        text: "Geen versleutelingsprotocol — het verkiezingsevenement zegt niet hoe stembiljetten worden versleuteld.",
                    },
                    "event-no-id": {
                        lead: "Evenement zonder id",
                        text: "Evenement zonder id — het verkiezingsevenement in dit bestand heeft geen identificatie.",
                    },
                    "no-areas": {
                        lead: "Geen gebieden",
                        text: "Geen gebieden — geen enkele kiezer kan een stembiljet krijgen zolang het evenement geen gebied heeft.",
                    },
                    "no-elections": {
                        lead: "Geen verkiezingen",
                        text: "Geen verkiezingen — een verkiezingsevenement heeft minstens één verkiezing nodig.",
                    },
                    "no-tenant": {
                        lead: "Geen tenant",
                        text: "Geen tenant — het bestand zegt niet bij welke tenant het hoort.",
                    },
                    "tenant-not-uuid": {
                        lead: "Tenant is geen identificatie",
                        text: "Tenant is geen identificatie — '{{value}}' is geen geldige tenant-id.",
                    },
                },
                candidate: {
                    "contest-missing": {
                        lead: "Stemming van de kandidaat ontbreekt",
                        text: "Stemming van de kandidaat ontbreekt — de kandidaat hoort bij een stemming die niet in dit bestand staat.",
                    },
                    "no-contest": {
                        lead: "Kandidaat zonder stemming",
                        text: "Kandidaat zonder stemming — elke kandidaat moet bij een stemming horen.",
                    },
                    "picture-mismatch": {
                        lead: "Foto verwijst ergens anders naar",
                        text: "Foto verwijst ergens anders naar — het stembiljet toont '{{url}}', wat niet verwijst naar het document '{{document}}' ernaast. Na de import zouden de twee naar verschillende bestanden verwijzen.",
                    },
                    "picture-not-shown": {
                        lead: "Foto nooit getoond",
                        text: "Foto nooit getoond — een kandidaat noemt een foto waarnaar geen enkel onderdeel van het stembiljet verwijst, dus hij zou worden geüpload en nooit getoond.",
                    },
                    "picture-unrecorded": {
                        lead: "Foto niet vastgelegd",
                        text: "Foto niet vastgelegd — de foto van een kandidaat staat op het stembiljet en nergens is vastgelegd welk document het is, dus hij kan achteraf niet op het platform worden gewijzigd of verwijderd.",
                    },
                },
                census: {
                    "column-not-declared": {
                        lead: "Registerkolom niet gedeclareerd",
                        text: "Registerkolom niet gedeclareerd — {{message}}",
                    },
                    "no-area-column": {
                        lead: "Kiezersregister noemt geen gebieden",
                        text: "Kiezersregister noemt geen gebieden — deze verkiezing heeft gebieden, maar elke kiezer zou het standaardstembiljet krijgen. Als de indeling moet gelden, heeft het register een gebiedskolom nodig.",
                    },
                    "note": {
                        lead: "Over het kiezersregister",
                        text: "Over het kiezersregister — {{message}}",
                    },
                    "unreadable": {
                        lead: "Kiezersregister onleesbaar",
                        text: "Kiezersregister onleesbaar — {{message}}",
                    },
                    "unreadable-member": {
                        lead: "Kiezersregister in de zip onleesbaar",
                        text: "Kiezersregister in de zip onleesbaar — {{message}}",
                    },
                },
                channels: {
                    "early-voting-closed": {
                        lead: "Vervroegd stemmen uitgeschakeld",
                        text: "Vervroegd stemmen uitgeschakeld — {{areas}} staan vervroegd stemmen toe en het evenement opent dat kanaal niet, dus de instelling doet niets.",
                    },
                    "early-voting-no-area": {
                        lead: "Vervroegd stemmen zonder gebied",
                        text: "Vervroegd stemmen zonder gebied — vervroegd stemmen staat open en geen enkel gebied staat het toe, dus de vervroegde periode zou geen kiezers hebben.",
                    },
                    "kiosk-client": {
                        lead: "Kiosk heeft een eigen client nodig",
                        text: "Kiosk heeft een eigen client nodig — stemmen via de kiosk vereist een authenticatieclient met de naam van de gewone client gevolgd door '-kiosk', en die maakt dit bestand niet aan.",
                    },
                    "none-open": {
                        lead: "Geen manier van stemmen",
                        text: "Geen manier van stemmen — elk stemkanaal staat uit, dus niemand kan stemmen.",
                    },
                    "telephone-elsewhere": {
                        lead: "Telefonisch stemmen later ingesteld",
                        text: "Telefonisch stemmen later ingesteld — telefonisch stemmen wordt na de import ingesteld op het IVR-tabblad van het evenement; niets daarvan staat in dit bestand.",
                    },
                    "unknown": {
                        lead: "Onbekende manier van stemmen",
                        text: "Onbekende manier van stemmen — niets leest '{{name}}'. De manieren van stemmen waarop het platform reageert zijn {{allowed}}.",
                    },
                },
                contacts: {
                    none: {
                        lead: "Geen contactpersonen",
                        text: "Geen contactpersonen — op de verkiezingsdag zijn dit de mensen die worden gebeld.",
                    },
                },
                contest: {
                    "algorithm-unknown": {
                        lead: "Onbekende telmethode",
                        text: "Onbekende telmethode — '{{value}}' is geen telalgoritme. Het platform accepteert {{allowed}}.",
                    },
                    "area-unknown": {
                        lead: "Gebied van de stemming onbekend",
                        text: "Gebied van de stemming onbekend — niets heeft de identificatie '{{area}}'.",
                    },
                    "cap-invalid": {
                        lead: "Onmogelijke selectielimiet",
                        text: "Onmogelijke selectielimiet — {{cap}} is geen aantal selecties.",
                    },
                    "cap-never-applies": {
                        lead: "Selectielimiet geldt nooit",
                        text: "Selectielimiet geldt nooit — een limiet van {{cap}} per type geldt nooit in een stemming waarin een kiezer er in totaal {{max}} mag kiezen.",
                    },
                    "chooses-more-than-available": {
                        lead: "Meer keuzes dan kandidaten",
                        text: "Meer keuzes dan kandidaten — een kiezer mag er {{max}} kiezen uit {{available}} kandidaten.",
                    },
                    "chooses-more-than-offered": {
                        lead: "Meer keuzes dan opties",
                        text: "Meer keuzes dan opties — een kiezer mag er tot {{chosen}} kiezen, maar er zijn er maar {{offered}} om uit te kiezen.",
                    },
                    "columns-invalid": {
                        lead: "Onmogelijke indeling",
                        text: "Onmogelijke indeling — {{columns}} kolommen is geen indeling.",
                    },
                    "columns-too-many": {
                        lead: "Te veel kolommen",
                        text: "Te veel kolommen — {{columns}} kolommen zijn onleesbaar op een telefoon, en daarmee stemmen de meeste kiezers.",
                    },
                    "count-missing": {
                        lead: "Stemming mist een getal",
                        text: "Stemming mist een getal — de stemming heeft {{field}} nodig.",
                    },
                    "count-negative": {
                        lead: "Negatief aantal",
                        text: "Negatief aantal — {{field}} is {{value}}, en een aantal kan niet onder nul liggen.",
                    },
                    "election-missing": {
                        lead: "Verkiezing van de stemming ontbreekt",
                        text: "Verkiezing van de stemming ontbreekt — de stemming hoort bij een verkiezing die niet in dit bestand staat.",
                    },
                    "elects-more-than-available": {
                        lead: "Meer zetels dan kandidaten",
                        text: "Meer zetels dan kandidaten — de stemming kiest {{winners}} uit {{available}} kandidaten.",
                    },
                    "elects-more-than-chosen": {
                        lead: "Kiest meer dan toegestaan",
                        text: "Kiest meer dan toegestaan — de stemming kiest {{winners}}, maar een kiezer mag er maar {{chosen}} kiezen.",
                    },
                    "elects-more-than-standing": {
                        lead: "Meer zetels dan kandidaten",
                        text: "Meer zetels dan kandidaten — de stemming kiest {{winners}} uit {{standing}} kandidaten.",
                    },
                    "elects-nobody": {
                        lead: "Kiest niemand",
                        text: "Kiest niemand — de stemming heeft geen winnaars.",
                    },
                    "max-votes-below-one": {
                        lead: "Niets om op te stemmen",
                        text: "Niets om op te stemmen — een kiezer mag minder dan één kandidaat kiezen.",
                    },
                    "min-above-max": {
                        lead: "Minimum boven maximum",
                        text: "Minimum boven maximum — een kiezer moet er minstens {{min}} kiezen maar mag er hoogstens {{max}} kiezen.",
                    },
                    "no-candidates": {
                        lead: "Geen kandidaten",
                        text: "Geen kandidaten — er is nog niemand kandidaat in deze stemming.",
                    },
                    "no-candidates-in-bundle": {
                        lead: "Geen kandidaten",
                        text: "Geen kandidaten — de stemming heeft geen kandidaten, dus niemand kan erin stemmen.",
                    },
                    "on-no-ballot": {
                        lead: "Stemming op geen enkel stembiljet",
                        text: "Stemming op geen enkel stembiljet — geen enkel gebied heeft deze stemming op zijn stembiljet, dus niemand kan erin stemmen.",
                    },
                    "policy-not-text": {
                        lead: "Stembiljetregel is geen tekst",
                        text: "Stembiljetregel is geen tekst — {{key}} moet tekst zijn, en is {{value}}.",
                    },
                    "policy-unknown": {
                        lead: "Onbekende stembiljetregel",
                        text: "Onbekende stembiljetregel — '{{value}}' is geen geldige {{key}}. Het platform accepteert {{allowed}}.",
                    },
                    "ranked-counted-unranked": {
                        lead: "Rangschikkend stembiljet geteld zonder rangorde",
                        text: "Rangschikkend stembiljet geteld zonder rangorde — een preferentiële stemming wordt geteld met '{{algorithm}}', dat de rangorde van de kiezers negeert.",
                    },
                    "tie-breaking-unknown": {
                        lead: "Onbekende regel bij gelijke stand",
                        text: "Onbekende regel bij gelijke stand — '{{value}}' is geen beleid voor gelijke stand. Het platform accepteert {{allowed}}.",
                    },
                    "unranked-counted-ranked": {
                        lead: "Gewoon stembiljet geteld op rangorde",
                        text: "Gewoon stembiljet geteld op rangorde — een niet-preferentiële stemming wordt geteld met '{{algorithm}}', dat gerangschikte stembiljetten nodig heeft.",
                    },
                    "voting-type-unknown": {
                        lead: "Onbekend stemtype",
                        text: "Onbekend stemtype — '{{value}}' is geen stemtype. Het platform accepteert {{allowed}}.",
                    },
                    "write-in-slots-not-allowed": {
                        lead: "Invulvakken niet toegestaan",
                        text: "Invulvakken niet toegestaan — {{count}} invulvakken staan op een stemming die zelf invullen niet toestaat, waardoor er naamloze opties op het stembiljet komen.",
                    },
                    "write-ins-no-slot": {
                        lead: "Zelf invullen zonder invulvak",
                        text: "Zelf invullen zonder invulvak — zelf een naam invullen is toegestaan en de stemming heeft geen invulvak, dus een kiezer kan nergens een naam typen.",
                    },
                },
                delivery: {
                    "no-importable": {
                        lead: "Geen importeerbaar archief",
                        text: "Geen importeerbaar archief — deze zip bevat een plan maar niet het archief dat het Beheerportaal importeert, dus het kiezersregister en de bestanden die het noemt zitten er niet in.",
                    },
                },
                election: {
                    "channels-differ": {
                        lead: "Verkiezing en evenement verschillen",
                        text: "Verkiezing en evenement verschillen — {{channels}} wijkt af van die van het evenement, dus de startknoppen voor deze verkiezing zouden niet overeenkomen.",
                    },
                    "grace-disallowed": {
                        lead: "Uitlooptijd uitgeschakeld",
                        text: "Uitlooptijd uitgeschakeld — er is {{seconds}} seconden uitlooptijd ingesteld maar uitlooptijd is niet toegestaan, dus het stemmen sluit op de deadline.",
                    },
                    "grace-negative": {
                        lead: "Negatieve uitlooptijd",
                        text: "Negatieve uitlooptijd — {{value}} seconden is geen tijdsduur.",
                    },
                    "grace-zero": {
                        lead: "Uitlooptijd van nul",
                        text: "Uitlooptijd van nul — een uitlooptijd is toegestaan en duurt nul seconden, dus die is er niet.",
                    },
                    "no-contests": {
                        lead: "Geen stemmingen",
                        text: "Geen stemmingen — niemand stemt in deze verkiezing.",
                    },
                    "revotes-negative": {
                        lead: "Onmogelijk aantal stemmen",
                        text: "Onmogelijk aantal stemmen — {{value}} is geen aantal keren dat een kiezer mag stemmen.",
                    },
                    "setting-unknown": {
                        lead: "Onbekende verkiezingsinstelling",
                        text: "Onbekende verkiezingsinstelling — '{{value}}' is geen geldige {{key}}. Het platform accepteert {{allowed}}.",
                    },
                    "spoil-without-revote": {
                        lead: "Ongeldig maken zonder tweede kans",
                        text: "Ongeldig maken zonder tweede kans — een kiezer mag een uitgebracht stembiljet weggooien en heeft geen tweede poging om het te vervangen.",
                    },
                },
                event: {
                    "no-identifier": {
                        lead: "Geen identificatie",
                        text: "Geen identificatie — elke gegenereerde id wordt ervan afgeleid, dus zonder identificatie kan niets twee keer op dezelfde manier worden opgebouwd.",
                    },
                    "no-name": {
                        lead: "Geen naam",
                        text: "Geen naam — kiezers zien de naam boven het stembiljet, en hij wordt de titel van de inlogpagina.",
                    },
                    "setting-unknown": {
                        lead: "Onbekende evenementinstelling",
                        text: "Onbekende evenementinstelling — '{{value}}' is geen geldige {{key}}. Het platform accepteert {{allowed}}.",
                    },
                },
                file: {
                    "cannot-decrypt": {
                        lead: "Kon niet ontsleutelen",
                        text: "Kon niet ontsleutelen — het bestand is versleuteld en ging niet open. Controleer het wachtwoord.",
                    },
                    "checksum-mismatch": {
                        lead: "Niet het verwachte bestand",
                        text: "Niet het verwachte bestand — de SHA-256 komt niet overeen met de opgegeven waarde, dus het is niet het bedoelde bestand. Controleer de checksum of upload het bestand opnieuw.",
                    },
                    "duplicate-name": {
                        lead: "Bestandsnaam twee keer gebruikt",
                        text: "Bestandsnaam twee keer gebruikt — '{{file}}' verwijst naar twee verschillende dingen. Bestanden worden op naam meegestuurd, dus het ene zou stilzwijgend het andere worden.",
                    },
                    "missing": {
                        lead: "Bestand ontbreekt",
                        text: "Bestand ontbreekt — '{{file}}' wordt genoemd en niets hier bevat het. Een lege archiefvermelding laat de import mislukken in plaats van een bestand kwijt te raken.",
                    },
                    "not-a-bundle": {
                        lead: "Geen export van een verkiezingsevenement",
                        text: "Geen export van een verkiezingsevenement — het bestand is gelezen maar heeft niet de vorm van een export: {{reason}}",
                    },
                    "not-json": {
                        lead: "Geen leesbaar bestand",
                        text: "Geen leesbaar bestand — dit is geen export van een verkiezingsevenement die het platform kan lezen: {{reason}}",
                    },
                    "unreadable-archive": {
                        lead: "Archief onleesbaar",
                        text: "Archief onleesbaar — {{reason}}",
                    },
                    "unused": {
                        lead: "Bestand niet gebruikt",
                        text: "Bestand niet gebruikt — '{{file}}' is aangeleverd en niets noemt het, dus het zou met de levering meegaan en aan niemand worden getoond.",
                    },
                    "version-incompatible": {
                        lead: "Geëxporteerd door een andere versie",
                        text: "Geëxporteerd door een andere versie — het bestand komt van versie {{found}}, die versie {{current}} niet kan importeren.",
                    },
                },
                identifier: {
                    duplicate: {
                        lead: "Identificatie twee keer gebruikt",
                        text: "Identificatie twee keer gebruikt — '{{identifier}}' wordt al gebruikt door {{first}}. Identificaties zijn uniek binnen het hele verkiezingsevenement, dus de tweede vervangt de eerste in plaats van te worden toegevoegd.",
                    },
                },
                ivr: {
                    "language-not-spoken": {
                        lead: "Taal niet beschikbaar per telefoon",
                        text: "Taal niet beschikbaar per telefoon — het gesprek spreekt alleen Engels, Frans en Spaans, dus bellers krijgen {{languages}} niet aangeboden.",
                    },
                    "missing-prompts": {
                        lead: "Gespreksteksten ontbreken",
                        text: "Gespreksteksten ontbreken — {{prompts}} hebben geen tekst in '{{language}}', en het telefoonsysteem weigert elk gesprek tot die er is.",
                        lead_one: "Gesprekstekst ontbreekt",
                        text_one:
                            "Gesprekstekst ontbreekt — {{prompts}} heeft geen tekst in '{{language}}', en het telefoonsysteem weigert elk gesprek tot die er is.",
                    },
                },
                labels: {
                    "in-use": {
                        lead: "Rechtenlabels in gebruik",
                        text: "Rechtenlabels in gebruik — {{labels}}. Alles met een label is verborgen voor elke beheerder zonder dat label, dus wie dit importeert heeft er een op het eigen account nodig, anders toont het Beheerportaal een lege lijst.",
                    },
                },
                languages: {
                    "default-not-offered": {
                        lead: "Standaardtaal niet aangeboden",
                        text: "Standaardtaal niet aangeboden — '{{chosen}}' staat niet tussen {{offered}}, dus kiezers zouden in plaats daarvan de eerste krijgen.",
                    },
                    "detection-unknown": {
                        lead: "Onbekende taaldetectie",
                        text: "Onbekende taaldetectie — '{{policy}}' is geen beleid dat het platform kent.",
                    },
                    "none": {
                        lead: "Geen talen",
                        text: "Geen talen — het stembiljet valt terug op Engels, wat een vangnet is en geen keuze.",
                    },
                },
                link: {
                    "area-missing": {
                        lead: "Stembiljetkoppeling naar een ontbrekend gebied",
                        text: "Stembiljetkoppeling naar een ontbrekend gebied — een stemming staat op het stembiljet van een gebied dat niet in dit bestand staat.",
                    },
                    "contest-missing": {
                        lead: "Stembiljetkoppeling naar een ontbrekende stemming",
                        text: "Stembiljetkoppeling naar een ontbrekende stemming — het stembiljet van een gebied noemt een stemming die niet in dit bestand staat.",
                    },
                },
                logo: {
                    "file-and-link": {
                        lead: "Twee logo's",
                        text: "Twee logo's — dit plan bevat zowel een geüpload logo ('{{file}}') als een link ('{{url}}'). Het bestand wordt meegeleverd; de link wordt genegeerd.",
                    },
                    "file-missing": {
                        lead: "Logobestand ontbreekt",
                        text: "Logobestand ontbreekt — '{{file}}' is als logo opgegeven en niet aangeleverd. Zet het bestand naast de werkmap onder precies die naam.",
                    },
                    "no-bytes": {
                        lead: "Logobestand leeg",
                        text: "Logobestand leeg — '{{file}}' is als logo opgegeven en bevat geen bytes. Een lege archiefvermelding laat de import mislukken in plaats van een afbeelding kwijt te raken.",
                    },
                },
                material: {
                    "empty-document": {
                        lead: "Materiaal met een leeg document",
                        text: "Materiaal met een leeg document — het zou worden geïmporteerd als een link naar niets. Laat het document helemaal weg voor materiaal zonder bestand.",
                    },
                    "file-missing": {
                        lead: "Materiaalbestand ontbreekt",
                        text: "Materiaalbestand ontbreekt — '{{file}}' wordt hier genoemd en is niet aangeleverd. Zet het bestand naast de werkmap onder precies die naam.",
                    },
                    "file-unused": {
                        lead: "Materiaalbestand niet gebruikt",
                        text: "Materiaalbestand niet gebruikt — '{{file}}' is aangeleverd en geen enkele rij noemt het, dus het zou worden geüpload en aan niemand worden getoond.",
                    },
                    "no-identifier": {
                        lead: "Materiaal zonder identificatie",
                        text: "Materiaal zonder identificatie — de id van het document wordt ervan afgeleid, dus zonder identificatie kan het bestand niet aan de rij worden gekoppeld.",
                    },
                    "tab-off": {
                        lead: "Tabblad Materiaal uitgeschakeld",
                        text: "Tabblad Materiaal uitgeschakeld — dit bestand bevat {{count}} ondersteunende materialen en het tabblad dat ze toont staat uit, dus kiezers zouden ze nooit zien.",
                    },
                    "wrong-event": {
                        lead: "Materiaal van een ander evenement",
                        text: "Materiaal van een ander evenement — dit ondersteunend materiaal hoort bij een ander verkiezingsevenement.",
                    },
                },
                messages: {
                    "weekly-no-time": {
                        lead: "Een herhaling zonder uur",
                        text: "Een herhaling zonder uur — een bericht wordt elke week herhaald maar zegt niet op welk tijdstip, dus wie het verstuurt moet een uur kiezen dat niemand heeft opgeschreven.",
                    },
                },
                passwords: {
                    "column-already-there": {
                        lead: "Wachtwoorden twee keer opgegeven",
                        text: "Wachtwoorden twee keer opgegeven — het kiezersregister heeft al een wachtwoordkolom, dus gegenereerde wachtwoorden zouden een tweede antwoord op dezelfde vraag zijn. Verwijder de kolom, of zet het genereren uit.",
                    },
                    "no-characters": {
                        lead: "Wachtwoorden zonder tekens",
                        text: "Wachtwoorden zonder tekens — gegenereerde wachtwoorden hebben minstens één soort teken nodig om uit te bestaan.",
                    },
                    "no-seed": {
                        lead: "Wachtwoorden zonder seed",
                        text: "Wachtwoorden zonder seed — de seed zorgt ervoor dat een nieuwe build dezelfde wachtwoorden oplevert in plaats van nieuwe.",
                    },
                },
                plan: {
                    "not-a-plan": {
                        lead: "Geen verkiezingsplan",
                        text: "Geen verkiezingsplan — het bestand kon niet als plan worden gelezen: {{error}}",
                    },
                    "saved-by-newer-version": {
                        lead: "Opgeslagen door een nieuwere versie",
                        text: "Opgeslagen door een nieuwere versie — {{saved}} tegenover {{supported}}. Als je het hier opent, gaat stilzwijgend verloren wat die versie heeft toegevoegd.",
                    },
                    "unreadable": {
                        lead: "Plan onleesbaar",
                        text: "Plan onleesbaar — {{error}}",
                    },
                },
                schedule: {
                    "closes-before-opens": {
                        lead: "Sluit voordat het opent",
                        text: "Sluit voordat het opent — het stemmen zou nooit open zijn.",
                    },
                    "crosses-daylight-saving": {
                        lead: "Valt over een klokwisseling",
                        text: "Valt over een klokwisseling — de periode is een uur langer of korter dan de tijden doen vermoeden.",
                    },
                    "key-ceremony-not-first": {
                        lead: "Sleutelceremonie te laat",
                        text: "Sleutelceremonie te laat — de verkiezingssleutel moet bestaan voordat er een stem mee kan worden versleuteld.",
                    },
                    "tally-ceremony-too-early": {
                        lead: "Telceremonie te vroeg",
                        text: "Telceremonie te vroeg — ze zou stemmen tellen die nog niet zijn uitgebracht.",
                    },
                    "window-incomplete": {
                        lead: "Stemperiode onvolledig",
                        text: "Stemperiode onvolledig — de periode moet handmatig worden geopend of gesloten in het Beheerportaal.",
                    },
                },
                threshold: {
                    "above-trustees": {
                        lead: "Drempel te hoog",
                        text: "Drempel te hoog — {{threshold}} van de {{trustees}} sleutelhouders zijn vereist, wat niet haalbaar is. De sleutel zou worden gegenereerd en de uitslag zou nooit kunnen worden ontsleuteld.",
                    },
                    "one": {
                        lead: "Drempel van één",
                        text: "Drempel van één — elke sleutelhouder kan de telling in zijn eentje openen, wat de lijst ook zegt — dezelfde garantie als één sleutelhouder, en helemaal geen tegen die persoon.",
                    },
                    "zero": {
                        lead: "Drempel van nul",
                        text: "Drempel van nul — de telling kan zonder enige sleutelhouder worden geopend.",
                    },
                },
                trustee: {
                    "email-malformed": {
                        lead: "Geen e-mailadres",
                        text: "Geen e-mailadres — '{{email}}' ziet er niet uit als een e-mailadres.",
                    },
                    "no-email": {
                        lead: "Sleutelhouder zonder adres",
                        text: "Sleutelhouder zonder adres — zo wordt die uitgenodigd voor de sleutelceremonie.",
                    },
                    "no-name": {
                        lead: "Sleutelhouder zonder naam",
                        text: "Sleutelhouder zonder naam — de importeur koppelt de leden van de sleutelceremonie op naam aan de sleutelhouders die al in de tenant zijn ingericht, en een lege naam wordt stilzwijgend een lid dat niet bestaat.",
                    },
                },
                trustees: {
                    "none": {
                        lead: "Geen sleutelhouders",
                        text: "Geen sleutelhouders — een verkiezingssleutel moet door minstens twee mensen worden beheerd. Met één kan die persoon elk stembiljet in zijn eentje ontsleutelen, en dat is juist wat de versleuteling moet voorkomen.",
                    },
                    "only-one": {
                        lead: "Slechts één sleutelhouder",
                        text: "Slechts één sleutelhouder — een verkiezingssleutel moet door minstens twee mensen worden beheerd. Met één kan die persoon elk stembiljet in zijn eentje ontsleutelen, en dat is juist wat de versleuteling moet voorkomen.",
                    },
                },
                voter: {
                    "area-unknown": {
                        lead: "Gebied van kiezer onbekend",
                        text: "Gebied van kiezer onbekend — niets heet '{{area}}'. Kiezers worden op naam aan hun gebied gekoppeld, dus deze kiezer zou geen stembiljet krijgen. Kopieer de naam van het gebied in plaats van hem opnieuw te typen.",
                    },
                    "duplicate-username": {
                        lead: "Gebruikersnaam twee keer gebruikt",
                        text: "Gebruikersnaam twee keer gebruikt — '{{username}}' staat ook op rij {{first}}. Twee kiezers met dezelfde gebruikersnaam worden één account, en deze zou de andere zonder melding vervangen.",
                    },
                    "no-area": {
                        lead: "Kiezer zonder gebied",
                        text: "Kiezer zonder gebied — het gebied bepaalt welk stembiljet een kiezer krijgt, en de build weigert een registerrij zonder gebied.",
                    },
                    "no-username": {
                        lead: "Kiezer zonder gebruikersnaam",
                        text: "Kiezer zonder gebruikersnaam — daarmee logt de kiezer in en daarvan wordt het account afgeleid.",
                    },
                },
                voters: {
                    "duplicate-column": {
                        lead: "Kolom twee keer opgegeven",
                        text: "Kolom twee keer opgegeven — twee kolommen stellen allebei '{{column}}' in. Houd er maar één.",
                    },
                    "unreadable-row": {
                        lead: "Rij onleesbaar",
                        text: "Rij onleesbaar — rij {{row}} kon niet worden gelezen: {{reason}}",
                    },
                    "vote-weight-misspelled": {
                        lead: "Kolom stemgewicht verkeerd gespeld",
                        text: "Kolom stemgewicht verkeerd gespeld — '{{column}}' wordt niet herkend. De kolom heet precies '{{expected}}', anders telt elke kiezer met gewicht 1.",
                    },
                    "vote-weight-not-a-number": {
                        lead: "Stemgewicht is geen getal",
                        text: "Stemgewicht is geen getal — '{{value}}' op rij {{row}} moet een geheel getal tussen 1 en {{max}} zijn.",
                    },
                    "vote-weight-out-of-range": {
                        lead: "Stemgewicht buiten bereik",
                        text: "Stemgewicht buiten bereik — {{value}} op rij {{row}} moet tussen {{min}} en {{max}} liggen.",
                    },
                },
            },
        },
    },
}

export default dutchTranslation
