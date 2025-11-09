<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# ui-essentials

Shared UI component library for Sequent Voting Platform portals. Provides a consistent design system and reusable components across admin-portal, voting-portal, and ballot-verifier.

## Features

- Comprehensive component library with Storybook documentation
- Consistent styling and theming
- Accessibility-compliant components (WCAG 2.1)
- TypeScript support
- Responsive design utilities

## Development

### Running Storybook

From the monorepo root:

```bash
cd packages/
yarn storybook:ui-essentials
```

### Building

After updating components, build the library:

```bash
cd packages/
yarn prettify:fix:ui-essentials && yarn build:ui-essentials
```

This ensures dependent portals can use the latest component versions.

## Documentation

Component documentation is available in Storybook. See also:
- [Chromatic Deployment][chromatic-link]
- [Developer Documentation](https://docs.sequentech.io/docusaurus/main/docs/developers/)

[chromatic-badge]: https://raw.githubusercontent.com/storybookjs/brand/059f152ecfa4e9895380cb0e4a1f48cf80311a69/badge/badge-storybook.svg
[chromatic-link]: https://4bc0ab927dbe43e835cc02d6-clvdjqwsjr.chromatic.com/
