---
id: election_management_contest_tally_sheets
title: Hojas de Recuento
description: "Una hoja de recuento contiene los resultados de una urna que la plataforma no contó, por ejemplo papeletas en papel o papeletas por correo."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Hojas de Recuento

:::note
La pregunta no tiene una pestaña **Hojas de Recuento** en esta versión del portal de administración. La pregunta solo tiene la pestaña **Datos**. Las hojas de recuento están en la pestaña **Hojas de Recuento** de la **elección**. Esta página describe esa pestaña.
:::

Una hoja de recuento contiene los resultados de una urna que la plataforma no contó, por ejemplo papeletas en papel o papeletas por correo. Un administrador electoral escribe en el portal de administración los resultados de la urna. Una segunda persona con permiso de revisión aprueba o desaprueba la hoja de recuento. El recuento suma los resultados de las hojas de recuento aprobadas a los resultados de los votos en línea.

Una urna es una combinación de una pregunta, un área y un canal. Cada urna puede tener más de una hoja de recuento. Cada hoja de recuento de una urna es una **versión**.

Para importar hojas de recuento desde un archivo, use la pestaña **Importación de actas** del proceso electoral. Consulte [Importación de actas](../02-election-event/08-03-election_management_election-event_tally-sheet-imports.md).

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, abra el proceso electoral.
2. Haga clic en la elección.
3. Haga clic en la pestaña **Hojas de Recuento**.

La pestaña muestra el título **Urnas** y el texto "Urnas digitalizadas por canal".

## Permisos {#permissions}

| Permiso (en palabras sencillas) | Qué puede hacer |
| --- | --- |
| Ver hojas de recuento | Ver la pestaña **Hojas de Recuento**, las urnas y sus versiones. |
| Crear hojas de recuento | Crear una hoja de recuento, o una nueva versión de una hoja de recuento. |
| Revisar hojas de recuento | Aprobar o desaprobar una versión de una hoja de recuento. |
| Ver importaciones de actas | Ver la importación que creó una versión, y descargar su archivo de origen. |

:::danger ADVERTENCIA
Dé el permiso para crear hojas de recuento y el permiso para revisar hojas de recuento a personas distintas. Así, una segunda persona comprueba cada hoja de recuento antes de que el recuento la cuente.
:::

## Lista de urnas {#ballot-box-list}

La lista muestra una fila por cada urna de la elección.

| Columna | Significado |
| --- | --- |
| **ID** | El identificador de una hoja de recuento de la urna. Está oculto por defecto. Use **Columnas** para mostrarlo. |
| **Canal** | El canal de la urna: `PAPER` o `POSTAL`. |
| **Pregunta** | La pregunta de la urna. |
| **Área** | El área de la urna. |
| **Última versión** | El número de la versión más reciente. |
| **Versión aprobada** | El número de la versión aprobada. Aparece un guion (-) si no hay ninguna versión aprobada. |
| **Etiquetas** | Etiquetas de la hoja de recuento, si las hay. |
| **Anotaciones** | Anotaciones de la hoja de recuento, si las hay. |
| **Acciones** | **Añadir** crea una versión nueva a partir de la última versión. **Versiones** abre la lista de versiones. |

| Botón | Uso |
| --- | --- |
| **Añadir** | Crear una hoja de recuento para una urna nueva. |
| **Generar Hoja de Recuento** | Aparece cuando la elección no tiene hojas de recuento ("No hay Hojas de Recuento."). Crea la primera hoja de recuento. |
| **Columnas** | Mostrar u ocultar columnas. |
| **Añadir filtro** | Filtrar por **Área**, **Pregunta**, **ID**, **Canal**, **Última versión**, **Etiquetas** o **Anotaciones**. |

## Formulario de hoja de recuento {#tally-sheet-form}

El formulario tiene el título **Hoja de Recuento**. Los pasos de la parte superior son **Editar** y **Confirmar**. Para una urna nueva, el botón para ir al paso **Confirmar** es **Siguiente**. Para una versión nueva, es **Confirmar**.

