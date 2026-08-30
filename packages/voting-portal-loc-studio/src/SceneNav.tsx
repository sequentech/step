// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Typography} from "@mui/material"
import {SCENES} from "./catalog"
import {useLocStudio} from "./LocStudioContext"

export const SceneNav: React.FC = () => {
    const {sceneId, variantId, setSceneId, setVariantId} = useLocStudio()

    return (
        <Box className="loc-studio-nav" component="nav">
            <Typography className="loc-studio-nav-title">Screens</Typography>
            {SCENES.map((scene) => (
                <Box key={scene.id} className="loc-studio-scene-group">
                    <button
                        type="button"
                        className={
                            scene.id === sceneId
                                ? "loc-studio-scene-button selected"
                                : "loc-studio-scene-button"
                        }
                        onClick={() => setSceneId(scene.id)}
                    >
                        {scene.label}
                    </button>
                    {scene.id === sceneId
                        ? scene.variants.map((variant) => (
                              <button
                                  key={variant.id}
                                  type="button"
                                  className={
                                      variant.id === variantId
                                          ? "loc-studio-variant-button selected"
                                          : "loc-studio-variant-button"
                                  }
                                  onClick={() => setVariantId(variant.id)}
                              >
                                  {variant.label}
                              </button>
                          ))
                        : null}
                </Box>
            ))}
        </Box>
    )
}
