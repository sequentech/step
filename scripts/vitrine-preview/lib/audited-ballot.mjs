// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// An auditable ballot made by the voting portal for the fixture election: the
// JSON its Audit screen shows, with the plaintext and the randomness of the
// voter's choice (Maria Santos) inside its contests. Step's backend does not
// publish audited ballots; the stand-in holds this one so the preview's ballot
// tracker has an audited record to find, check and decode.

export const AUDITED_BALLOT = {
  "version": 2,
  "issue_date": "30/10/2026",
  "config": {
    "id": "council-style",
    "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
    "election_event_id": "demo-2026",
    "election_id": "council",
    "num_allowed_revotes": 0,
    "description": "Municipal council election",
    "public_key": {
      "public_key": "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4",
      "is_demo": false
    },
    "area_id": "5e1c2a3b-7f64-4b08-9d51-6c3a0e8f2b17",
    "area_presentation": {
      "allow_early_voting": "no_early_voting"
    },
    "contests": [
      {
        "id": "council-contest-1",
        "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
        "election_event_id": "demo-2026",
        "election_id": "council",
        "name": "Who should chair the municipal council?",
        "name_i18n": null,
        "description": "Choose one person. You can change your vote until voting closes.",
        "description_i18n": null,
        "alias": null,
        "alias_i18n": null,
        "max_votes": 1,
        "min_votes": 0,
        "winning_candidates_num": 1,
        "voting_type": "first-past-the-post",
        "counting_algorithm": "plurality-at-large",
        "is_encrypted": true,
        "is_acclaimed": null,
        "candidates": [
          {
            "id": "council-ana-ferrer",
            "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "election_event_id": "demo-2026",
            "election_id": "council",
            "contest_id": "council-contest-1",
            "name": "Ana Ferrer",
            "name_i18n": null,
            "description": "Teacher and former deputy mayor.",
            "description_i18n": null,
            "alias": null,
            "alias_i18n": null,
            "candidate_type": "A",
            "presentation": {
              "i18n": null,
              "is_explicit_invalid": false,
              "is_explicit_blank": null,
              "is_disabled": false,
              "is_category_list": false,
              "invalid_vote_position": null,
              "is_write_in": false,
              "sort_order": 0,
              "urls": null,
              "subtype": null
            },
            "annotations": null
          },
          {
            "id": "council-jon-etxeberria",
            "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "election_event_id": "demo-2026",
            "election_id": "council",
            "contest_id": "council-contest-1",
            "name": "Jon Etxeberria",
            "name_i18n": null,
            "description": "Civil engineer, city planner.",
            "description_i18n": null,
            "alias": null,
            "alias_i18n": null,
            "candidate_type": "A",
            "presentation": {
              "i18n": null,
              "is_explicit_invalid": false,
              "is_explicit_blank": null,
              "is_disabled": false,
              "is_category_list": false,
              "invalid_vote_position": null,
              "is_write_in": false,
              "sort_order": 1,
              "urls": null,
              "subtype": null
            },
            "annotations": null
          },
          {
            "id": "council-maria-santos",
            "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "election_event_id": "demo-2026",
            "election_id": "council",
            "contest_id": "council-contest-1",
            "name": "Maria Santos",
            "name_i18n": null,
            "description": "Hospital director.",
            "description_i18n": null,
            "alias": null,
            "alias_i18n": null,
            "candidate_type": "A",
            "presentation": {
              "i18n": null,
              "is_explicit_invalid": false,
              "is_explicit_blank": null,
              "is_disabled": false,
              "is_category_list": false,
              "invalid_vote_position": null,
              "is_write_in": false,
              "sort_order": 2,
              "urls": null,
              "subtype": null
            },
            "annotations": null
          },
          {
            "id": "council-pieter-de-vries",
            "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "election_event_id": "demo-2026",
            "election_id": "council",
            "contest_id": "council-contest-1",
            "name": "Pieter de Vries",
            "name_i18n": null,
            "description": "Cooperative founder.",
            "description_i18n": null,
            "alias": null,
            "alias_i18n": null,
            "candidate_type": "A",
            "presentation": {
              "i18n": null,
              "is_explicit_invalid": false,
              "is_explicit_blank": null,
              "is_disabled": false,
              "is_category_list": false,
              "invalid_vote_position": null,
              "is_write_in": false,
              "sort_order": 3,
              "urls": null,
              "subtype": null
            },
            "annotations": null
          }
        ],
        "presentation": {
          "i18n": {
            "en": {
              "name": "Who should chair the municipal council?",
              "description": "Choose one person. You can change your vote until voting closes."
            }
          },
          "allow_writeins": false,
          "base32_writeins": null,
          "invalid_vote_policy": "allowed",
          "under_vote_policy": null,
          "blank_vote_policy": "allowed",
          "over_vote_policy": null,
          "duplicated_rank_policy": null,
          "preference_gaps_policy": null,
          "pagination_policy": null,
          "cumulative_number_of_checkboxes": null,
          "shuffle_categories": null,
          "shuffle_category_list": null,
          "show_points": null,
          "enable_checkable_lists": null,
          "collapsible_lists": null,
          "candidates_order": null,
          "candidates_selection_policy": null,
          "candidates_icon_checkbox_policy": null,
          "max_selections_per_type": null,
          "types_presentation": null,
          "sort_order": null,
          "columns": null
        },
        "created_at": null,
        "annotations": null,
        "tie_breaking_policy": null
      }
    ],
    "election_event_presentation": {
      "i18n": {
        "en": {
          "description": "Vote online in the municipal elections.",
          "name": "Municipal elections 2026"
        },
        "es": {
          "description": "Vote por Internet en las elecciones municipales.",
          "name": "Elecciones municipales 2026"
        }
      },
      "materials": null,
      "language_conf": {
        "enabled_language_codes": [
          "en",
          "es"
        ],
        "default_language_code": "en",
        "language_detection_policy": null
      },
      "logo_url": null,
      "redirect_finish_url": null,
      "kiosk_redirect_finish_url": null,
      "css": null,
      "skip_election_list": false,
      "show_user_profile": true,
      "show_cast_vote_logs": "hide-logs-tab",
      "elections_order": null,
      "voting_portal_countdown_policy": null,
      "custom_urls": null,
      "keys_ceremony_policy": null,
      "contest_encryption_policy": null,
      "decoded_ballot_inclusion_policy": null,
      "locked_down": null,
      "publish_policy": null,
      "enrollment": null,
      "otp": null,
      "voter_signing_policy": null,
      "voter_certificate_policy": null,
      "weighted_voting_policy": null,
      "ceremonies_policy": null,
      "delegated_voting_policy": null,
      "results_website": null,
      "voting_portal_datetime_format": null
    },
    "election_presentation": {
      "i18n": {
        "en": {
          "description": "Choose who chairs the municipal council.",
          "name": "Municipal council election"
        },
        "es": {
          "description": "Choose who chairs the municipal council.",
          "name": "Municipal council election"
        }
      },
      "dates": null,
      "language_conf": null,
      "contests_order": null,
      "audit_button_cfg": null,
      "sort_order": null,
      "cast_vote_confirm": null,
      "cast_vote_gold_level": null,
      "start_screen_title_policy": null,
      "is_grace_priod": null,
      "grace_period_policy": null,
      "grace_period_secs": null,
      "init_report": null,
      "manual_start_voting_period": null,
      "voting_period_end": null,
      "tally": null,
      "initialization_report_policy": null,
      "security_confirmation_policy": null,
      "consolidated_report_policy": null,
      "decline_to_vote_policy": null,
      "voting_screen_back_policy": null,
      "css": null,
      "blank_ballots_policy": null
    },
    "election_dates": {
      "first_started_at": null,
      "last_started_at": null,
      "first_paused_at": null,
      "last_paused_at": null,
      "first_stopped_at": null,
      "last_stopped_at": null,
      "scheduled_event_dates": null
    },
    "election_event_annotations": null,
    "election_annotations": null,
    "area_annotations": null,
    "multi_contest_encoding_mode": null
  },
  "contests": [
    "EQAAAGNvdW5jaWwtY29udGVzdC0xoJICiW+kq1AJCIwggiHibul950Ktgy45KzPr5KHU3nb2ubKVsIGZ7fvcvE8CtwXad5KYh1Y81q6Rs8L/397ZFwEIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAOZxgM4xRRauD/3XqJj0snxfPm5ao0jrJ77ap3KrGRoHVremp5V8IdBYEBKF/QnKcMya1LX9Ukmqz1l3UILhVnBcCRqYIrgaH5DQtLunpPKIDieusUUIEfDZSEeI2lSjB2aVh3QDnw7R8HivCgdao1G1Se7T4vdSkT2bUmcFXu4M"
  ],
  "ballot_hash": "9020e3849a0ad24eb0f322e1cd400b60618ecd1830dda699448a968cfc9ca5f7",
  "voter_signing_pk": null,
  "voter_ballot_signature": null
};
