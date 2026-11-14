---
title: Obtener los resultados
sidebar_position: 7
description: "Este procedimiento muestra los resultados de un recuento, descarga los documentos de resultados y publica los resultados en el sitio web de resultados. Lo realiza el administrador electoral."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Obtener los resultados

Este procedimiento muestra los resultados de un recuento, descarga los documentos de resultados y
publica los resultados en el sitio web de resultados. Lo realiza el administrador electoral.

## Antes de empezar {#before-you-start}

- El estado del recuento es `SUCCESS`. Consulte [Realizar la ceremonia de recuento](06-tally.md).
- Su cuenta tiene el permiso **Leer Resultados de Recuento** para ver los resultados, y el permiso
  **Exportar Ceremonia** para descargarlos.

## 1. Consultar los resultados {#1-look-at-the-results}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Recuento**.
3. Haga clic en el icono de ver del recuento.
4. Haga clic en **Resultados**.
5. Abra **Resultados & Participación**.
6. Seleccione la elección, la pregunta y el área que desea ver.

**Resultado esperado:** la página muestra:

- El **Resumen de participación**: los votantes elegibles, los votantes, y los votos válidos,
  inválidos y en blanco, con sus porcentajes.
- La participación por canal.
- Los **Resultados de Candidatos**: los votos de cada candidato y las posiciones ganadoras. Para una
  pregunta de **Segunda Vuelta Instantánea**, la página también muestra cada ronda, los candidatos
  eliminados y el ganador.

7. Compare el número de votos contados con el número de votos emitidos en la pestaña
   **Panel de Control**.

## 2. Descargar los documentos de resultados {#2-download-the-result-documents}

Puede descargar los resultados del proceso electoral, de una elección, de una pregunta o de un área.

1. En la página de resultados, abra el menú de acciones del elemento.
2. Haga clic en el formato que desea descargar. El menú muestra los formatos que existen para el
   elemento.

| Formato | Contenido |
| --- | --- |
| **PDF** | El documento de resultados, listo para imprimir y firmar. |
| **HTML** | El documento de resultados, para un navegador. También puede crear un PDF a partir de él. |
| **JSON** | Los resultados en un formato de datos, para otros sistemas. |
| **TAR_GZ** | Un archivo con todos los archivos de resultados. |

Para una elección, el menú también puede exportar los resultados de todas sus áreas en un único
archivo HTML o JSON. Para el proceso electoral, también puede exportar los resultados en formato
**XLSX**, para una hoja de cálculo.

3. Guarde los archivos en un lugar seguro.

**Resultado esperado:** el navegador guarda el archivo.

## 3. Crear los informes {#3-make-the-reports}

La pestaña **Informes** crea otros documentos, por ejemplo el informe de resultados electorales, un
informe estadístico o el registro de actividad.

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Informes**.
3. Haga clic en **Crear informe**.
4. Seleccione el **Tipo de informe**, por ejemplo **Resultados Electorales** o **Informe de
   Participación**.
5. Seleccione la **Elección** y la **Plantilla**.
6. En **Política de cifrado**, seleccione **Contraseña configurada** para proteger el archivo con una
   contraseña, o **Sin cifrar**.
7. Haga clic en el botón de guardar.
8. En la fila del informe, haga clic en **Vista previa** para ver el documento. Para los tipos
   **Registros de Actividad** e **Informe de Participación**, haga clic en **Generar** para crear el
   archivo.

**Resultado esperado:** el informe aparece en la vista previa, o está disponible para descargar
cuando su tarea ha terminado.

## 4. Publicar los resultados en el sitio web de resultados {#4-publish-the-results-on-the-results-website}

Haga este paso solo si su proceso electoral usa el sitio web de resultados.

### Antes de publicar {#before-you-publish}

- **Results Website** está **Habilitado** en la pestaña **Datos** del proceso electoral. Consulte
  [Crear el proceso electoral](02-event.md#2-configure-the-election-event).
- El estado del recuento es `SUCCESS`.
- Su cuenta tiene el permiso para publicar los resultados.
- Las personas responsables de la elección han aprobado los resultados.

:::caution PRECAUCIÓN
El público puede ver los resultados inmediatamente después de la publicación. Compruebe los
resultados antes de publicarlos.
:::

1. En la página de resultados del recuento, abra **Publish to results website**.
2. En **Route**, seleccione **Event results** o **Election results**. Para **Election results**,
   seleccione la **Election**.
3. En **Access**, seleccione **Public access** o **Authenticated access**.
4. En **Visibility**, seleccione el ámbito de los resultados. Con **Public access**, la visibilidad
   es siempre **Full published scope**. Si el proceso electoral fija el acceso o la visibilidad, no
   puede cambiar estos campos.
5. En **Contests**, seleccione las preguntas que desea publicar.
6. Haga clic en **Publish selected contests**.
7. Lea el mensaje "Start publish to results website?" y confirme.

**Resultado esperado:** aparece el mensaje "Results publication started". El **Publication history**
muestra una versión nueva.

8. En **Publication history**, haga clic en **Open** para ver la página publicada. **Open** aparece
   cuando la publicación está completa y Sequent ha configurado la dirección del sitio web de
   resultados.

Para quitar una publicación del sitio web de resultados, haga clic en **Revoke** en su fila de
**Publication history**.

Para más información, consulte [Sitio web de resultados](../02-results-website.md) y
[Resultados electorales](../01-tutorials/17-admin_portal_tutorials_election-results.md).

## 5. Guardar los registros {#5-keep-the-records}

1. Exporte el proceso electoral para su archivo. En la pestaña **Datos**, haga clic en el botón de
   exportar y seleccione el contenido que desea incluir. Seleccione **Cifrar con Contraseña**.
2. Guarde juntos el archivo de exportación, los documentos de resultados y los registros de las
   ceremonias.
3. Guarde los fragmentos de clave hasta el final del período de reclamaciones. Después, destrúyalos
   según sus normas.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| El botón **Resultados** no está disponible. | El recuento no está completo. Espere hasta que el estado sea `SUCCESS`. |
| "Results website publishing is disabled for this election event." | Establezca **Results Website** en **Habilitado** en la pestaña **Datos** del proceso electoral. |
| "Results can be published after this tally has completed." | Espere hasta que el estado del recuento sea `SUCCESS`. |
| Un formato no está en el menú. | La plataforma no creó ese documento para el elemento. Contacte con el soporte de Sequent. |
| El número de votos no es el que espera. | No publique los resultados. Revise la publicación, las áreas y la pestaña **Registros**, y después contacte con el soporte de Sequent. |

## Más información {#more-information}

- [Resultados electorales en detalle](../01-tutorials/17-admin_portal_tutorials_election-results.md)
- [Informes y plantillas](../01-tutorials/18-reports_and_templates.md)
- [Proceso electoral: Transmisión](../02-reference/02-election-event/08-02-election_management_election-event_transmission.md)
- [Proceso electoral: Informes](../02-reference/02-election-event/13-01-election_management_election-event_reports.md)
- [Registros electorales](../01-tutorials/19-admin_portal_tutorials_election-logs.md)
- [Exportar los datos del proceso electoral](../01-tutorials/20-admin_portal_tutorials_export-data.md)
- [Sitio web de resultados](../02-results-website.md)
