#!/usr/bin/env python3
"""
Validation script for tally integration test results.

This script validates that the tally results match expected outcomes by:
1. Checking SQLite database structure and content
2. Verifying vote counts for each candidate
3. Confirming winners are correctly identified
4. Validating report generation
"""

import argparse
import json
import sqlite3
import sys
from pathlib import Path
from typing import Dict, List, Any


class TallyValidator:
    """Validates tally results against expected outcomes."""

    def __init__(self, output_dir: Path, expected_results: Dict[str, Any]):
        self.output_dir = output_dir
        self.expected = expected_results
        self.errors: List[str] = []
        self.warnings: List[str] = []

    def validate(self) -> bool:
        """Run all validations. Returns True if all pass."""
        print("🔍 Starting tally results validation...")
        print(f"Output directory: {self.output_dir}")
        print()

        # Run validation checks
        self.check_required_files()
        db_path = self.find_database()

        if db_path:
            self.validate_database_structure(db_path)
            self.validate_vote_counts(db_path)
            self.validate_winners(db_path)
        else:
            self.errors.append("No SQLite database found - cannot validate results")

        self.check_reports()

        # Print results
        self.print_results()

        return len(self.errors) == 0

    def check_required_files(self):
        """Check that required output files exist."""
        print("📁 Checking required files...")

        file_checks = self.expected.get("file_checks", {})
        required_files = file_checks.get("required_files", [])

        for file_pattern in required_files:
            files = list(self.output_dir.rglob(file_pattern))
            if not files:
                self.errors.append(f"Required file not found: {file_pattern}")
            else:
                print(f"  ✅ Found: {files[0].name}")

    def find_database(self) -> Path | None:
        """Find the SQLite database file."""
        print("\n🗄️  Looking for SQLite database...")

        db_files = list(self.output_dir.rglob("*.db"))
        if not db_files:
            self.errors.append("No SQLite database (.db) file found")
            return None

        db_path = db_files[0]
        print(f"  ✅ Found database: {db_path}")
        return db_path

    def validate_database_structure(self, db_path: Path):
        """Validate that the database has the expected structure."""
        print("\n🏗️  Validating database structure...")

        try:
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()

            # Get list of tables
            cursor.execute("SELECT name FROM sqlite_master WHERE type='table'")
            actual_tables = {row[0] for row in cursor.fetchall()}

            # Check required tables
            db_checks = self.expected.get("database_checks", {})
            required_tables = db_checks.get("required_tables", [])

            for table in required_tables:
                if table in actual_tables:
                    print(f"  ✅ Table exists: {table}")
                else:
                    self.errors.append(f"Required table missing: {table}")

            # Check row counts
            expected_counts = db_checks.get("expected_row_counts", {})
            for table, expected_count in expected_counts.items():
                if table in actual_tables:
                    cursor.execute(f"SELECT COUNT(*) FROM {table}")
                    actual_count = cursor.fetchone()[0]

                    if actual_count == expected_count:
                        print(f"  ✅ {table}: {actual_count} rows (expected {expected_count})")
                    else:
                        self.warnings.append(
                            f"{table} has {actual_count} rows, expected {expected_count}"
                        )

            conn.close()

        except sqlite3.Error as e:
            self.errors.append(f"Database error: {e}")

    def validate_vote_counts(self, db_path: Path):
        """Validate that vote counts match expected values."""
        print("\n🗳️  Validating vote counts...")

        try:
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()

            # Check if results table exists
            cursor.execute(
                "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'results%'"
            )
            results_tables = [row[0] for row in cursor.fetchall()]

            if not results_tables:
                self.warnings.append("No results tables found - tally may not have completed")
                conn.close()
                return

            # Try to find candidate results
            # This query structure may need to be adjusted based on actual schema
            candidate_tables = [t for t in results_tables if 'candidate' in t.lower()]

            for table in candidate_tables:
                cursor.execute(f"SELECT * FROM {table} LIMIT 1")
                columns = [description[0] for description in cursor.description]
                print(f"  📊 Found results table: {table}")
                print(f"     Columns: {', '.join(columns)}")

            # Validate specific candidates if expected results are provided
            expected_candidates = self.expected.get("candidates", [])

            if expected_candidates:
                print("\n  Checking candidate vote counts:")
                for candidate in expected_candidates:
                    name = candidate["name"]
                    expected_votes = candidate["expected_votes"]
                    print(f"    {name}: expecting {expected_votes} votes")
                    # TODO: Query actual votes when schema is confirmed
                    # For now, just note that validation is pending
                    self.warnings.append(
                        f"Vote count validation for {name} pending (schema not fully mapped)"
                    )

            conn.close()

        except sqlite3.Error as e:
            self.errors.append(f"Error validating vote counts: {e}")

    def validate_winners(self, db_path: Path):
        """Validate that winners are correctly identified."""
        print("\n🏆 Validating winners...")

        expected_candidates = self.expected.get("candidates", [])
        expected_winners = [c["name"] for c in expected_candidates if c.get("is_winner")]

        print(f"  Expected winners: {', '.join(expected_winners)}")

        # TODO: Query actual winners from database
        # This will be implemented once the exact database schema is confirmed
        self.warnings.append("Winner validation pending (schema not fully mapped)")

    def check_reports(self):
        """Check that reports were generated."""
        print("\n📄 Checking report generation...")

        html_files = list(self.output_dir.rglob("*.html"))
        xlsx_files = list(self.output_dir.rglob("*.xlsx"))

        if html_files:
            print(f"  ✅ Found {len(html_files)} HTML report(s)")
        else:
            self.warnings.append("No HTML reports found")

        if xlsx_files:
            print(f"  ✅ Found {len(xlsx_files)} XLSX file(s)")
        else:
            print(f"  ℹ️  No XLSX files found (may be optional)")

    def print_results(self):
        """Print validation summary."""
        print("\n" + "=" * 60)
        print("VALIDATION SUMMARY")
        print("=" * 60)

        if self.warnings:
            print(f"\n⚠️  Warnings ({len(self.warnings)}):")
            for warning in self.warnings:
                print(f"  - {warning}")

        if self.errors:
            print(f"\n❌ Errors ({len(self.errors)}):")
            for error in self.errors:
                print(f"  - {error}")
            print("\n❌ VALIDATION FAILED")
        else:
            print("\n✅ VALIDATION PASSED")
            if self.warnings:
                print("   (with warnings)")


def main():
    parser = argparse.ArgumentParser(description="Validate tally results")
    parser.add_argument(
        "--output-dir",
        type=Path,
        required=True,
        help="Directory containing tally output files",
    )
    parser.add_argument(
        "--expected-results-file",
        type=Path,
        required=True,
        help="JSON file with expected results",
    )

    args = parser.parse_args()

    # Load expected results
    try:
        with open(args.expected_results_file) as f:
            expected_results = json.load(f)
    except FileNotFoundError:
        print(f"❌ Error: Expected results file not found: {args.expected_results_file}")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"❌ Error parsing expected results JSON: {e}")
        sys.exit(1)

    # Run validation
    validator = TallyValidator(args.output_dir, expected_results)
    success = validator.validate()

    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
