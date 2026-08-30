// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo, useState} from "react"
import {Box, Button, TextField, Typography} from "@mui/material"
import {humanizeKey, keyGroup, visibleText} from "./translations"
import {useCurrentScene, useLocStudio} from "./LocStudioContext"
import {isContentKey} from "./uploadedElection"

interface GroupedKey {
    key: string
    group: string
    label: string
    preview: string
}

export const KeyPanel: React.FC = () => {
    const {
        selectedKey,
        hoveredKey,
        onScreenKeys,
        setSelectedKey,
        setHoveredKey,
        currentBundle,
        setOverride,
        resetOverride,
        isKeyEdited,
        getOriginalForKey,
        uploadedEvent,
    } = useLocStudio()
    const {variant} = useCurrentScene()
    const [query, setQuery] = useState("")

    const groupedKeys = useMemo(() => {
        const items: GroupedKey[] = onScreenKeys
            .filter((key) => key in currentBundle)
            .map((key) => {
                const contentRef = isContentKey(key) ? uploadedEvent?.fieldRefs.get(key) : undefined
                return {
                    key,
                    group: contentRef ? contentRef.group : keyGroup(key),
                    label: contentRef ? contentRef.fieldLabel : humanizeKey(key),
                    preview: visibleText(currentBundle[key] || ""),
                }
            })
        const needle = query.trim().toLowerCase()
        const filtered = needle
            ? items.filter(
                  (item) =>
                      item.key.toLowerCase().includes(needle) ||
                      item.label.toLowerCase().includes(needle) ||
                      item.preview.toLowerCase().includes(needle)
              )
            : items

        const groups = new Map<string, GroupedKey[]>()
        filtered.forEach((item) => {
            const list = groups.get(item.group) || []
            list.push(item)
            groups.set(item.group, list)
        })
        return Array.from(groups.entries()).map(([group, keys]) => ({
            group,
            keys: keys.sort((left, right) => left.preview.localeCompare(right.preview)),
        }))
    }, [currentBundle, onScreenKeys, query])

    const currentValue = selectedKey ? (currentBundle[selectedKey] ?? "") : ""
    const isOverridden = Boolean(selectedKey && isKeyEdited(selectedKey))
    const selectedLabel = selectedKey
        ? isContentKey(selectedKey)
            ? (uploadedEvent?.fieldRefs.get(selectedKey)?.fieldLabel ?? selectedKey)
            : humanizeKey(selectedKey)
        : ""

    return (
        <Box className="loc-studio-keys" component="aside">
            <Typography className="loc-studio-nav-title">On this screen</Typography>
            <Typography className="loc-studio-help">{variant.description}</Typography>
            <TextField
                size="small"
                fullWidth
                placeholder="Search text on this screen"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
            />
            <Box className="loc-studio-key-list">
                {groupedKeys.length === 0 ? (
                    <Typography className="loc-studio-help">
                        No editable text on this view. Try another screen variation.
                    </Typography>
                ) : (
                    groupedKeys.map((group) => (
                        <Box key={group.group} className="loc-studio-key-group">
                            <Typography className="loc-studio-key-group-title">
                                {group.group}
                            </Typography>
                            {group.keys.map((item) => (
                                <button
                                    key={item.key}
                                    type="button"
                                    className={
                                        item.key === selectedKey
                                            ? "loc-studio-key-item selected"
                                            : item.key === hoveredKey
                                              ? "loc-studio-key-item hovered"
                                              : "loc-studio-key-item"
                                    }
                                    onClick={() => setSelectedKey(item.key)}
                                    onMouseEnter={() => setHoveredKey(item.key)}
                                    onMouseLeave={() => setHoveredKey(null)}
                                >
                                    <span className="loc-studio-key-preview">
                                        {item.preview || "(empty)"}
                                    </span>
                                    <span className="loc-studio-key-label">{item.label}</span>
                                    {isKeyEdited(item.key) ? (
                                        <span className="loc-studio-key-badge">edited</span>
                                    ) : null}
                                </button>
                            ))}
                        </Box>
                    ))
                )}
            </Box>
            {selectedKey ? (
                <Box className="loc-studio-editor">
                    <Typography className="loc-studio-editor-title">{selectedLabel}</Typography>
                    <Typography className="loc-studio-help">{selectedKey}</Typography>
                    <TextField
                        multiline
                        minRows={4}
                        fullWidth
                        value={currentValue}
                        onChange={(event) => setOverride(selectedKey, event.target.value)}
                    />
                    {isOverridden ? (
                        <Typography className="loc-studio-help">
                            Original: {visibleText(getOriginalForKey(selectedKey) || "") || "(empty)"}
                        </Typography>
                    ) : null}
                    <Box className="loc-studio-editor-actions">
                        <Button
                            size="small"
                            onClick={() => navigator.clipboard.writeText(selectedKey)}
                        >
                            Copy key
                        </Button>
                        <Button
                            size="small"
                            disabled={!isOverridden}
                            onClick={() => resetOverride(selectedKey)}
                        >
                            Reset
                        </Button>
                    </Box>
                </Box>
            ) : (
                <Typography className="loc-studio-help">
                    Hover a highlighted phrase in the preview, then click it to edit.
                </Typography>
            )}
        </Box>
    )
}
