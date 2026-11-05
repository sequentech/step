---
id: admin_portal_tutorials_ballot-boxes
title: Urnas (hojas de recuento)
description: "Este tutorial explica cómo digitalizar los resultados de canales de votación no electrónicos (como las papeletas en papel o postales) usando urnas, y cómo revisarlos y aprobarlos para que se incluyan en el recuento final junto con los votos electrónicos…"
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Este tutorial explica cómo digitalizar los resultados de canales de votación no electrónicos (como las papeletas **Papel** o **Postal**) usando **Urnas**, y cómo revisarlos y aprobarlos para que se incluyan en el recuento final junto con los votos electrónicos.

:::info
Una urna (llamada internamente **Hoja de Recuento**) es el recuento digitalizado de votos para una combinación única de **Pregunta**, **Área** y **Canal**. Cada vez que envía nuevas cifras para esa combinación, se crea una nueva **versión**. No se sobrescribe nada, así que siempre conserva un registro completo de quién introdujo y quién revisó cada versión.
:::

## Paso 1: Abra la pestaña Hojas de Recuento {#step-1-open-the-tally-sheets-tab}

Las urnas se gestionan a nivel de **Elección** (no por pregunta).

1. Vaya al **Evento Electoral** y seleccione la **Elección** concreta.
2. Seleccione la pestaña **Hojas de Recuento**.

El encabezado de la página dice **Urnas — Urnas digitalizadas por canal**.

:::info
Esta pestaña solo es visible para los usuarios con el permiso **Ver Acta de Recuento**. Crear urnas requiere **Crear Acta de Recuento**, y aprobar o desaprobar versiones requiere **Revisar hoja de recuento**.
:::

## Paso 2: Entienda la lista de urnas {#step-2-understand-the-ballot-boxes-list}

La lista principal muestra una fila por cada combinación única de **Pregunta + Área + Canal**, con:

* **Canal**: por ejemplo `PAPER` o `POSTAL`.
* **Pregunta** y **Área** a las que pertenece la urna.
* **Última versión**: el número de versión de las cifras enviadas más recientemente.
* **Versión aprobada**: el número de versión que se ha aprobado y se usará en el recuento final (`-` si todavía no se ha aprobado ninguna).

Cada fila tiene dos acciones disponibles en el menú de acciones:

* **Añadir**: envía una nueva versión de cifras para esa urna.
* **Versiones**: abre el historial completo de versiones de esa urna.

## Paso 3: Digitalice una urna nueva {#step-3-digitalize-a-new-ballot-box}

1. Seleccione el botón **+ Añadir** encima de la lista. Si todavía no hay urnas, seleccione **Generar Hoja de Recuento** en la pantalla vacía.
2. Elija la **Pregunta**, el **Área** y el **Canal** a los que pertenecen las cifras.
3. Introduzca los recuentos de votos:
   * **Total de votos válidos**
   * **Votos Implícitamente Inválidos** y **Votos Explícitamente Inválidos**. El campo **Total de votos inválidos** se calcula automáticamente como su suma.
   * **Votos en Blanco**
   * **Censo**. Debe introducirlo manualmente y debe ser mayor o igual que el total de votos.
   * El número de votos de cada **Candidato**
4. Seleccione **Confirmar**. El sistema comprueba que la suma de los votos de los candidatos más los votos en blanco es igual al total de votos válidos, y que el censo es suficiente. Si no lo es, aparece un error y no puede continuar.
5. Revise el resumen de solo lectura y seleccione **Guardar**.

Esto crea la **versión 1** de la urna, con el estado **Pendiente**.

:::tip
No puede crear una segunda urna independiente para la misma combinación de Pregunta + Área + Canal. Para corregir o actualizar las cifras de una urna existente, use la acción **Añadir** de la lista (vea el paso siguiente).
:::

## Paso 4: Envíe una versión nueva (corrección) {#step-4-submit-a-new-version-correction}

Si hay que corregir cifras (por ejemplo, después de un nuevo recuento), no cree una urna nueva. Añada una versión nueva a la existente:

1. En la lista de urnas, seleccione **Añadir** en la fila de la urna que quiere actualizar.
2. El formulario se abre con los valores de la última versión. Edite los campos que necesite.
3. Seleccione **Confirmar**, revise el resumen y seleccione **Guardar**.

Esto crea una nueva versión (por ejemplo, la versión 2) con el estado **Pendiente**, sin afectar todavía a las versiones anteriores.

## Paso 5: Revise el historial de versiones {#step-5-review-version-history}

1. En la lista de urnas, seleccione **Versiones** en la fila de la urna que quiere revisar.
2. La tabla de versiones muestra, para cada versión enviada de esa combinación de Pregunta + Área + Canal:
   * **Versión**
   * **Creado por** y **Creado el**
   * **Revisado por** y **Revisado el** (`-` hasta que se revise)
   * **Estado**: `PENDING`, `APPROVED` o `DISAPPROVED`
3. Use **Mostrar** para ver las cifras de solo lectura de cualquier versión.
4. Seleccione **Atrás** para volver a la lista de urnas.

## Paso 6: Apruebe o desapruebe una versión {#step-6-approve-or-disapprove-a-version}

Las versiones pendientes se pueden aprobar o desaprobar desde la tabla de versiones. Para ello se requiere el permiso **Revisar hoja de recuento**.

1. Seleccione **Aprobar** o **Desaprobar** en una versión pendiente.
2. Confirme la acción en el cuadro de diálogo.

:::info
Aprobar una versión cambia su estado a **Aprobada** y registra el revisor y la hora de revisión. También descarta automáticamente las otras versiones de esa misma urna, así que solo queda una versión por urna como registro aprobado y oficial.
:::

Desaprobar una versión la marca como **Desaprobada**. Después, puede enviar una nueva versión corregida con **Añadir**.

:::tip
Solo se incluyen en la ceremonia de recuento las urnas **Aprobadas**. Sus cifras se combinan con los votos emitidos electrónicamente y descifrados para producir los resultados finales. Asegúrese de que cada urna pertinente tiene una versión aprobada antes de realizar el recuento.
:::
