---
title: Obtener los resultados
sidebar_position: 7
description: "Este procedimiento muestra los resultados de un recuento y descarga los documentos de resultados. El administrador electoral hace este procedimiento."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Obtener los resultados

Este procedimiento muestra los resultados de un recuento y descarga los documentos de resultados.
El administrador electoral hace este procedimiento.

## Antes de empezar {#before-you-start}

- El estado del recuento es `SUCCESS`. Consulte [Realizar la ceremonia de recuento](06-tally.md).
- Su cuenta tiene el permiso para exportar los resultados.

## 1. Ver los resultados {#1-look-at-the-results}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Recuento**.
3. Haga clic en el icono de ver del recuento.
4. Haga clic en **Resultados**.
5. Abra **Resultados & Participación**.
6. Seleccione la elección y después la pregunta. En **Áreas**, seleccione **Global** o un área.

**Resultado esperado:** la página muestra:

- El **Resumen de participación**: los votantes elegibles, los votos contados y los votos válidos,
  nulos y en blanco, con sus porcentajes.
- Los **Resultados de Candidatos**: los votos de cada candidato y las posiciones ganadoras.

7. Compare **Total de Votos Contados** con **Votantes** en la pestaña **Panel de Control** de la
   elección.

## 2. Descargar los documentos de resultados {#2-download-the-result-documents}

Puede descargar los resultados del proceso electoral, de una elección, de una pregunta o de un
área.

1. En la página de resultados, abra el menú de acciones del elemento. Los elementos del menú
   tienen la forma "Exportar en formato PDF - Resultados de 'Elección 1'".
2. Haga clic en el formato que quiere descargar. El menú muestra los formatos que existen para el
   elemento.

| Formato | Contenido |
| --- | --- |
| **PDF** | El documento de resultados, listo para imprimir y firmar. |
| **HTML** | El documento de resultados, para un navegador. También puede crear un PDF a partir de él. |
| **JSON** | Los resultados en un formato de datos, para otros sistemas. |
| **TAR_GZ** | Un archivo comprimido con todos los archivos de resultados. |
| **RECEIPTS_PDF** | Los recibos de voto. |

3. Guarde los archivos en un lugar seguro.

**Resultado esperado:** el navegador guarda el archivo.

## 3. Crear los informes {#3-make-the-reports}

La pestaña **Informe** crea otros documentos, por ejemplo el informe de resultados electorales, un
informe estadístico o el registro de actividad.

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Informe**.
3. Haga clic en **Añadir**. Si la lista está vacía, haga clic en **Crear informe**.
4. En **Tipo**, seleccione el tipo de informe, por ejemplo **Resultados Electorales**.
5. Seleccione la **Elección** y la **Plantilla**, si es necesario.
6. En **Política de cifrado**, seleccione **Sin cifrar** o **Contraseña configurada**.
7. Si selecciona **Contraseña configurada**, escriba la contraseña en **Contraseña:** y en
   **Repetir Contraseña:**, y después haga clic en **Guardar contraseña**.
8. Haga clic en **Guardar**.
9. En la fila del informe, abra el menú de acciones y haga clic en **Generar**.

**Resultado esperado:** se inicia la tarea "Generar Reporte". Cuando termine, haga clic en
**Descargar Archivo** en el panel de la tarea, o en la pestaña **Tareas**.

:::caution PRECAUCIÓN
Guarde la contraseña del informe en un lugar seguro. No puede cambiar la política de cifrado de un
informe después de definir una contraseña.
:::

## 4. Publicar los resultados {#4-publish-the-results}

La versión 9.0 no publica los resultados en un sitio web público. Dé los documentos de resultados
a las personas que anuncian los resultados.

## 5. Conservar los registros {#5-keep-the-records}

1. Exporte el proceso electoral para su archivo. En la pestaña **Datos**, haga clic en
   **Exportar**, seleccione **Cifrar con Contraseña** y el contenido que quiere incluir, y después
   haga clic en **Exportar**.
2. Copie la contraseña que aparece en la ventana **Contraseña** y guárdela en un lugar seguro.
3. Cuando la tarea "Exportar evento electoral" esté completa, haga clic en **Descargar Archivo**.
4. Guarde juntos el archivo exportado, los documentos de resultados y los registros de las
   ceremonias.
5. Guarde los fragmentos de clave hasta el final del período de reclamaciones. Después,
   destrúyalos como exigen sus normas.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| El botón **Resultados** no está disponible. | El recuento no está completo. Espere hasta que el estado sea `SUCCESS`. |
| Un formato no está en el menú. | La plataforma no ha creado ese documento para el elemento. Contacte con el soporte de Sequent. |
| El número de votos no es el que espera. | No publique los resultados. Compruebe la publicación, las áreas y la pestaña **Logs**, y después contacte con el soporte de Sequent. |
| El menú de acciones de los resultados no está. | Su cuenta no tiene el permiso **Exportar Ceremonia**. Pregunte al administrador del inquilino. |
| "Las contraseñas no coinciden. Asegúrate de que ambos campos contengan la misma contraseña." | Escriba la misma contraseña del informe en los dos campos. |

## Más información {#more-information}

- [Proceso electoral: Recuento](../Reference/User-Manual/Election-Management/Election-Event/Tally/election_management_election-event_tally.md)
- [Proceso electoral: Transmisión](../Reference/User-Manual/Election-Management/Election-Event/Tally/election_management_election-event_transmission.md)
- [Proceso electoral: Informes](../Reference/User-Manual/Election-Management/Election-Event/Reports/election_management_election-event_reports.md)
- [Informes y plantillas](../Tutorials/Reports-and-Templates/reports_and_templates.md)
- [Asociación de valores a plantillas](../Tutorials/Reports-and-Templates/value_to_template_association.md)
- [Proceso electoral: Datos](../Reference/User-Manual/Election-Management/Election-Event/Data/election_management_election-event_data.md)
- [Proceso electoral: Tareas](../Reference/User-Manual/Election-Management/Election-Event/Tasks/election_management_election-event_tasks.md)
