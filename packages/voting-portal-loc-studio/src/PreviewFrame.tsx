// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useLayoutEffect, useRef, useState} from "react"
import {Box, GlobalStyles} from "@mui/material"
import {humanizeKey} from "./translations"
import {
    collectKeysFromRoots,
    elementForKey,
    hoverTargetFromPoint,
    isStudioChrome,
    previewRoots,
} from "./hover"
import {useLocStudio} from "./LocStudioContext"
import {LivePreview} from "./LivePreview"
import {handlePreviewDoubleClick} from "./previewNavigation"
import {previewBallotLayoutStyles} from "./previewBallotLayout"

interface HighlightBox {
    key: string
    label: string
    x: number
    y: number
    width: number
    height: number
    selected: boolean
}

export const PreviewFrame: React.FC = () => {
    const {
        selectedKey,
        hoveredKey,
        setSelectedKey,
        setHoveredKey,
        setOnScreenKeys,
        previewRevision,
        setSceneId,
        setVariantId,
    } = useLocStudio()
    const frameRef = useRef<HTMLDivElement>(null)
    const hoveredKeyRef = useRef<string | null>(null)
    const [highlights, setHighlights] = useState<HighlightBox[]>([])
    const [chip, setChip] = useState<{label: string; x: number; y: number} | null>(null)

    useLayoutEffect(() => {
        let debounce: number | undefined
        const collect = () => {
            window.clearTimeout(debounce)
            debounce = window.setTimeout(() => {
                setOnScreenKeys(collectKeysFromRoots(previewRoots(frameRef.current)))
            }, 80)
        }
        collect()
        const timeout = window.setTimeout(collect, 500)
        const observer = new MutationObserver(collect)
        if (frameRef.current) {
            observer.observe(frameRef.current, {childList: true, subtree: true})
        }
        observer.observe(document.body, {childList: true})
        return () => {
            window.clearTimeout(debounce)
            window.clearTimeout(timeout)
            observer.disconnect()
        }
    }, [previewRevision, setOnScreenKeys])

    useEffect(() => {
        const roots = previewRoots(frameRef.current)
        const boxes: HighlightBox[] = []
        const addBox = (key: string | null, selected: boolean) => {
            if (!key) {
                return
            }
            const element = elementForKey(roots, key)
            if (!element) {
                return
            }
            const rect = element.getBoundingClientRect()
            boxes.push({
                key,
                label: humanizeKey(key),
                x: rect.left,
                y: rect.top,
                width: rect.width,
                height: rect.height,
                selected,
            })
        }
        addBox(hoveredKey, false)
        if (selectedKey && selectedKey !== hoveredKey) {
            addBox(selectedKey, true)
        } else if (selectedKey) {
            addBox(selectedKey, true)
        }
        setHighlights(boxes)
        if (hoveredKey) {
            const hoverBox = boxes.find((box) => box.key === hoveredKey)
            setChip(
                hoverBox
                    ? {
                          label: hoverBox.label,
                          x: hoverBox.x,
                          y: Math.max(8, hoverBox.y - 28),
                      }
                    : null
            )
        } else {
            setChip(null)
        }
    }, [hoveredKey, selectedKey, previewRevision])

    useEffect(() => {
        const handleClick = (event: MouseEvent) => {
            const target = event.target as HTMLElement | null
            if (!target || isStudioChrome(target) || !event.isTrusted) {
                return
            }
            if (target.closest("a")) {
                event.preventDefault()
            }
            const hit = hoverTargetFromPoint(event.clientX, event.clientY)
            if (hit) {
                event.preventDefault()
                event.stopPropagation()
                setSelectedKey(hit.key)
            }
        }

        const handleMove = (event: MouseEvent) => {
            const target = event.target as HTMLElement | null
            if (!target || isStudioChrome(target)) {
                if (hoveredKeyRef.current !== null) {
                    hoveredKeyRef.current = null
                    setHoveredKey(null)
                }
                return
            }
            const hit = hoverTargetFromPoint(event.clientX, event.clientY)
            const key = hit?.key ?? null
            if (key !== hoveredKeyRef.current) {
                hoveredKeyRef.current = key
                setHoveredKey(key)
            }
        }

        const handleDoubleClick = (event: MouseEvent) => {
            handlePreviewDoubleClick(event, ({sceneId, variantId}) => {
                setSceneId(sceneId)
                setVariantId(variantId)
            })
        }

        document.addEventListener("click", handleClick, true)
        document.addEventListener("dblclick", handleDoubleClick, true)
        document.addEventListener("mousemove", handleMove)
        return () => {
            document.removeEventListener("click", handleClick, true)
            document.removeEventListener("dblclick", handleDoubleClick, true)
            document.removeEventListener("mousemove", handleMove)
        }
    }, [setHoveredKey, setSelectedKey, setSceneId, setVariantId])

    return (
        <Box className="loc-studio-preview">
            <GlobalStyles styles={previewBallotLayoutStyles} />
            <Box className="loc-studio-preview-frame" key={previewRevision} ref={frameRef}>
                <LivePreview />
            </Box>
            {highlights.map((box) => (
                <Box
                    key={`${box.key}-${box.selected ? "selected" : "hover"}`}
                    className={
                        box.selected
                            ? "loc-studio-highlight selected"
                            : "loc-studio-highlight"
                    }
                    style={{
                        left: box.x,
                        top: box.y,
                        width: box.width,
                        height: box.height,
                    }}
                />
            ))}
            {chip ? (
                <Box className="loc-studio-hover-chip" style={{left: chip.x, top: chip.y}}>
                    {chip.label}
                </Box>
            ) : null}
        </Box>
    )
}
