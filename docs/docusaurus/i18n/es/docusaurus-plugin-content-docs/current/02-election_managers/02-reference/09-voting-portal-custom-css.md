---
id: voting_portal_custom_css
title: Ganchos CSS del portal de votación
sidebar_position: 9
description: "Use las clases estables que se indican a continuación en Datos → Diseño de la papeleta → CSS Personalizado. Prefiera una clase de pantalla o de componente frente a las clases Emotion generadas, al texto traducido, a las posiciones de los elementos o a una etiqueta HTML supuesta."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Use las clases estables que se indican a continuación en **Datos → Diseño de la papeleta → CSS Personalizado**. Prefiera una clase de pantalla o de componente frente a las clases Emotion generadas, al texto traducido, a las posiciones de los elementos o a una etiqueta HTML supuesta. Para ver los pasos, consulte [Personalizar el portal de votación con CSS](../01-tutorials/22-admin_portal_tutorials_custom-css.md).
Una acción de navegación puede ser un enlace con aspecto de botón. Las clases existentes siguen siendo compatibles. Los elementos de maquetación, las etiquetas, los iconos, los indicadores de carga y los controles tienen ganchos descriptivos, además de sus secciones contenedoras. Los fragmentos de React, los proveedores y los componentes de enrutamiento no generan elementos DOM propios; sus elementos hijos renderizados son los que llevan los ganchos.
Los elementos internos generados por la biblioteca siguen usando las clases públicas de la biblioteca, como `.MuiCheckbox-root` y `.MuiInputLabel-root`, limitadas al componente correspondiente.

## Pantallas y estructura compartida {#screens-and-shared-structure}

| Pantalla                | Selector raíz                                                          |
| ----------------------- | ---------------------------------------------------------------------- |
| Lista de papeletas      | `.election-selection-screen`                                           |
| Instrucciones           | `.start-screen`                                                        |
| Votación                | `.voting-screen`                                                       |
| Revisión                | `.review-screen` (alternativa para papeleta no disponible: `.review-error-screen`) |
| Confirmación            | `.confirmation-screen`                                                 |
| Localizador de papeletas | `.ballot-locator-screen`                                              |
| Auditoría               | `.audit-screen`                                                        |
| Materiales de soporte   | `.support-materials-screen`                                            |
| Error                   | `.error-screen`                                                        |
| Inicio de sesión / registro | `.login-screen`, `.register-screen`                                |
| Vista previa de la publicación | `.preview-publication-screen`                                   |

Las raíces de pantalla también tienen `.screen`. Dentro de la pantalla correspondiente, use `.screen-title`, `.screen-description`, `.stepper-box`, `.screen-help-button` y `.actions-container`. La pantalla de Instrucciones no tiene botón de ayuda de página. Las secciones opcionales solo se muestran cuando la configuración de la elección las activa.

| Área                  | Ganchos admitidos                                                                                                                                                                          |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Cabecera              | `.header-class`, `.header-logo-link`, `.header-logo`, `.header-actions`, `.app-version` (versión y hash)                                                                               |
| Menú de idioma        | `.language-selector`, `.language-selector-button`, `.language-selector-menu`, `.language-option`                                                                                         |
| Menú de perfil        | `.profile-menu-container`, `.profile-menu-button`, `.profile-menu`, `.user-details`, `.profile`, `.logout-button`                                                                        |
| Pie de página         | `.footer-class`, `.footer-link`                                                                                                                                                          |
| Progreso              | `.step-container`, `.step-number`, `.step-separator`, `.selected`, `.not-selected` (limítelos a `.step-container`)                                                                        |
| Estructura de diálogos | `.dialog`, `.dialog-title`, `.dialog-title-text`, `.dialog-content`, `.dialog-error`, `.dialog-actions`, `.dialog-close-button`, `.dialog-expand-button`, `.ok-button`, `.cancel-button` |
| Controles de icono compartidos | `.icon-button`; las clases de botón específicas de cada función que aparecen más abajo se aplican al control, no a su SVG                                                       |

