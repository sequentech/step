---
id: value_to_template_association
title: Asociación de valores a plantillas
description: "Este tutorial explica cómo la plataforma encuentra la plantilla de un documento, y cómo los valores de la elección llegan a la plantilla. Léalo cuando un documento no use la plantilla que espera."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Este tutorial explica cómo la plataforma encuentra la plantilla de un documento, y cómo los
valores de la elección llegan a la plantilla. Léalo cuando un documento no use la plantilla que
espera.

## Cómo selecciona la plataforma la plantilla {#how-the-platform-selects-the-template}

Una plantilla no se usa solo porque exista. La plataforma vincula un documento a una plantilla
mediante una fila de la pestaña **Informe**:

1. La plataforma necesita un documento de un tipo, por ejemplo **Resultados Electorales**, para
   un proceso electoral y, si la hay, una elección.
2. Busca en la pestaña **Informe** del proceso electoral un informe de ese tipo. Si el documento
   es para una elección, el informe debe ser para esa elección.
3. Si encuentra un informe con una **Plantilla**, usa la plantilla con ese alias de la página
   **Plantillas**.
4. Si no encuentra ningún informe, o el informe no tiene plantilla, usa la plantilla
   predeterminada del tipo.

La plataforma usa el primer informe que coincide. Cree solo un informe de cada tipo para cada
elección, para que el resultado sea claro.

| Lo que tiene | Plantilla que usa la plataforma |
| --- | --- |
| Ningún informe del tipo | La plantilla predeterminada. |
| Un informe del tipo sin **Plantilla** | La plantilla predeterminada. |
| Un informe del tipo con **Plantilla** | Su plantilla. |
| Un informe para otra elección | La plantilla predeterminada, para los documentos de las otras elecciones. |

## Cómo llegan los valores a la plantilla {#how-the-values-get-into-the-template}

Los textos de una plantilla contienen variables entre llaves dobles. Cuando la plataforma crea
el documento, reemplaza cada variable por un valor del proceso electoral, por ejemplo un nombre,
una fecha o un número de votos. La plantilla predeterminada de cada tipo muestra las variables
de ese tipo.

Para ver el resultado con valores de ejemplo, haga clic en **Vista previa** en el menú de
acciones del informe, en la pestaña **Informe**. La vista previa usa datos de ejemplo, no los
datos reales.

## Comprobar la asociación {#check-the-association}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Informe**.
3. Busque el informe del tipo de documento. Compruebe las columnas **Elección** y **Plantilla**.
4. Haga clic en **Vista previa** en el menú de acciones del informe.
5. Asegúrese de que el documento tiene el texto de su plantilla.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| El documento tiene el texto predeterminado. | Cree o cambie el informe de ese tipo en la pestaña **Informe**. Seleccione su plantilla en **Plantilla**. |
| Su plantilla no está en la lista **Plantilla** del informe. | El **Tipo** de la plantilla no es el tipo del informe. Cambie la plantilla. |
| El documento muestra un error o un valor vacío. | Una variable de la plantilla no es correcta. Compárela con la plantilla predeterminada del tipo. |

## Páginas relacionadas {#related-pages}

- [Informes y plantillas](reports_and_templates.md)
- [Plantillas](../../Reference/User-Manual/Templates/admin_portal_reference_user-manual_templates.md)
- [Proceso electoral: Informes](../../Reference/User-Manual/Election-Management/Election-Event/Reports/election_management_election-event_reports.md)
