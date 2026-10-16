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

#[test]
fn accepts_explicit_column_order_charsets_keys_views_and_mysql_literal_shapes()
{
    let dump = read_mysql_dump(br#"
# dump metadata
-- another comment
/* ordinary comment */
SET NAMES utf8mb4;
SET @OLD_SQL_MODE=@@SQL_MODE, SQL_NOTES=0;
CREATE TABLE sample (id int, label varchar(20) CHARACTER SET latin1, blob longblob, PRIMARY KEY (id), UNIQUE KEY label_key (label), KEY other_key (id)) DEFAULT CHARACTER SET utf8mb4;
INSERT INTO sample (blob,label,id) VALUES (x'00fF','It''s\n\r\t\b\Z\0\%\_',-12.5),(_binary 'a',NULL,2);
CREATE VIEW sample_view AS SELECT id FROM sample;
DROP VIEW IF EXISTS sample_view;
CREATE VIEW sample_view AS SELECT id FROM sample;
"#).unwrap();
    let table = &dump.tables["sample"];
    assert_eq!(table.columns, ["id", "label", "blob"]);
    assert_eq!(table.charset.as_deref(), Some("utf8mb4"));
    assert_eq!(table.column_charsets["label"], "latin1");
    assert_eq!(table.primary_key, ["id"]);
    assert_eq!(
        table.unique_keys,
        vec![vec!["id".to_string()], vec!["label".to_string()]]
    );
    assert_eq!(table.rows[0][0], SqlValue::Bytes(b"-12.5".to_vec()));
    assert_eq!(
        table.rows[0][1],
        SqlValue::Bytes(b"It's\n\r\t\x08\x1a\0\\%\\_".to_vec())
    );
    assert_eq!(table.rows[0][2], SqlValue::Bytes(vec![0, 255]));
    assert_eq!(table.rows[1][1], SqlValue::Null);
    assert!(dump.views.contains("sample_view"));
}

#[test]
fn malformed_dump_syntax_and_executable_expressions_are_rejected_without_values(
) {
    let cases = [
        ("/* never closed", "sql.unterminated-comment"),
        ("/*!50001 SET SQL_NOTES=0", "sql.unterminated-comment"),
        ("/*!50001 /* nested */ */", "sql.nested-comment"),
        ("CREATE TABLE `unfinished", "sql.unterminated-literal"),
        ("CREATE TABLE sample id int", "sql.syntax"),
        ("CREATE TABLE sample (id int", "sql.syntax"),
        ("CREATE TABLE sample ()", "sql.syntax"),
        (
            "CREATE TABLE sample (id int, id text)",
            "sql.duplicate-column",
        ),
        ("CREATE TABLE sample (PRIMARY KEY (id))", "sql.empty-schema"),
        ("CREATE TABLE sample (id int, PRIMARY KEY id)", "sql.syntax"),
        (
            "CREATE TABLE sample (id int, UNIQUE KEY missing)",
            "sql.syntax",
        ),
        (
            "INSERT IGNORE INTO sample VALUES (1)",
            "sql.unsupported-statement",
        ),
        ("INSERT INTO missing VALUES (1)", "sql.undeclared-table"),
        ("DROP TABLE sample", "sql.unsupported-statement"),
        ("UNLOCK other", "sql.unsupported-statement"),
        ("LOCK TABLES missing WRITE", "sql.unsupported-statement"),
        (
            "ALTER TABLE missing ENABLE KEYS",
            "sql.unsupported-statement",
        ),
        ("SET NAMES unsupported", "sql.unsupported-statement"),
        ("SET SQL_MODE=fn()", "sql.unsupported-statement"),
        ("SET SQL_MODE=GLOBAL", "sql.unsupported-statement"),
        ("SET SQL_MODE=SELECT", "sql.unsupported-statement"),
        ("SET SQL_MODE=1,", "sql.unsupported-statement"),
        ("SET SQL_MODE=1, arbitrary=2", "sql.unsupported-statement"),
        (
            "CREATE VIEW v AS UPDATE sample",
            "sql.unsupported-statement",
        ),
        (
            "CREATE VIEW v AS SELECT 1; CREATE VIEW v AS SELECT 2;",
            "sql.duplicate-view",
        ),
    ];
    for (sql, expected) in cases {
        let error = read_mysql_dump(sql.as_bytes()).expect_err(sql);
        assert_eq!(error.code, expected, "{sql}");
        assert!(error.line > 0);
    }
    for (tail, expected) in [
        (
            "INSERT INTO sample (other) VALUES (1)",
            "sql.unknown-column",
        ),
        (
            "INSERT INTO sample (id,id) VALUES (1,2)",
            "sql.duplicate-column",
        ),
        ("INSERT INTO sample () VALUES ()", "sql.syntax"),
        ("INSERT INTO sample SELECT 1", "sql.unsupported-statement"),
        ("INSERT INTO sample VALUES 1", "sql.syntax"),
        ("INSERT INTO sample VALUES (1) junk", "sql.syntax"),
        ("INSERT INTO sample VALUES (1),", "sql.syntax"),
        (
            "INSERT INTO sample VALUES ('private-value",
            "sql.unterminated-literal",
        ),
        (
            "INSERT INTO sample VALUES ('private-value\\",
            "sql.unterminated-literal",
        ),
        ("INSERT INTO sample VALUES (NOW())", "sql.value"),
        ("INSERT INTO sample VALUES (x'0')", "sql.value"),
        ("INSERT INTO sample VALUES (x'gg')", "sql.value"),
        ("INSERT INTO sample VALUES (NULL,NULL)", "sql.row-width"),
        (
            "INSERT INTO sample VALUES (1); DROP TABLE IF EXISTS sample",
            "sql.destructive-statement",
        ),
    ] {
        let sql = format!("CREATE TABLE sample (id text); {tail}");
        let error = read_mysql_dump(sql.as_bytes()).expect_err(tail);
        assert_eq!(error.code, expected, "{tail}");
        assert!(!error.to_string().contains("private-value"));
    }
    assert_eq!(
        read_mysql_dump(b"CREATE TABLE `\xff` (id int)")
            .unwrap_err()
            .code,
        "sql.identifier"
    );
    assert_eq!(
        read_mysql_dump(b"CREATE TABLE + (id int)")
            .unwrap_err()
            .code,
        "sql.identifier"
    );
    assert_eq!(
        read_mysql_dump(b"CREATE TABLE").unwrap_err().code,
        "sql.syntax"
    );
    assert_eq!(read_mysql_dump(b"CREATE TABLE sample (id int, label text); INSERT INTO sample (id) VALUES (1)").unwrap_err().code, "sql.row-width");
}

#[test]
fn known_envelope_cannot_drop_populated_tables_or_mutate_schema() {
    let prefix = "CREATE TABLE sample (id int);";
    for tail in [
        "LOCK TABLES sample READ",
        "ALTER TABLE sample ADD COLUMN other int",
        "UNLOCK TABLES extra",
    ] {
        assert_eq!(
            read_mysql_dump(format!("{prefix}{tail}").as_bytes())
                .unwrap_err()
                .code,
            "sql.unsupported-statement"
        );
    }
    assert!(read_mysql_dump(
        format!("{prefix} DROP TABLE IF EXISTS sample;").as_bytes()
    )
    .unwrap()
    .tables
    .is_empty());
    assert_eq!(
        read_mysql_dump(
            b"CREATE TABLE sample (id int); INSERT INTO sample VALUES ();"
        )
        .unwrap_err()
        .code,
        "sql.value"
    );
}

#[test]
fn source_and_token_limits_refuse_before_any_archive_is_produced() {
    assert_eq!(
        read_mysql_dump(&vec![b' '; 128 * 1024 * 1024 + 1])
            .unwrap_err()
            .code,
        "sql.limit"
    );
    assert_eq!(
        read_mysql_dump(&vec![b';'; 4_000_001]).unwrap_err().code,
        "sql.limit"
    );
}