| Campo | Significado |
| --- | --- |
| **Buscar pregunta** | La pregunta de la urna. Obligatorio. |
| **Buscar área** | El área de la urna. La lista muestra las áreas de la pregunta seleccionada. Obligatorio. |
| **Canal** | `PAPER` o `POSTAL`. Obligatorio. |
| **Votos Totales** | Calculado: **Votos Válidos Totales** más **Votos Inválidos Totales**. No puede escribirlo. |
| **Votos Válidos Totales** | El número de votos válidos. Obligatorio. |
| **Votos Inválidos Totales** | Calculado: **Votos Implícitamente Inválidos** más **Votos Explícitamente Inválidos**. No puede escribirlo. |
| **Votos Implícitamente Inválidos** | El número de votos implícitamente inválidos de la urna. Obligatorio. |
| **Votos Explícitamente Inválidos** | El número de votos explícitamente inválidos de la urna. Obligatorio. |
| **Votos en Blanco Totales** | El número de votos en blanco de esta pregunta. Obligatorio. |
| **Papeletas en Blanco** | El número de papeletas completamente en blanco de la urna. Opcional. Debe ser el mismo en cada pregunta de la urna. |
| **Censo** | El número de votantes de la urna. Obligatorio. **Votos Totales** no puede ser mayor que **Censo**. |
| **Candidatos** | Un campo **Votos Totales** para cada candidato. Obligatorio. |

El portal de administración comprueba los números mientras escribe. Si un número no es correcto, aparece un mensaje rojo debajo del campo y no puede pasar al paso siguiente.

## Crear una hoja de recuento {#make-a-tally-sheet}

1. En la pestaña **Hojas de Recuento**, haga clic en **Añadir**. Si la elección no tiene hojas de recuento, haga clic en **Generar Hoja de Recuento**.
2. En **Buscar pregunta**, seleccione la pregunta.
3. En **Buscar área**, seleccione el área.
4. En **Canal**, seleccione el canal.
5. Escriba los números de la urna en cada campo.
6. Escriba los votos de cada candidato.
7. Asegúrese de que no aparece ningún mensaje rojo.
8. Haga clic en **Siguiente**.
9. Compare los números de la pantalla con el registro en papel de la urna.
10. Si un número no es correcto, haga clic en **Atrás** y corríjalo.
11. Haga clic en **Guardar**.

**Resultado esperado:** aparece el mensaje "Hoja de Recuento creada". La urna aparece en la lista. La nueva versión tiene el estado `PENDING`.

## Corregir una hoja de recuento {#correct-a-tally-sheet}

No puede cambiar una versión guardada. Para corregir una hoja de recuento, cree una nueva versión.

1. En la fila de la urna, haga clic en **Añadir**.
2. Corrija los números. La pregunta, el área y el canal no cambian.
3. Haga clic en **Confirmar**.
4. Haga clic en **Guardar**.

**Resultado esperado:** aparece el mensaje "Hoja de Recuento creada". El número de **Última versión** aumenta en uno. La nueva versión tiene el estado `PENDING`.

## Versiones {#versions}

Haga clic en **Versiones** en la fila de una urna. Se abre la página "Versiones de la urna". Muestra el canal, el área y la pregunta, y una fila por cada versión, de la más reciente a la más antigua.

| Columna | Significado |
| --- | --- |
| **Versión** | El número de la versión. |
| **Creado por** | El usuario que creó la versión. |
| **Creado el** | La fecha y la hora en que el usuario creó la versión. |
| **Revisado por** | El usuario que aprobó o desaprobó la versión. |
| **Revisado el** | La fecha y la hora de la revisión. Aparece un guion (-) si nadie la ha revisado. |
| **Estado** | El estado de la revisión. La tabla siguiente muestra los valores. |
| **Etiquetas** | Etiquetas de la versión, si las hay. |
| **Anotaciones** | Anotaciones de la versión, si las hay. |
| **Importación de origen** | Para una versión creada por una importación: el estado de la importación, un enlace para abrir la importación y un enlace para descargar el archivo de origen. Aparece un guion (-) en una versión que usted escribió. |
| **Acciones** | **Mostrar**, **Aprobar** y **Desaprobar**. |