Los diálogos y menús se renderizan en portales fuera de la raíz de la pantalla. Diríjalos directamente a sus propias clases desde una hoja de estilos global, no como descendientes de una pantalla. El CSS Personalizado de la elección tiene actualmente el alcance del contenedor de la aplicación y no llega a esos portales; añadir ganchos no cambia ese alcance. Los diálogos de ayuda de página tienen `.screen-help-dialog` más una de estas clases: `.election-selection-help-dialog`, `.voting-help-dialog`, `.review-help-dialog`, `.confirmation-help-dialog` o `.ballot-locator-help-dialog`. Otros diálogos incluyen `.demo-dialog`, `.decline-to-vote-dialog`, `.ballot-validation-dialog`, `.audit-ballot-dialog`, `.confirm-cast-ballot-dialog`, `.review-ballot-id-help-dialog`, `.logout-dialog` y `.session-expiry-dialog`.

## Preguntas individuales {#individual-contests}

En Votación y en Revisión, una pregunta con `external_id: "1001"` tiene la clase `.c-1001` en su `.contest-container`. Las clases existentes `.contest-0`, `.contest-1`, etc. siguen siendo compatibles. Use la clase basada en el ID externo cuando el estilo deba sobrevivir a la reordenación, a la paginación, a los UUID generados por la importación o a estilos de grupos de votantes distintos.

```css
.c-1001 .candidate-item {
    background-color: #fff4cc;
}
```

La clase usa las mismas reglas de formato que las clases de elección: conserva las mayúsculas, elimina los espacios en blanco y todos los caracteres excepto letras ASCII, dígitos, `_` y `-`, antepone `c-` y, a continuación, trunca la clase completa a 40 caracteres. Los ID ausentes, nulos, vacíos o eliminados por completo no añaden ninguna clase. Elija ID externos que sigan siendo únicos después de este formato; de lo contrario, ID distintos pueden producir la misma clase.

La clase `e-*` existente en `.app-root` procede del alias o nombre traducido de la elección, con alternativas al idioma por defecto y al alias o nombre sin traducir y, por último, del UUID de la elección. **No** procede de `election.external_id` y puede cambiar según el idioma. Así, `.e-Senat .c-1001 .candidate-item` se aplica cuando el alias o nombre resuelto de la elección es `Senat` y el ID externo de la pregunta es `1001`.

El ID externo se incluye en la representación Borsh que se usa para firmar los estilos de papeleta. Esto cambia el formato firmado aunque el ID externo esté ausente. Actualice y vuelva a publicar antes de que comience la votación; no introduzca este cambio durante una elección activa. Volver a publicar no actualiza las firmas de las papeletas ya emitidas, que requieren un software de verificación compatible con el formato anterior. El JSON antiguo sin este campo se sigue cargando. No se necesita ninguna migración de base de datos ni ningún campo de configuración nuevo.

## Componentes específicos de cada pantalla {#screen-specific-components}

| Área                         | Ganchos admitidos                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tarjetas de papeleta         | `.elections-list`, `.election-list-item` (tarjeta completa), `.election-item`, `.election-info`, `.election-title`, `.election-vote-status`, `.election-open-status`, `.election-dates`, `.election-open-date`, `.election-close-date`, `.election-website-link`, `.election-countdown`, `.election-actions`                                                                                                                                                                                    |
| Acciones y mensajes de la lista de papeletas | `.click-to-vote-button`, `.locate-ballot-button`, `.election-results-button`, `.election-event-results-button`, `.support-materials-button`, `.materials-gate-banner`, `.election-selection-warning`, `.elections-empty`                                                                                                                                                                                                                                                                     |
| Instrucciones                | `.instructions-title`, `.instructions-description`, `.instructions-steps`, `.instructions-step`, `.instructions-select-step`, `.instructions-review-step`, `.instructions-cast-step`                                                                                                                                                                                                                                                                                                         |
| Declaración de elegibilidad  | `.security-confirmation` (declaración completa), `.security-confirmation-checkbox`, `.security-confirmation-label`                                                                                                                                                                                                                                                                                                                                                                           |
| Acciones de inicio           | `.start-voting-button`, `.decline-to-vote-button`                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| Preguntas y opciones         | `.contest`, `.contest-container`, `.contest-title`, `.contest-description`, `.contest-options-toggle`, `.candidates-container`, `.candidates-list`, `.candidates-list-title`, `.candidates-list-toggle`, `.candidates-list-checkbox`, `.candidates-selected-count`, `.candidate-item`, `.candidate-title`, `.candidate-description`, `.candidate-checkbox`, `.candidate-input`, `.candidate-writein-textfield`, `.candidate-position-select`, `.candidate-position-label`, `.candidate-link` |
| Acciones de votación y revisión | `.back-button`, `.clear-selection-button`, `.next-button`, `.edit-ballot-button`, `.audit-button`, `.cast-ballot-button`, `.cast-ballot-error`                                                                                                                                                                                                                                                                                                                                               |
| Seguimiento de la revisión   | `.hash-container`, `.hash-text`, `.hash-actions`, `.hash-copy-button`, `.hash-help-button`, `.hash-copy-status`                                                                                                                                                                                                                                                                                                                                                                              |
| Búsqueda y resultado del localizador | `.ballot-locator-content`, `.ballot-locator-search`, `.ballot-locator-result`, `.ballot-id-field` (campo completo), `.ballot-id-input`, `.ballot-id-error`, `.ballot-lookup-status`, `.ballot-content-description`, `.ballot-content`, `.locate-ballot-button`, `.locate-again-button`, `.back-button`                                                                                                                                                                                          |
| Pestañas y registros del localizador | `.ballot-locator-tabs`, `.ballot-lookup-tab`, `.ballot-lookup-panel`, `.cast-vote-logs-tab`, `.cast-vote-logs-panel`, `.cast-vote-logs-title`, `.cast-vote-logs-table`, `.cast-vote-logs-pagination`, `.cast-vote-log-copy-button`                                                                                                                                                                                                                                                           |

