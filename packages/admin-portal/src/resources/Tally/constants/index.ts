export const JSON_MOCK = [
    {
        description: "Senaduria Ballots",
        presentation: {
            theme: "default",
            urls: [],
            theme_css:
                ".basic-booth-layout .logo-img, [av-booth] .logo-img { max-height: 60px; max-width: 400px; width: auto; } [avb-voting-step] .step-number-selected { background: #042363 !important; color: #FFFFFF; !important;} [avb-voting-step] .step-number-unselected { color: #042363 !important; opacity: 50%; stroke-opacity: 50%; border: 1px solid #042363 !important; } .btn-success-action, .btn.btn-success { border: 0; background-color: #042363 !important;; color: #FFFFFF!important; !important;} .btn-opt-action { color: #042363 !important;; background-color: #FFFFFF!important; border: 1px solid #042363 !important;; !important;} .btn-warning-solid { background-color: #042363 color: #FFFFFF; border: none; } #audit-ballot-btn { display: none; }   .opt .answer-links .view-link, [avb-simultaneous-question-answer-v2] .opt .answer-links .view-link, [avb-simultaneous-questions-v2-screen] .opt .answer-links .view-link { border: none; background: 0 0; text-decoration: none; color: #042363; font-size: 14px;}",
            extra_options: {
                allow_voting_end_graceful_period: true,
                disable__demo_voting_booth: false,
                disable__public_home: false,
                disable_voting_booth_audit_ballot: true,
                disable__election_chooser_screen: false,
                success_screen__hide_ballot_tracker: false,
                success_screen__hide_qr_code: false,
                success_screen__hide_download_ballot_ticket: false,
                success_screen__redirect__url: "https://www.ine.mx",
                success_screen__redirect_to_login: false,
                success_screen__ballot_ticket__logo_url:
                    "https://www.jotform.com/uploads/INE/ine.jpg",
                success_screen__ballot_ticket__logo_header: " ",
                review_screen__split_cast_edit: true,
            },
        },
    },
]
