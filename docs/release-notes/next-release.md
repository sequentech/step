<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT

## ✨ Admin Portal: Adding `Help` > `User Manual` in sidebar

In [#1700](https://github.com/sequentech/meta/issues/1700) we have added support
to show in the `Admin Portal` a `Help` item with configure links that open in a
new tab.

This is configurable from `Admin Portal` > `Settings` > `Look & Feel`, in a new
field that appears there called `Help Links`.

You can configure the links with the following format (this is just an example):

```json
[
    {
        "url": "https://example.com",
        "title": "System Manual",
        "i18n": {
            "en": {
                "title": "System Manual"
            },
            "es": {
                "title": "Manual del Sistema"
            },
            "fr": {
                "title": "System Manual"
            },
            "tl": {
                "title": "System Manual"
            },
            "cat": {
                "title": "System Manual"
            }
        }
    }
]
```

By default it will be undefined so it won't show. If the list is an empty array,
it will also not show.