Los mensajes de validación también conservan sus [ganchos CSS específicos de cada mensaje](./08-ballot-errors-custom-css.md).

## ID de papeleta de confirmación {#confirmation-ballot-id}

| Componente                                               | Selector dentro de `.confirmation-screen`                                         |
| -------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Fila completa del ID de papeleta, con su etiqueta y sus controles | `.ballot-id-container`                                                   |
| Grupo de valor con borde                                 | `.ballot-id-border`                                                               |
| Etiqueta / icono de estado                               | `.ballot-id-label` / `.ballot-id-status-icon`                                     |
| Ambos valores adaptables                                 | `.ballot-id-value`                                                                |
| Variantes adaptables individuales                        | `.ballot-id-value-desktop`, `.ballot-id-value-mobile`                             |
| Control de ayuda                                         | `.ballot-id-help-button`                                                          |
| Control / estado de copia                                | `.hash-copy-button`, `.hash-copy-status`                                          |
| Título, descripción y QR de verificación                 | `.ballot-verification-title`, `.ballot-verification-description`, `.qr-container` |
| Acciones de recibo / finalización                        | `.print-receipt-button`, `.finish-button`                                         |

Los diálogos de confirmación usan `.ballot-id-help-dialog`, `.demo-ballot-id-help-dialog`, `.demo-ballot-url-dialog`, `.demo-print-receipt-dialog` y `.print-receipt-error-dialog`.

Para ocultar **solo la fila completa del ID de papeleta**, en escritorio y en móvil:

```css
.confirmation-screen .ballot-id-container {
  display: none;
}
```

Esta regla también oculta los controles de copia y de ayuda de la fila, así como los enlaces de valor adaptables, de modo que no queden paradas de teclado sin función. No oculta la descripción de verificación, el código QR, la acción de recibo ni el botón Finalizar. Ninguna sección está oculta por defecto, y el CSS no cambia el cifrado, la emisión ni la verificación de las papeletas. Las elecciones totalmente aclamadas ya omiten el ID de papeleta y el QR porque no se emite ninguna papeleta.

Al ocultar otras secciones, mantenga juntas las etiquetas y sus controles; no oculte un destino de nombre o de descripción accesible mientras deja visible su control. Vuelva a comprobar el foco del teclado, el contraste y la maquetación adaptable después de aplicar el CSS del inquilino. Reconstruya `ui-essentials` antes de verificar los cambios en el código fuente de los componentes compartidos.

## Elementos internos y pantallas secundarias {#inner-elements-and-secondary-screens}

