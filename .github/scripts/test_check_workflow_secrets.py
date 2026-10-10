# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

import unittest

import yaml

from check_workflow_secrets import check_workflow

OUTSIDE_ENVIRONMENT = (
    "workflow.yml: job '{}' reads release credentials outside the "
    "'release' environment"
)
IN_CONTRIBUTION_WORKFLOW = (
    "workflow.yml: job '{}' reads release credentials in a workflow triggered by {}"
)


def check(source):
    return check_workflow("workflow.yml", yaml.safe_load(source))


class CheckWorkflowSecretsTest(unittest.TestCase):
    def test_accepts_release_secret_in_release_environment(self):
        errors = check("""
on: workflow_dispatch
jobs:
  deploy:
    runs-on: ubuntu-24.04
    environment: release
    steps:
      - run: echo
        env:
          TOKEN: ${{ secrets.PAT }}
""")
        self.assertEqual(errors, [])

    def test_accepts_release_environment_given_by_name(self):
        errors = check("""
on: workflow_dispatch
jobs:
  deploy:
    runs-on: ubuntu-24.04
    environment:
      name: release
    steps:
      - uses: docker/login-action@v3
        with:
          password: ${{ secrets.DOCKERHUB_TOKEN }}
""")
        self.assertEqual(errors, [])

    def test_rejects_release_secret_without_environment(self):
        errors = check("""
on: push
jobs:
  build:
    runs-on: ubuntu-24.04
    steps:
      - uses: aws-actions/configure-aws-credentials@v4
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID_GLOBALDOT }}
""")
        self.assertEqual(errors, [OUTSIDE_ENVIRONMENT.format("build")])

    def test_rejects_release_secret_in_other_environment(self):
        errors = check("""
on: push
jobs:
  build:
    runs-on: ubuntu-24.04
    environment: preview
    steps:
      - run: echo ${{ secrets.PAT }}
""")
        self.assertEqual(errors, [OUTSIDE_ENVIRONMENT.format("build")])

    def test_rejects_secrets_read_as_a_whole_or_by_index(self):
        errors = check("""
on: push
jobs:
  dump:
    runs-on: ubuntu-24.04
    steps:
      - run: echo ${{ toJSON(secrets) }}
  index:
    runs-on: ubuntu-24.04
    steps:
      - run: echo ${{ secrets['PAT'] }}
""")
        self.assertEqual(
            errors,
            [OUTSIDE_ENVIRONMENT.format("dump"), OUTSIDE_ENVIRONMENT.format("index")],
        )

    def test_ignores_other_secrets(self):
        errors = check("""
on: pull_request
jobs:
  scan:
    runs-on: ubuntu-24.04
    steps:
      - run: echo
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          SONAR_TOKEN: ${{ secrets.SONAR_TOKEN }}
          PATH_PREFIX: ${{ secrets.PATH_PREFIX }}
""")
        self.assertEqual(errors, [])

    def test_rejects_secrets_inherit(self):
        errors = check("""
on: workflow_dispatch
jobs:
  build:
    uses: ./.github/workflows/build.yml
    secrets: inherit
""")
        self.assertEqual(
            errors,
            ["workflow.yml: job 'build' passes every secret with 'secrets: inherit'"],
        )

    def test_accepts_explicit_secrets_to_local_workflow(self):
        errors = check("""
on: workflow_dispatch
jobs:
  build:
    uses: ./.github/workflows/build.yml
    secrets:
      PAT: ${{ secrets.PAT }}
""")
        self.assertEqual(errors, [])

    def test_rejects_release_secrets_to_external_workflow(self):
        errors = check("""
on: workflow_dispatch
jobs:
  build:
    uses: example/workflows/.github/workflows/build.yml@v1
    secrets:
      PAT: ${{ secrets.PAT }}
""")
        self.assertEqual(errors, [OUTSIDE_ENVIRONMENT.format("build")])

    def test_rejects_release_secret_in_pull_request_workflow(self):
        errors = check("""
on:
  push:
  pull_request:
jobs:
  publish:
    runs-on: ubuntu-24.04
    environment: release
    steps:
      - run: echo
        env:
          TOKEN: ${{ secrets.PAT }}
""")
        self.assertEqual(
            errors, [IN_CONTRIBUTION_WORKFLOW.format("publish", "pull_request")]
        )

    def test_rejects_release_secret_in_comment_workflow(self):
        errors = check("""
on: [issue_comment, pull_request_target]
jobs:
  sign:
    runs-on: ubuntu-24.04
    steps:
      - run: echo
        env:
          TOKEN: ${{ secrets.PAT }}
""")
        self.assertEqual(
            errors,
            [
                IN_CONTRIBUTION_WORKFLOW.format(
                    "sign", "issue_comment, pull_request_target"
                ),
                OUTSIDE_ENVIRONMENT.format("sign"),
            ],
        )


if __name__ == "__main__":
    unittest.main()