| **Estado** | Significado |
| --- | --- |
| `PENDING` | La versión espera una revisión. El recuento no la cuenta. |
| `APPROVED` | Un revisor aprobó la versión. El recuento la cuenta. |
| `DISAPPROVED` | Un revisor desaprobó la versión. El recuento no la cuenta. |

Haga clic en **Atrás** para volver a la lista de urnas.

## Revisar una hoja de recuento {#review-a-tally-sheet}

Necesita permiso para revisar hojas de recuento. Las acciones **Aprobar** y **Desaprobar** solo aparecen en una versión con el estado `PENDING`.

:::caution PRECAUCIÓN
Apruebe solo la versión correcta. Cuando aprueba una versión, la plataforma elimina todas las demás versiones de la urna, también las versiones más recientes que esperan una revisión.
:::

1. En la pestaña **Hojas de Recuento**, haga clic en **Versiones** en la fila de la urna.
2. En la fila de la versión, haga clic en **Mostrar**.
3. Compare los números con el registro en papel de la urna.
4. Haga clic en **Atrás**.
5. Haga clic en **Versiones** en la fila de la urna.
6. En la fila de la versión, haga clic en **Aprobar** o **Desaprobar**.
7. Lea el mensaje "¿Está seguro de aprobar esta Hoja de Recuento?" o "¿Está seguro de desaprobar esta Hoja de Recuento?".
8. Haga clic en **Aprobar** o **Desaprobar**.

**Resultado esperado:** aparece el mensaje "Hoja de Recuento revisada". Cambia el estado de la versión.

:::note
El recuento lee las hojas de recuento aprobadas cuando se ejecuta. Si aprueba una hoja de recuento después de que un recuento esté terminado, vuelva a realizar el recuento. Consulte [Volver a contar](../../03-procedures/06-tally.md#count-again).
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Todos los campos son obligatorios" | Rellene todos los campos obligatorios. Después, haga clic en **Siguiente** o **Confirmar** de nuevo. |
| **Siguiente** o **Confirmar** no está disponible. | Lea los mensajes rojos debajo de los campos. Corrija los números. |
| "El total de votos (…) no puede ser mayor que el censo (…)" | Compruebe que **Censo** y los números de votos son correctos. |
| "Los votos de candidatos (…) deben estar entre … y … según las reglas de votación de esta contienda (…)" | El total de los votos de candidatos no coincide con **Votos Válidos Totales**, **Votos en Blanco Totales** y el número de marcas de cada papeleta. Corrija los números. |
| "Las Papeletas en Blanco deben tener el mismo valor en todas las hojas de escrutinio de esta urna" | Use el mismo número de **Papeletas en Blanco** en cada pregunta de la urna. Este mensaje es una advertencia. No impide guardar. |
| "El valor de Papeletas en Blanco está fuera del rango que implican los recuentos de votos en blanco por contienda de esta urna" | Compruebe que **Papeletas en Blanco** coincide con los **Votos en Blanco Totales** de cada pregunta. Este mensaje es una advertencia. |
| "El algoritmo de recuento de esta contienda (…) no se reconoce, …" | Revise la configuración de la pregunta. |
| "Error creando Hoja de Recuento" | Compruebe que tiene permiso para crear hojas de recuento. Una pregunta resuelta por aclamación no puede tener hojas de recuento. Consulte [Preguntas resueltas por aclamación](../11-acclaimed-contests.md). |
| "Error revisando la Hoja de Recuento" | Compruebe que tiene permiso para revisar hojas de recuento. Solo se puede revisar una versión `PENDING`. |
| El recuento falla con un mensaje que empieza por "Approved tally sheets cannot be counted when voter-weighted voting is enabled". | El proceso electoral usa el voto ponderado por votante. Las hojas de recuento no se pueden usar con él. Contacte con el soporte de Sequent. |

## Páginas relacionadas {#related-pages}

- [Importación de actas](../02-election-event/08-03-election_management_election-event_tally-sheet-imports.md)
- [Datos de la pregunta](01-election_management_contest_data.md)
- [Realizar la ceremonia de recuento](../../03-procedures/06-tally.md)
