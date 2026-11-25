---
id: admin_portal_tutorials_election-results
title: Resultados electorales
description: "La ruta principal está en Obtener los resultados. Esta página da los detalles: las secciones de la página de resultados, todos los formatos y niveles de exportación, y la publicación en el sitio de resultados."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La ruta principal está en [Obtener los resultados](../03-procedures/07-results.md). Esta página da los detalles: las secciones de la página de resultados, todos los formatos y niveles de exportación, y la publicación en el sitio de resultados.

## Antes de empezar {#before-you-start}

- El estado del recuento es `SUCCESS`. Consulte [Ceremonia de recuento](16-admin_portal_tutorials_tally-ceremony.md).
- Para ver los resultados, su cuenta necesita **Leer Resultados de Recuento**.
- Para descargar los documentos, su cuenta necesita **Exportar Ceremonia**.

## La página de resultados {#the-results-page}

Abra el recuento desde la pestaña **Recuento** y vaya al paso **Resultados**. La página tiene estas secciones.

### Información general {#general-information}

- **Fecha de Recuento**: la fecha del recuento.
- Una tabla con una fila para cada elección: **Elecciones**, **Votantes Elegibles**, **Total de votantes** y **Participación**. Algunas elecciones también muestran **Total de votos de renuncia** o **Total de Papeletas en Blanco**.

### Resultados y participación {#results--participation}

Seleccione una elección, después una pregunta y después un área. La pestaña de área **Global** muestra todas las áreas juntas. Para cada selección, la página muestra:

- El **Resumen de participación**: **Votantes Elegibles**, **Total de votantes**, **Total de votos válidos**, **Total de votos inválidos** y **Votos en Blanco**, con sus porcentajes. Los votos inválidos y en blanco pueden mostrarse como votos explícitos e implícitos.
- La **Participación por canal**: los votos de cada canal, por ejemplo **En línea**, **Kiosco**, **Votación anticipada** y **Votación telefónica**. Los votos de las actas de escrutinio se muestran como **Papel**, **Postal** o **Presencial**.
- Los **Resultados de Candidatos**: para cada candidato, el **Número de Votos**, el **Porcentajes de Votos** y la **Posición ganadora**. Con el voto ponderado por votante, la tabla también muestra el **Peso**.
- Gráficos de los votos de los candidatos, de los votos en blanco y de los votos inválidos.

Una pregunta que se ganó por aclamación muestra una nota. Se decidió sin votación, por eso no tiene votos.

### Preguntas de segunda vuelta instantánea {#instant-runoff-contests}

