---
id: reports_and_templates
title: Informes y plantillas
description: "Este tutorial crea un documento de informe con su propia plantilla. Usted crea la plantilla en la página Plantillas, la vincula a un informe en la pestaña Informe, la comprueba con Vista previa y genera el documento."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Este tutorial crea un documento de informe con su propia plantilla. Usted crea la plantilla en
la página **Plantillas**, la vincula a un informe en la pestaña **Informe**, la comprueba con
**Vista previa** y genera el documento. El mismo método se aplica a todos los tipos de informe,
por ejemplo **Resultados Electorales** o **Informe Estadístico**.

## Antes de empezar {#before-you-start}

- Su rol puede editar plantillas (**Editar Plantilla de Comunicación**) y crear y generar
  informes (**Crear Informe**, **Generar Informe**, **Vista Previa del Informe**).
- El proceso electoral existe. Para los informes de resultados, el recuento está completo.

## 1. Crear la plantilla {#1-make-the-template}

1. En el menú de la izquierda, haga clic en **Plantillas**.
2. Haga clic en **Añadir**. Si la lista está vacía, haga clic en **Crear Plantilla**.
3. Escriba un **Alias de la Plantilla**, por ejemplo `results-election-1`.
4. Escriba el **Nombre de la Plantilla**.
5. En **Tipo**, seleccione el tipo del informe, por ejemplo **Resultados Electorales**.
6. En **Elige métodos**, seleccione **Documento**.
7. Escriba el cuerpo del documento en **Documento**.
8. Si es necesario, establezca **Opciones de PDF** y **Opciones de Informe** en formato JSON.
9. Haga clic en **Guardar**.

**Resultado esperado:** se muestra el mensaje "Plantilla creada".

:::note
Si no crea una plantilla, la plataforma usa su plantilla predeterminada para el tipo de informe.
Cree una plantilla solo cuando deba cambiar el texto o el diseño.
:::

## 2. Vincular la plantilla a un informe {#2-link-the-template-to-a-report}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Informe**.
3. Haga clic en **Añadir**. Si la lista está vacía, haga clic en **Crear informe**.
4. En **Tipo de informe**, seleccione el mismo tipo que la plantilla.
5. En **Elección**, seleccione la elección, si el tipo la usa.
6. En **Plantilla**, seleccione el alias de su plantilla.
7. En **Política de cifrado**, seleccione **Sin cifrar** o **Contraseña configurada**.
8. Haga clic en **Guardar**.

**Resultado esperado:** se muestra el mensaje "Informe creado con éxito". El informe se muestra
en la lista.

## 3. Comprobar la plantilla con datos de ejemplo {#3-check-the-template-with-sample-data}

1. En la fila del informe, abra el menú de acciones.
2. Haga clic en **Vista previa**.
3. Espere a que termine la tarea y después descargue el archivo.
4. Compruebe el texto y el diseño.

Si el documento no es correcto, cambie la plantilla en la página **Plantillas** y vuelva a ver
la vista previa.

## 4. Generar el documento {#4-generate-the-document}

1. En la fila del informe, abra el menú de acciones.
2. Haga clic en **Generar**.
3. Espere a que termine la tarea "Generar Reporte".
4. Haga clic en **Descargar Archivo**.

**Resultado esperado:** tiene el documento con los datos reales.

## Informes repetibles {#repeatable-reports}

Algunos tipos de informe pueden ejecutarse según una programación. Para estos tipos, el
formulario del informe muestra **Repetible**. Actívelo, escriba la **Expresión Cron** y añada
los **Destinatarios de correo electrónico**. Escriba cada dirección y pulse Intro.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La plantilla no está en **Plantilla**. | Compruebe que el **Tipo** de la plantilla es el mismo que el tipo del informe. |
| El documento no usa su plantilla. | Consulte [Asociación de valores a plantillas](value_to_template_association.md). |
| La tarea muestra `FAILED`. | Abra la tarea en la pestaña **Tareas** y lea los registros. Compruebe las variables de la plantilla. |

## Páginas relacionadas {#related-pages}

- [Plantillas](../../Reference/User-Manual/Templates/admin_portal_reference_user-manual_templates.md)
- [Proceso electoral: Informes](../../Reference/User-Manual/Election-Management/Election-Event/Reports/election_management_election-event_reports.md)
- [Obtener los resultados](../../procedures/07-results.md)
