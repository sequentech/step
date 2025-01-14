use anyhow::{anyhow, Context, Result};
use csv::ReaderBuilder;
use rand::Rng;
use rusqlite::{params, Connection};

use crate::types::user::User;

pub fn load_users(csv_path: &str) -> Result<(), anyhow::Error> {
    // 1. Define the CSV path

    // 3. Open the CSV with headers
    let mut rdr = ReaderBuilder::new()
        .has_headers(true) // if you have a header row in the CSV
        .from_path(csv_path)
        .context("Error opening the CSV file")?;

    // 4. Connect to SQLite
    let conn = Connection::open("voters.db")
        .context("Failed to open or create 'voters.db'")
        .context("Error creating sqlite connection")?;

    // 5. Create table if not exists (all columns as TEXT for simplicity)
    //    Adjust columns/types as needed for your data (BOOLEAN, INTEGER, etc.).
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS voters (
            id TEXT PRIMARY KEY,
            first_name TEXT,
            last_name TEXT,
            middle_name TEXT,
            date_of_birth TEXT,
            embassy TEXT,
            country TEXT,
            id_card_number TEXT,
            id_card_type TEXT
        );
        "#,
    )
    .context("Failed to create 'voters' table")?;

    // 6. Insert records
    for record_result in rdr.records() {
        let record = record_result
            .context("Failed to read record from CSV")
            .context("Failed to read record from CSV")?;

        // Extract each column by index, in the same order as your CSV
        let id = record.get(0).unwrap_or_default().trim().to_string();
        let first_name = record.get(4).unwrap_or_default().trim().to_string();
        let last_name = record.get(5).unwrap_or_default().trim().to_string();
        let middle_ame = record.get(9).unwrap_or_default().trim().to_string();
        let date_of_birth = record.get(12).unwrap_or_default().trim().to_string();
        let embassy = record.get(14).unwrap_or_default().trim().to_string();
        let country = record.get(15).unwrap_or_default().trim().to_string();
        let id_card_number = record.get(18).unwrap_or_default().trim().to_string();
        let id_card_type = record.get(8).unwrap_or_default().trim().to_string();

        conn.execute(
            r#"
            INSERT OR IGNORE INTO voters (
                id, first_name, last_name, middle_name, date_of_birth,
                embassy, country, id_card_number, id_card_type
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                ?8, ?9
            )
            "#,
            params![
                id,
                first_name,
                last_name,
                middle_ame,
                date_of_birth,
                embassy,
                country,
                id_card_number,
                id_card_type
            ],
        )
        .with_context(|| format!("Failed to insert row for id '{}'", id))
        .context(format!("Failed to insert row for id '{}'", id))?;
    }

    // On success
    Ok(())
}

pub fn get_users_from_db() -> anyhow::Result<Vec<User>> {
    let conn = rusqlite::Connection::open("voters.db")?;

    let mut stmt = conn.prepare(
        r#"
        SELECT 
            id, first_name, last_name,
            middle_name, embassy, country, id_card_number, id_card_type, date_of_birth
        FROM voters
        "#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(User {
            id: row.get::<_, String>(0)?,
            first_name: row.get::<_, String>(1)?,
            last_name: row.get::<_, String>(2)?,
            middle_name: row.get::<_, String>(3)?,
            embassy: row.get::<_, String>(4)?,
            country: row.get::<_, String>(5)?,
            id_card_number: row.get::<_, String>(6)?,
            id_card_type: row.get::<_, String>(7)?,
            date_of_birth: row.get::<_, String>(8)?,
        })
    })?;

    let mut users = Vec::new();
    for user_res in rows {
        users.push(user_res?);
    }

    Ok(users)
}

// Example function: returns a random user from the given country
pub fn random_user_by_country(country: &str) -> Result<Option<User>> {
    let conn = Connection::open("voters.db")?;
    let mut stmt = conn.prepare(
        " SELECT 
           id, first_name, last_name,
            middle_name, embassy, country, id_card_number, id_card_type, date_of_birth
         FROM voters
         WHERE country = ?1",
    )?;

    let rows = stmt.query_map([country], |row| {
        Ok(User {
            id: row.get::<_, String>(0)?,
            first_name: row.get::<_, String>(1)?,
            last_name: row.get::<_, String>(2)?,
            middle_name: row.get::<_, String>(3)?,
            embassy: row.get::<_, String>(4)?,
            country: row.get::<_, String>(5)?,
            id_card_number: row.get::<_, String>(6)?,
            id_card_type: row.get::<_, String>(7)?,
            date_of_birth: row.get::<_, String>(8)?,
        })
    })?;

    let users: Vec<_> = rows.filter_map(|r| r.ok()).collect();
    if users.is_empty() {
        Ok(None)
    } else {
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..users.len());
        Ok(Some(users[idx].clone()))
    }
}
