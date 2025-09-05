// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
module.exports = {
  docs: [
        'system_introduction/system_introduction',
    {
      type: 'category',
      label: 'Election Managers',
      items: [
        'admin_portal/election_management',
        {
          type: 'category',
          label: 'Admin Portal',
          items: [
            'admin_portal/Reference/basic_navigation',
          {
            type: 'category',
            label: 'Election Event',
            items: [
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Dashboard/election_management_election_event_dashboard',
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Monitoring/election_management_election_event_monitoring',
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Data/election_management_election_event_data',
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Localization/election_management_election_event_localization',
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Voters/election_management_election_event_voters',
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Areas/election_management_election_event_areas',
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Keys/election_management_election_event_keys',
              {
                type: 'category',
                label: 'Tally',
                items: [
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Tally/election_management_election_event_tally',
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Tally/election_management_election_event_transmission',

                ],
              },
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Publish/election_management_election_event_publish',
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Tasks/election_management_election_event_tasks',
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Logs/election_management_election_event_logs',
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Scheduled-Events/election_management_election_event_scheduled_events',
              {
                type: 'category',
                label: 'Reports and Templates',
                items: [
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Reports/election_management_election_event_reports',
              'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Reports/election_management_election_event_templates',

                ],
              },
             'admin_portal/Reference/User-Manual/Election-Management/Election-Event/Approvals/election_management_election_event_approvals',
            ],
          },
          {
            type: 'category',
            label: 'Election', //TODO - add content
            items: [
              'admin_portal/Reference/User-Manual/Election-Management/Election/Dashboard/election_management_election_dashboard',
              'admin_portal/Reference/User-Manual/Election-Management/Election/Monitoring/election_management_election_monitoring',
              'admin_portal/Reference/User-Manual/Election-Management/Election/Data/election_management_election_data',
              'admin_portal/Reference/User-Manual/Election-Management/Election/Voters/election_management_election_voters',
              'admin_portal/Reference/User-Manual/Election-Management/Election/Publish/election_management_election_publish',
              'admin_portal/Reference/User-Manual/Election-Management/Election/Approvals/election_management_election_approvals',
            ],
          },
          {
            type: 'category',
            label: 'Contest', //TODO - add content
            items: [
              'admin_portal/Reference/User-Manual/Election-Management/Contest/Data/election_management_contest_data',
              'admin_portal/Reference/User-Manual/Election-Management/Contest/Tally-Sheets/election_management_contest_tally_sheets',
            ],
          },
            {
              type: 'category',
              label: 'Candidate', //TODO - add content
              items: [
                'admin_portal/Reference/User-Manual/Election-Management/Candidate/Data/election_management_candidate_data',
              ],
            },
          ],
        },
        {
          type: 'category',
          label: 'Tutorials',
          items: [
          'admin_portal/Tutorials/admin_portal_tutorials_setting-up-your-first-election',
          'admin_portal/Tutorials/admin_portal_tutorials_permission-labels',
          'admin_portal/Tutorials/admin_portal_tutorials_setting-up-an-automated-election',
          'admin_portal/Tutorials/Reports-and-Templates/reports_and_templates',
          'admin_portal/Tutorials/Reports-and-Templates/value_to_template_association',
          ],
        },
      ],
    },
    {
      type: 'category',
      label: 'Voters',
      items: [
        'voting_portal/voting_portal',
        {
          type: 'category',
          label: 'Tutorials',
          items: [
            'voting_portal/Tutorials/voting_guide',
            'voting_portal/Tutorials/voter_locate_ballot',
            'voting_portal/Tutorials/voter_audit_ballot'
          ],
        },
      ],
    },
    {
      type: 'category',
      label: 'Election Auditors',
      items: [
        'ballot_verifier/ballot_verifier',
        'election_verifier/election_verifier',
      ],
    },
    {
      type: 'category',
      label: 'Developers',
      items: [
        'developers/api_reference',
        'developers/developers_cli',
        'developers/add_new_language',
        {
          type: 'category',
          label: 'Admin Portal',
          items: [
            'developers/Admin-Portal/developers_admin_portal',
            'developers/Admin-Portal/load-test_admin-portal',
          ],
        },
        {
          type: 'category',
          label: 'Voting Portal',
          items: [
            'developers/Voting-Portal/developers_voting_portal',
            'developers/Voting-Portal/cast_vote_errors',
            'developers/Voting-Portal/demo_mode',
          ],
        },
        {
          type: 'category',
          label: 'Keycloak',
          items: ['developers/Keycloak/developers_keycloak'],
        },
        {
          type: 'category',
          label: 'Velvet',
          items: ['developers/Velvet/developers_velvet'],
        },
        {
          type: 'category',
          label: 'Windmill',
          items: ['developers/Windmill/developers_windmill'],
        },
        {
          type: 'category',
          label: 'Braid',
          items: ['developers/Braid/braid_trustees_configuration'],
        },
        'cli/Tutorials/Load-Testing/load_testing'
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      "link": {
        "type": "doc",
        "id": 'reference/reference'
      },
      items:[
        'reference/glossary',
        'reference/product_lifecycle_and_release_cadence',
        {
          type: 'category',
          label: 'Tally Deep Dive',
          "link": {
            "type": "doc",
            "id": 'reference/Tally Deep Dive/tally_deep_dive',
          },
          items: [
            'reference/Tally Deep Dive/tally_benchmarks',
            'reference/Tally Deep Dive/tally_dump_votes',
            'reference/Tally Deep Dive/tally_mixnet',
            'reference/Tally Deep Dive/tally_decrypt',
            'reference/Tally Deep Dive/tally_consolidate',
            'reference/Tally Deep Dive/tally_results',
          ],
        },

        {
          type: 'category',
          label: 'Cryptography and Security',
          "link": {
            "type": "doc",
            "id": 'reference/cryptography_security/cryptography/cryptography',
          },
          items: [
            {
              type: 'category',
              label: 'Cryptography',
              items: [
                'reference/cryptography_security/cryptography/mixnet',
                'reference/cryptography_security/cryptography/proofs',

              ],
            },
            {
              type: 'category',
              label: 'Security',
              items: [
                'reference/cryptography_security/security/permissions',
                'reference/cryptography_security/security/threat_model'
              ],
            },
          ],
        },

      ],
    },
  ],
};