Para una pregunta de **Segunda Vuelta Instantánea**, la página también muestra cada **Ronda**. Cada ronda muestra los votos de cada **Candidato**, el candidato **Eliminado** y, al final, el **Ganador**. Si una ronda se detuvo en un empate, consulte [Resolver un empate](16-admin_portal_tutorials_tally-ceremony.md#resolve-a-tie).

## Descargar los documentos de resultados {#download-the-result-documents}

Cada nivel de los resultados tiene un menú de acciones. El menú solo muestra los formatos que el recuento generó para ese elemento.

| Nivel | Dónde | Elemento del menú |
| --- | --- | --- |
| Proceso electoral | El título **Resultados & Participación** | "Export in ... format - '...' results" |
| Elección | La pestaña de la elección | Igual, y además "Export All Areas Results in ... format for '...'" |
| Pregunta | La pestaña de la pregunta | "Export in ... format - '...' results" |
| Área | La pestaña del área | "Export in ... format - '...' results" |

| Formato | Contenido |
| --- | --- |
| **PDF** | El documento de resultados, listo para imprimir y firmar. |
| **HTML** | El documento de resultados, para un navegador. |
| PDF desde **HTML** | El menú también tiene un elemento PDF junto a cada elemento HTML. La plataforma genera el PDF a partir del documento HTML cuando hace clic en él. |
| **JSON** | Los resultados como datos, para otros sistemas. |
| **TAR_GZ** | Un archivo comprimido con los archivos de resultados. |
| Todas las áreas, **HTML**, **JSON** o **PDF** | Para una elección: los resultados de todas sus áreas en un solo archivo. No está disponible para un informe de inicialización. |
| **XLSX** | Solo para el proceso electoral: los resultados en una hoja de cálculo. |

El PDF desde HTML y el archivo XLSX los genera una tarea. La tarea aparece en el panel de tareas del portal de administración, como **Renderizar el documento PDF** o **Exportar los resultados del escrutinio en formato XLSX**. Cuando la tarea termina, el navegador guarda el archivo.

:::note
Si el despliegue usa paquetes de transmisión, el menú de una elección también tiene elementos para generarlos y enviarlos. Consulte [Transmisión](../02-reference/02-election-event/08-02-election_management_election-event_transmission.md).
:::

## Informes con resultados {#reports-with-results}

La pestaña **Informe** genera documentos a partir de plantillas. Estos tipos de informe tratan de resultados y participación:

| **Tipo de informe** | **Elección** | Acciones |
| --- | --- | --- |
| **Resultados Electorales** | Obligatoria | **Previsualizar** |
| **Informe de Inicialización** | Obligatoria | **Previsualizar** |
| **Informe de Participación** | Opcional | **Generar** y **Previsualizar**. También puede marcarlo como **Repetible**. |

Consulte [Informes y plantillas](18-reports_and_templates.md).

## Publicar los resultados en el sitio de resultados {#publish-the-results-on-the-results-website}

Los pasos principales están en [Obtener los resultados](../03-procedures/07-results.md#4-publish-the-results-on-the-results-website). Se aplican estas reglas:

- **Results Website** del proceso electoral debe estar **Habilitado**.
- El recuento debe estar completo. Antes, la sección muestra "Results can be published after this tally has completed."
- **Route**: **Event results** publica las elecciones del recuento. **Election results** publica una **Elección**.
- **Access**: **Public access** o **Authenticated access**. Con **Public access**, la visibilidad siempre es **Full published scope**.
- **Visibility**: **Full published scope** muestra todos los resultados publicados. **Personal visibility** muestra a cada votante con sesión iniciada solo los resultados del área de ese votante.
- Si el proceso electoral establece **Results Website Access** o **Results Website Visibility**, la sección usa esos valores y usted no puede cambiarlos.
- **Preguntas**: solo se muestran las preguntas con resultados. Todas están seleccionadas por defecto.

La confirmación dice "This will create a new publication from the current tally execution. The existing voter-facing results stay active until this publish task succeeds."

Cada publicación recibe una nueva **Version** en el **Publication history**. **Open** y **Revoke** solo se muestran para una publicación que está publicada. **Revoke** la quita del sitio de resultados. La plataforma registra cada publicación y cada revocación en el registro electoral.

| Permiso | Le permite |
| --- | --- |
| **Editar Publicación de Resultados** | Publicar y revocar. |
| **Leer Publicación de Resultados** | Ver el **Publication history**. |

Para el sitio de resultados, consulte [Sitio de resultados](../02-results-website.md).

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| El botón **Resultados** no está disponible. | El recuento no está completo. Espere hasta que el estado sea `SUCCESS`. |
| El menú de acciones no se muestra. | Pida el permiso **Exportar Ceremonia**. |
| Un formato no está en el menú. | El recuento no generó ese documento para el elemento. Contacte con el soporte de Sequent. |
| El archivo PDF o XLSX no se descarga. | Mire el panel de tareas. Si la tarea falló, vuelva a intentarlo y, después, contacte con el soporte de Sequent. |
| "Results website publishing is disabled for this election event. Enable it in the election event data before publishing results." | Ponga **Results Website** en **Habilitado** en la pestaña **Datos** del proceso electoral. |
| "You need publish-results-write permission to publish or revoke results." | Pida el permiso **Editar Publicación de Resultados**. |
| "You need publish-results-read permission to view publication history." | Pida el permiso **Leer Publicación de Resultados**. |
| "No tallied contests available." | La ruta seleccionada no tiene ninguna pregunta con resultados. Seleccione otra ruta o elección. |
| "Could not start results publication" | Vuelva a intentarlo. Si el mensaje continúa, contacte con el soporte de Sequent. |
| "Could not revoke results publication" | Vuelva a intentarlo. Si el mensaje continúa, contacte con el soporte de Sequent. |
| El número de votos no es el que espera. | No publique los resultados. Revise la publicación, las áreas y la pestaña **Logs**, y después contacte con el soporte de Sequent. |
