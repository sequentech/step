<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Website

This website is built using [Docusaurus](https://docusaurus.io/), a modern static website generator.

### Installation

```
$ yarn
```

### Local Development

```
$ yarn start
```

This command starts a local development server and opens up a browser window. Most changes are reflected live without having to restart the server.

### Build

```
$ yarn build
```

This command generates static content into the `build` directory and can be served using any static contents hosting service.

### Deployment

Using SSH:

```
$ USE_SSH=true yarn deploy
```

Not using SSH:

```
$ GIT_USER=<Your GitHub username> yarn deploy
```

If you are using GitHub pages for hosting, this command is a convenient way to build the website and push to the `gh-pages` branch.

## Synchronized code examples

Use ordinary Markdown fences for shared commands. For alternatives, place adjacent
fences with the same `group` and a distinct `tab` label:

````markdown
```bash group="engine" tab="k6"
step-cli load prepare voting-load.yaml \
  --engine k6 \
  --output runs/smoke
```

```bash group="engine" tab="Chromium"
step-cli load prepare voting-load.yaml \
  --engine chromium \
  --output runs/smoke
```
````

Selecting a tab switches all matching groups on that page and remembers the
choice. Use consistent labels within a group. Groups on other pages are independent.
Any labels and number of alternatives work: for example, `group="language"` with
`tab="Go"`, `tab="Rust"` and `tab="PHP"`. A group missing the selected alternative
keeps its current selection. A lone fence renders without tabs.

No imports or JSX are needed. Titles and line highlighting work as usual. Keep
prose outside adjacent alternatives; prose separates them into distinct selectors.
The first alternative is the default for a new reader. Use different group names
for independent choices on the same page.

From this directory, run `node --test plugins/*.test.js` to test the Markdown
transform and `yarn build` to check the complete site. The widget uses Docusaurus
Tabs for keyboard navigation, synchronization and light/dark theme support.