Limite los ganchos internos repetidos a su sección contenedora. Por ejemplo, `.instructions-select-step .instructions-step-title` selecciona solo el primer encabezado de instrucción. Las etiquetas e iconos de los botones tienen ganchos separados, como `.finish-button-label`, `.print-receipt-label`, `.print-receipt-icon` y `.print-receipt-progress`; ocultar una etiqueta por sí sola no oculta su botón.

| Área                     | Ganchos internos útiles                                                                                                                                                                                                                                                                                                                                               |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Aplicación / carga       | `.voting-portal-wrapper`, `.loader`, `.loader-progress`, `.loader-label`, `.apollo-loading`, `.apollo-loading-progress`                                                                                                                                                                                                                                          |
| Títulos de pantalla      | `.screen-title-text`                                                                                                                                                                                                                                                                                                                                             |
| Cabecera / pie de página | `.header-page-limit`, `.header-content`, `.app-version-content`, `.app-version-label`, `.app-version-value`, `.footer-powered-by`                                                                                                                                                                                                                                |
| Instrucciones            | `.instructions-step-title`, `.instructions-step-description`                                                                                                                                                                                                                                                                                                     |
| Candidatos               | `.candidate-text`, `.candidate-write-in`, `.candidate-write-in-label`, `.candidate-preference-label`, `.candidate-preference-option`                                                                                                                                                                                                                             |
| Grupos de candidatos     | `.candidates-list-heading`, `.candidates-list-actions`, `.candidates-list-toggle-label`, `.candidates-list-toggle-icon`, `.candidates-list-collapse`, `.candidates-subtype`, `.candidates-subtype-title`, `.candidates-subtype-list`                                                                                                                             |
| Validación               | `.write-in-error`, `.contest-validation-warning`, `.contest-validation-info`, `.warn-box-content`, `.warn-box-severity`, `.warn-box-close-button`                                                                                                                                                                                                                |
| Auditoría                | `.audit-download-container`, `.audit-download-title`, `.audit-download-description`, `.download-auditable-ballot-button`, `.auditable-ballot-data`, `.audit-verification-title`, `.audit-verification-description`, `.ballot-verifier-link`, `.audit-warning`, `.print-audit-button`, `.restart-voting-button`                                                   |
| Materiales               | `.support-materials-header`, `.support-materials-list`, `.support-material`, `.support-material-title`, `.support-material-subtitle`, `.support-material-actions`, `.support-material-preview-button`                                                                                                                                                            |
| Vista previa de materiales | `.support-material-preview-dialog`, `.support-material-preview-content`, `.support-material-image`, `.support-material-pdf`, `.support-material-video`, `.support-material-audio`, `.support-material-download-button`                                                                                                                                           |
| Confirmación de materiales | `.materials-acknowledgement`, `.materials-acknowledgement-label`, `.materials-acknowledgement-checkbox`, `.materials-acknowledgement-error`, `.materials-actions`, `.materials-continue-button`                                                                                                                                                                  |
| Registros del localizador | `.cast-vote-logs-filter`, `.cast-vote-logs-table-container`, `.cast-vote-logs-table-head`, `.cast-vote-logs-table-body`, `.cast-vote-logs-row`, `.cast-vote-logs-sort-button`; las celdas de columna usan `.cast-vote-logs-username-cell`, `.cast-vote-logs-ballot-id-cell`, `.cast-vote-logs-kind-cell`, `.cast-vote-logs-timestamp-cell`, `.cast-vote-logs-message-cell` |
| Texto expandible         | `.expandable-text`, `.expandable-text-content`, `.expandable-text-full`, `.expandable-text-preview`, `.expandable-text-ellipsis`, `.expandable-text-toggle`                                                                                                                                                                                                      |
| Error                    | `.error-content`, `.error-description`, `.error-detail`, `.error-back-link`, `.back-button`                                                                                                                                                                                                                                                                      |

Las superficies en portal también exponen `.dialog-container`, `.dialog-backdrop`, `.dialog-paper`, `.language-selector-paper`, `.language-selector-options`, `.profile-menu-paper`, `.profile-menu-options`, `.candidate-preference-menu`, `.candidate-preference-paper`, `.candidate-preference-options`, `.header-tooltip` y `.header-tooltip-content`. Los diálogos de auditoría tienen sus propios ganchos `.audit-ballot-id-help-dialog`, `.audit-help-dialog` y `.audit-download-help-dialog`.
