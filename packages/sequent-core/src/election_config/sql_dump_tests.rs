// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::sql_dump::{read_mysql_dump, SqlValue};

#[test]
fn reads_original_bytes_in_binary_literals_and_escaped_text() {
    let sql = b"CREATE TABLE `example` (`id` varchar(8), `picture` longblob, `name` varchar(80));\nINSERT INTO `example` VALUES ('001',_binary '\xff\\\'x', 'O\\\'Brien'),('002',NULL,'two; -- characters');";
    let dump = read_mysql_dump(sql).expect("valid data-only dump");
    let table = dump.tables.get("example").expect("declared table");
    assert_eq!(table.columns, vec!["id", "picture", "name"]);
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0][0], SqlValue::Bytes(b"001".to_vec()));
    assert_eq!(table.rows[0][1], SqlValue::Bytes(vec![0xff, b'\'', b'x']));
    assert_eq!(table.rows[0][2], SqlValue::Bytes(b"O'Brien".to_vec()));
    assert_eq!(table.rows[1][1], SqlValue::Null);
    assert_eq!(
        table.rows[1][2],
        SqlValue::Bytes(b"two; -- characters".to_vec())
    );
}

#[test]
fn understands_mysql_dump_envelope_without_executing_statements() {
    let sql = br#"/*!40101 SET @OLD_CHARACTER_SET_CLIENT=@@CHARACTER_SET_CLIENT */;
DROP TABLE IF EXISTS `example`;
CREATE TABLE `example` (`id` varchar(8) NOT NULL, PRIMARY KEY (`id`)) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
LOCK TABLES `example` WRITE;
/*!40000 ALTER TABLE `example` DISABLE KEYS */;
INSERT INTO `example` VALUES ('001');
/*!40000 ALTER TABLE `example` ENABLE KEYS */;
UNLOCK TABLES;
"#;
    let dump = read_mysql_dump(sql).expect("known inert dump envelope");
    assert_eq!(dump.tables["example"].rows.len(), 1);
}

#[test]
fn rejects_executable_sql_with_a_location_and_no_source_values() {
    let sql = b"CREATE TABLE `example` (`id` varchar(8));\nSELECT 'private-value' INTO OUTFILE '/tmp/leak';";
    let error =
        read_mysql_dump(sql).expect_err("executable SQL must be refused");
    assert_eq!(error.line, 2);
    assert_eq!(error.code, "sql.unsupported-statement");
    assert!(!error.to_string().contains("private-value"));
    assert!(!error.to_string().contains("/tmp/leak"));
}

#[test]
fn rejects_an_insert_width_mismatch_without_disclosing_the_record() {
    let sql = b"CREATE TABLE `example` (`id` varchar(8), `name` varchar(80));\nINSERT INTO `example` VALUES ('001','private-name','extra');";
    let error = read_mysql_dump(sql).expect_err("column counts must match");
    assert_eq!(error.code, "sql.row-width");
    assert!(!error.to_string().contains("private-name"));
}

#[test]
fn does_not_accept_a_duplicate_table_definition() {
    let sql =
        b"CREATE TABLE `example` (`id` varchar(8));\nCREATE TABLE `example` (`other` varchar(8));";
    let error = read_mysql_dump(sql).expect_err("duplicate schema declaration");
    assert_eq!(error.code, "sql.duplicate-table");
}

#[test]
fn active_statements_hidden_in_version_comments_are_refused() {
    let error = read_mysql_dump(
        b"/*!50001 SELECT 'private-value' INTO OUTFILE '/tmp/leak' */;",
    )
    .unwrap_err();
    assert_eq!(error.code, "sql.unsupported-statement");
    assert!(!error.to_string().contains("private-value"));
}
#[test]
fn unknown_set_operations_and_changes_to_literal_escaping_are_refused() {
    assert_eq!(
        read_mysql_dump(b"SET arbitrary_option=1;")
            .unwrap_err()
            .code,
        "sql.unsupported-statement"
    );
    assert_eq!(
        read_mysql_dump(b"SET SQL_MODE='NO_BACKSLASH_ESCAPES';")
            .unwrap_err()
            .code,
        "sql.unsupported-mode"
    );
}
#[test]
fn an_insert_with_no_data_is_not_silently_accepted() {
    assert_eq!(
        read_mysql_dump(
            b"CREATE TABLE `example` (`id` int); INSERT INTO `example` VALUES;"
        )
        .unwrap_err()
        .code,
        "sql.syntax"
    );
}
