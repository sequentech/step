---
id: election_management_election_data
title: Datos
description: "La pestaña Datos de una elección contiene la configuración de una elección: sus nombres, idiomas, canales de votación, diseño de la papeleta y políticas."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Datos** de una elección contiene la configuración de una elección: sus nombres,
idiomas, canales de votación, diseño de la papeleta y políticas. También contiene las políticas
que controlan el informe de inicialización y el recuento.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, abra el proceso electoral.
2. Haga clic en la elección.
3. Haga clic en la pestaña **Datos**.

La pestaña se muestra con el permiso **Ver Datos de la Elección**. Para guardar cambios, su rol
necesita el permiso **Edit Election** (editar elección).

Para crear una elección, haga clic en la flecha junto al proceso electoral en el menú para mostrar
sus elecciones. Después haga clic en **Crear una Elección**. El formulario tiene **Nombre**, **ID
externo** y **Descripción**. Consulte
[Crear el proceso electoral](../../../../../procedures/02-event.md#3-create-the-elections).

## Secciones {#sections}

| Sección | Campos | Uso |
| --- | --- | --- |
| **General** | **ID externo**, y una pestaña para cada idioma con **Nombre**, **Alias** y **Descripción** | El identificador y los textos de la elección. No puede cambiar el **ID externo** después de guardarlo. |
| **Idioma** | Una casilla para cada idioma y una opción **Por defecto** | Los idiomas de la elección. |
| **Canales de Voto Permitidos** | **En línea**, **Kiosco** | Los canales de votación de la elección. |
| **Diseño de la Papeleta** | **Opciones de visualización del botón de auditoría**, el orden de las preguntas, **Reordenar concursos** | El aspecto de la papeleta. **Opciones de visualización del botón de auditoría** tiene **Mostrar**, **No mostrar** y **Mostrar en el cuadro de diálogo de ayuda**. El orden de las preguntas tiene **Aleatorias**, **Personalizado** y **Alfabético**. |
| **Imagen** | Un archivo de imagen | La imagen de la elección en el portal de votación. |
| **Configuración Avanzada** | Consulte la tabla de abajo | Las políticas de la elección. |

### Configuración avanzada {#advanced-configuration}

| Campo | Valores | Significado |
| --- | --- | --- |
| **Modal de Confirmación de Voto** | Activado o desactivado | Si el portal de votación muestra una ventana de confirmación cuando el votante emite el voto. |
| **Número de votos permitidos** | Un número | El número de votos que un votante puede emitir en esta elección. |
| **Gold level Authentication Policy** (política de autenticación de nivel oro) | **Gold level Authentication**, **No Gold level Authentication** | El nivel de autenticación que necesita el votante para emitir un voto. El valor por defecto es **No Gold level Authentication**. |
| **Etiqueta de permiso** | Texto | La etiqueta de permiso de la elección. Solo se muestra con el permiso **Editar la etiqueta de permiso**. Consulte [Etiquetas de permiso](../../../../../Tutorials/admin_portal_tutorials_permission-labels.md). |
| Archivo de configuración | Un archivo JSON | Ajustes avanzados de la elección en formato JSON. Cámbielo solo con la ayuda del soporte de Sequent. |
| **Inicializar Política de Reportes** | **No Requerido**, **Requerido** | Con **Requerido**, **Comenzar votación** no está disponible hasta que exista el informe de inicialización. |
| **Política de período de gracia** (en **Filtros personalizados**) | **Sin período de gracia**, **Período de gracia sin alerta** | El período de gracia al final del período de votación. El valor por defecto es **Sin período de gracia**. |
| **Tiempo de gracia en segundos** (en **Filtros personalizados**) | Un número | La duración del período de gracia. El valor por defecto es 0. |
| **Permitir Recuento** (en **Filtros personalizados**) | **Permitido**, **No permitido**, **Permitir cuando termine el periodo de votación** | Cuándo puede empezar el recuento. El valor por defecto es **Permitido**. |

| Valor de **Permitir Recuento** | Significado |
| --- | --- |
| **Permitido** | Puede iniciar el recuento en cualquier momento. |
| **No permitido** | No puede iniciar el recuento. |
| **Permitir cuando termine el periodo de votación** | Solo puede iniciar el recuento después de que el período de votación esté cerrado. |

:::danger ADVERTENCIA
Con **Permitido**, el portal de administración le deja iniciar el recuento mientras los votantes
todavía pueden votar. Use **Permitir cuando termine el periodo de votación** si el recuento debe
esperar al final del período de votación.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| No se puede cambiar el **ID externo**. | Esto es correcto. El **ID externo** es fijo después de guardarlo. |
| **Etiqueta de permiso** no está en el formulario. | Su rol no tiene el permiso **Editar la etiqueta de permiso**. |
| "No tienes permiso para acceder a esta elección." | La elección tiene una etiqueta de permiso que su cuenta no tiene. |
| **Comenzar votación** no está disponible. | **Inicializar Política de Reportes** es **Requerido**. Haga el informe de inicialización. Consulte [Publicar y gestionar el período de votación](../../../../../procedures/05-publish.md#4-make-the-initialization-report-if-necessary). |

## Páginas relacionadas {#related-pages}

- [Crear el proceso electoral](../../../../../procedures/02-event.md#3-create-the-elections)
- [Proceso electoral: Datos](../../Election-Event/Data/election_management_election-event_data.md)
- [Proceso electoral: Recuento](../../Election-Event/Tally/election_management_election-event_tally.md)
- [Proceso electoral: Eventos programados](../../Election-Event/Scheduled-Events/election_management_election-event_scheduled-events.md)
