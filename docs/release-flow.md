# Release Flow

For more details, refer to the [official Release Flow documentation](http://releaseflow.org/).

## Configuration

Refer to the [`.rfconfig.yml`](../rfconfig.yml) file [reference](https://github.com/release-flow/release-flow?tab=readme-ov-file#configuration) for configuration details.

This file is pre-configured with the following settings:

- The `trunk` branch is named `main`.
- The version number to be used on the `trunk` branch is defined.
- The prefix for working branches is set to `feat/meta-*` (as usual).
- The prefix for release branches is set to `release/*`, followed by the major and minor version numbers, e.g., `release/1.0`, `release/1.1`, etc.

## Important Notes

I faced some challenges using [`release-flow`](https://github.com/release-flow/release-flow) alongside [`release-it`](https://github.com/release-it/release-it), but eventually found the optimal configuration.

### Branching and Tagging

During testing, I encountered [this issue with `release-it`](https://github.com/release-it/release-it/issues/1142), which arose because I was **merging** feature branches into different release branches *(and sometimes into `main`)*. This caused `release-it` to fail to find the correct version number and reject the release due to attempting to release an earlier version.

The key solution is:

- Develop feature branches as usual (it doesn't matter whether you start from `main` or a `release/*` branch).
- Once the feature is ready, merge it back into its source branch using a **squash merge**.
- If you need the feature or fix in another branch, use **cherry-pick**.

### Commit Messages

To ensure `release-flow` and `release-it` work well together, a few special commits are necessary:

- When creating a new branch, use the following command:  
  `git commit --allow-empty -m "Release branch $releaseBranchName"`
- When preparing a new release, commit with:  
  `git commit -m "Preparation for release"`
  - This commit writes the commit SHA of the release into a file named `.release-commit`.
    - This is required to ensure `release-it` works, even if there are no changes on the `trunk` or `release/*` branch.

### Initial Tag

The `main` branch must have an initial tag that matches the version number specified in the `.rfconfig.yml` file. This is essential for `release-it` to function properly.

## Example Flow

![Release Flow Example](assets/release-flow-look.png)

## GitHub Action Integration

![Release Flow GitHub Action](assets/release-flow-gh.png)

## Use Cases

### Creating a New Feature Branch from `main` and Applying It to a Release

1. Create a new feature branch from `main`.
2. Merge the feature branch back into `main` using `--squash`.
3. Release the new version from `main`.
4. Cherry-pick the squashed merge of the feature branch into the release branch, e.g., `release/2.7`.
5. Release the new version from the release branch.

### Creating a New Release Branch

1. Run the `Release Flow` GitHub Action.
2. Pull the newly created release branch from the remote.

### Creating a New Release Candidate from the Release Branch

1. Run the `Release Flow` GitHub Action.
2. Pull the latest changes from the remote.
