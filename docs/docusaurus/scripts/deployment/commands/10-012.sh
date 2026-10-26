# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
k3s kubectl -n step-infra exec deploy/postgres -- psql -U postgres -d keycloak -tA \
  -c "select r.name, r.ssl_required, r.sso_idle_timeout, r.events_enabled,
        (select a.value from realm_attribute a where a.realm_id = r.id
          and a.name = 'bruteForceProtected'),
        (select a.value from realm_attribute a where a.realm_id = r.id
          and a.name = 'failureFactor'),
        (select l.value from realm_events_listeners l where l.realm_id = r.id
          and l.value = 'custom-event-listener')
      from realm r where r.name like '%-event-%'"
