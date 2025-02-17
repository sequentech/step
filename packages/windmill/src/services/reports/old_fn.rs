#[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data_batch_11111111111111111111111111OLD_BAD_OLD_BAD_OLD_BAD_OLD_BAD_OLD_BAD_OLD_BAD_OLD_BAD_OLD_BAD_OLD_BAD_OLD_BAD_(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        offset: i64,
        limit: i64,
    ) -> Result<Self::UserData> {
        let mut user_data = self
            .prepare_user_data_common(hasura_transaction, keycloak_transaction)
            .await?;
        let event_realm_name =
            get_event_realm(&self.get_tenant_id(), &self.get_election_event_id());
        let tenant_realm_name = get_tenant_realm(&self.get_tenant_id());
        // This is used to fill the user data.

        let Some(election_id) = self.get_election_id() else {
            return Err(anyhow!("Empty election_id"));
        };
        info!("Preparing data of audit logs report for election_id: {election_id}");
        let election: Election = get_election_by_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        .ok_or(anyhow!(
            "No election found for the given election id: {election_id}"
        ))?;

        // To filter log entries by election we´ll prepare a list with the user Ids that belong to this election.
        // To get the voter_ids related to this election, we need the areas.
        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .with_context(|| "Error at get_areas_by_election_id")?;

        if election_areas.is_empty() {
            return Err(anyhow!(
                "No areas found for the given election id: {election_id}"
            ));
        }

        // We need the permission_label to filter the logs by Admin users
        // This field is not mandatory so if it´s not there the admin users simply won´t be reported.
        let perm_lbl_attributes: Option<HashMap<String, String>> = match election.permission_label {
            Some(permission_label) => Some(HashMap::from([(
                "permission_labels".to_string(),
                permission_label,
            )])),
            None => {
                warn!("No permission_label found for the election, admin users won't be reported");
                None
            }
        };

        let max_batch_size = PgConfig::from_env()?.default_sql_batch_size;
        let admins_filter = ListUsersFilter {
            tenant_id: self.get_tenant_id(),
            realm: tenant_realm_name.clone(),
            attributes: perm_lbl_attributes.clone(),
            limit: Some(max_batch_size),
            ..Default::default() // Fill the options that are left to None
        };

        // Fill election_admin_ids with the Admins that matches the election_permission_label
        let mut election_admin_ids: HashSet<String> = HashSet::new();
        let mut admins_offset: i32 = 0;
        loop {
            let (admins, total_count) = list_users(
                &hasura_transaction,
                &keycloak_transaction,
                ListUsersFilter {
                    offset: Some(admins_offset),
                    ..admins_filter.clone()
                },
            )
            .await
            .with_context(|| "Failed to fetch list_users")?;

            admins_offset += total_count;
            for admin in admins {
                let Some(admin_id) = admin.id.clone() else {
                    info!("Unexpected, admin user with no id {:?}", admin);
                    continue;
                };
                election_admin_ids.insert(admin_id);
            }
            if total_count < max_batch_size {
                break;
            }
        }

        let voters_filter = ListUsersFilter {
            tenant_id: self.get_tenant_id(),
            realm: event_realm_name.clone(),
            election_event_id: Some(String::from(&self.get_election_event_id())),
            election_id: Some(election_id.clone()),
            limit: Some(max_batch_size),
            area_id: None,        // To fill below
            ..Default::default()  // Fill the options that are left to None
        };

        let mut voters_offset: i32 = 0;
        let mut election_user_ids: HashSet<String> = HashSet::new();
        // Loop over each area to fill election_user_ids with the voters
        for area in election_areas.iter() {
            loop {
                let (users, total_count) = list_users(
                    &hasura_transaction,
                    &keycloak_transaction,
                    ListUsersFilter {
                        area_id: Some(area.id.clone()),
                        offset: Some(voters_offset),
                        ..voters_filter.clone()
                    },
                )
                .await
                .with_context(|| "Failed to fetch list_users")?;

                voters_offset += total_count;
                for user in users {
                    election_user_ids.insert(user.id.unwrap_or_default());
                }
                if total_count < max_batch_size {
                    break;
                }
            }
        }

        // Fetch list of audit logs
        let mut sequences: Vec<AuditLogEntry> = Vec::new();
        let mut electoral_logs: DataList<ElectoralLogRow> = DataList {
            items: vec![],
            total: TotalAggregate {
                aggregate: Aggregate { count: 0 },
            },
        };

        let mut offset: i64 = 0;
        loop {
            let electoral_logs_batch = list_electoral_log(GetElectoralLogBody {
                tenant_id: String::from(&self.get_tenant_id()),
                election_event_id: String::from(&self.ids.election_event_id),
                limit: Some(IMMUDB_ROWS_LIMIT as i64),
                offset: Some(offset),
                filter: None,
                order_by: None,
            })
            .await
            .with_context(|| "Error in fetching list of electoral logs")?;

            let batch_size = electoral_logs_batch.items.len();
            offset += batch_size as i64;
            electoral_logs.items.extend(electoral_logs_batch.items);
            electoral_logs.total.aggregate.count = electoral_logs_batch.total.aggregate.count;
            if batch_size < IMMUDB_ROWS_LIMIT {
                break;
            }
        }

        // iterate on list of audit logs and create array
        for item in &electoral_logs.items {
            // Discard the log entries that are not related to this election
            let userkind = match &item.user_id {
                Some(user_id) if election_admin_ids.contains(user_id) => "Admin".to_string(),
                Some(user_id) if election_user_ids.contains(user_id) => "Voter".to_string(),
                Some(_) => continue, // Some user_id not belonging to this election
                None => continue,    // There is no user_id, ignore log entry
            };

            let created_datetime: DateTime<Local> = if let Ok(created_datetime_parsed) =
                ISO8601::timestamp_secs_utc_to_date_opt(item.created)
            {
                created_datetime_parsed
            } else {
                return Err(anyhow!(
                    "Invalid item created timestamp: {:?}",
                    item.created
                ));
            };
            let formatted_datetime: String = created_datetime.to_rfc3339();

            // Set default username if user_id is None
            let username = item
                .username
                .clone()
                .map(|user| {
                    if user == "null" {
                        "-".to_string()
                    } else {
                        user
                    }
                })
                .unwrap_or_else(|| "-".to_string());

            // Map fields from `ElectoralLogRow` to `AuditLogEntry`
            let audit_log_entry = AuditLogEntry {
                number: item.id, // Increment number for each item
                datetime: formatted_datetime,
                username,
                userkind,
                activity: item
                    .statement_head_data()
                    .map(|head| head.description.clone())
                    .unwrap_or("-".to_string()),
            };

            // Push the constructed `AuditLogEntry` to the sequences array
            sequences.push(audit_log_entry);
        }

        user_data.sequences = sequences;

        Ok(user_data)
    }