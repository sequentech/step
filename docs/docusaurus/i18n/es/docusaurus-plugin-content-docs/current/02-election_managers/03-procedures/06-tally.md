---
title: Realizar la ceremonia de recuento
sidebar_position: 6
description: "La ceremonia de recuento descifra y cuenta los votos. El administrador electoral crea el recuento. Los fideicomisarios suben sus fragmentos de clave."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Realizar la ceremonia de recuento

La ceremonia de recuento descifra y cuenta los votos. El administrador electoral crea el recuento. Los fideicomisarios suben sus fragmentos de clave. Cuando suficientes fideicomisarios han subido su fragmento, el administrador electoral inicia el recuento.

## Antes de empezar {#before-you-start}

- El estado de la ceremonia de claves es `SUCCESS`. Consulte [Realizar la ceremonia de claves](04-keys.md).
- El proceso electoral tiene una publicación. Consulte [Publicar y gestionar el período de votación](05-publish.md).
- Todos los canales de votación están cerrados, salvo que haga un informe de inicialización.
- **Permitir Recuento** de cada elección permite el recuento. Consulte [Crear el proceso electoral](02-event.md#3-create-the-elections).
- Hay disponible un número de fideicomisarios igual o mayor que el umbral, con una copia de su fragmento de clave.
- Si la elección tiene actas de escrutinio (resultados de papel u otras urnas), cada acta está aprobada. Consulte [Actas de escrutinio](../02-reference/04-contest/02-election_management_contest_tally-sheets.md).
- Ha acordado una fecha y una hora con los fideicomisarios.

:::danger ADVERTENCIA
Inicie el recuento de los resultados de la elección solo después del fin oficial del período de votación. Si **Permitir Recuento** está en **Permitido**, el portal de administración le deja iniciar el recuento mientras los votantes todavía pueden votar. Use **Permitir cuando termine el periodo de votación** para evitarlo.
:::

## 1. Crear el recuento (administrador electoral) {#1-create-the-tally-election-administrator}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Recuento**.
3. Haga clic en **Iniciar la Ceremonia de Recuento**.
4. En **Elecciones para el Recuento**, seleccione las elecciones que se van a contar.
5. En **Ceremonia de Claves**, seleccione la ceremonia de claves de estas elecciones. Si solo hay una ceremonia de claves, el portal de administración la selecciona.
6. Haga clic en **Iniciar la Ceremonia de Recuento**. En una ceremonia de claves automática, el botón es **Iniciar Recuento**.
7. Lea el mensaje "Esta acción notificará a los trustees para importar sus fragmentos de clave."
8. Haga clic en **Ok**.

**Resultado esperado:** aparece el mensaje "Recuento creado". El recuento aparece en la lista de la pestaña **Recuento** con el estado `STARTED`.

9. Avise a los fideicomisarios de que la ceremonia de recuento ha empezado.

En una ceremonia de claves automática, los fideicomisarios no hacen nada y el recuento se ejecuta de inmediato. Vaya al [paso 3](#3-start-the-tally-election-administrator), elemento 7.

## 2. Subir el fragmento de clave (cada fideicomisario) {#2-upload-the-key-fragment-each-trustee}

Cada fideicomisario hace estos pasos con su propia cuenta. Necesita una copia de su fragmento de clave de su dispositivo de almacenamiento.

1. Inicie sesión en el portal de administración.
2. Abra el proceso electoral.
3. Haga clic en la pestaña **Recuento**. Aparece el mensaje "Ha sido invitado a participar en una ceremonia de recuento."
4. En la fila del recuento, haga clic en **Agregar Clave de Cómputo** de la columna **Acciones**.
5. Lea la lista de **Elecciones para el Recuento**. Asegúrese de que es la lista que espera.
6. Arrastre el archivo de su fragmento de clave al área de carga.
7. Asegúrese de que aparece el mensaje "Copia de seguridad verificada correctamente."
8. Haga clic en **Siguiente**.
9. Retire el dispositivo de almacenamiento y guárdelo en su lugar seguro.

**Resultado esperado:** el fideicomisario ha subido su fragmento de clave.

## 3. Iniciar el recuento (administrador electoral) {#3-start-the-tally-election-administrator}

1. En la pestaña **Recuento**, haga clic en el icono de ver del recuento.
2. Mire la sección **Fideicomisarios**. Muestra cuántos fideicomisarios han importado su clave y cuántos son necesarios.
3. Espere hasta que el estado sea `CONNECTED`. Este estado indica que suficientes fideicomisarios han subido su fragmento de clave.
4. Haga clic en **Iniciar Recuento**.
5. Lea el mensaje "¿Quiere iniciar el recuento?".
6. Haga clic en **Iniciar Recuento**.

**Resultado esperado:** aparece el mensaje "Recuento iniciado". El estado cambia a `IN_PROGRESS`. La tabla de progreso de los recuentos de las elecciones muestra el avance de cada elección.

7. Espere hasta que el estado sea `SUCCESS`. El tiempo depende del número de votos. Si el estado es `AWAITING_INPUT`, resuelva el empate. Consulte [Resolver un empate](#resolve-a-tie).
8. Haga clic en **Resultados**. Consulte [Obtener los resultados](07-results.md).
9. Registre la fecha, la hora, los fideicomisarios presentes y el resultado de la ceremonia.

:::caution PRECAUCIÓN
El recuento cuenta solo las actas de escrutinio con el estado `APPROVED`. Ignora las actas `PENDING` y `DISAPPROVED` y no muestra ningún mensaje. Apruebe las actas de escrutinio antes de crear el recuento.
:::

## Estado del recuento {#tally-status}

| Estado | Significado |
| --- | --- |
| `STARTED` | El recuento está creado. Los fideicomisarios pueden subir sus fragmentos de clave. |
| `CONNECTED` | Suficientes fideicomisarios han subido su fragmento de clave. Puede iniciar el recuento. |
| `IN_PROGRESS` | La plataforma mezcla, descifra y cuenta los votos. |
| `AWAITING_INPUT` | El recuento se ha detenido en un empate que necesita una decisión. |
| `SUCCESS` | El recuento está completo. Los resultados están disponibles. |
| `CANCELLED` | El recuento está cancelado. |

La tabla de progreso de los recuentos de las elecciones muestra estos pasos para cada elección: `WAITING`, `MIXING`, `DECRYPTING`, `SUCCESS` o `ERROR`.

## Resolver un empate {#resolve-a-tie}

Una pregunta de **Segunda Vuelta Instantánea** con la política de desempate **Procedimiento externo** puede detenerse en un empate. La página del recuento muestra entonces "Recuento en pausa por un empate sin resolver".

1. Obtenga la decisión sobre el empate, según lo exijan las reglas de su elección.
2. En la página del recuento, escriba la decisión de cada empate.
3. Haga clic en **Aplicar resoluciones y recalcular**.

**Resultado esperado:** el estado cambia a `IN_PROGRESS` y, después, a `SUCCESS`.

## Repetir el recuento {#count-again}

Puede volver a contar un recuento completado, por ejemplo después de un cambio aprobado de las actas de escrutinio.

1. En la pestaña **Recuento**, haga clic en **Repetir el recuento** en la fila del recuento.
2. Lea el mensaje "Esto generará un nuevo evento de resultados para la sesión de recuento completada."
3. Haga clic en **Repetir el recuento**.

## Cancelar un recuento {#cancel-a-tally}

Puede cancelar un recuento antes de que empiece, cuando su estado es `STARTED` o `CONNECTED`.

:::caution PRECAUCIÓN
No puede deshacer una cancelación. Los fideicomisarios deben subir su fragmento de clave otra vez para un recuento nuevo.
:::

1. En la pestaña **Recuento**, haga clic en **Cancelar la Ceremonia de Cómputo** en la fila del recuento.
2. Haga clic en **Cancelar recuento**.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "La Ceremonia de Recuento no puede comenzar hasta que la Ceremonia de Claves se haya completado con éxito." | Complete la ceremonia de claves. Consulte [Realizar la ceremonia de claves](04-keys.md). |
| "La Ceremonia de Escrutinio no puede comenzar hasta que crees una publicación en la pestaña Publicar." | Publique la papeleta. Consulte [Publicar y gestionar el período de votación](05-publish.md). |
| "Selecciona al menos una elección." | Seleccione una o más elecciones. |
| "Publica cada elección seleccionada antes de crear su recuento." | Publique la papeleta. Consulte [Publicar y gestionar el período de votación](05-publish.md). |
| "Finaliza la votación en cada elección seleccionada y detén sus canales activos antes de crear el recuento." | Cierre todos los canales de votación de las elecciones. Consulte [Publicar y gestionar el período de votación](05-publish.md#6-pause-or-close-the-voting-period). |
| "El recuento está deshabilitado para una elección seleccionada." | **Permitir Recuento** de la elección está en **No permitido**. Cámbielo en la pestaña **Datos** de la elección. |
| "Las elecciones tienen ceremonias de claves diferentes" | Haga un recuento para cada ceremonia de claves. |
| "No puedes continuar con la ceremonia porque la sesión de recuento no está conectada o el inicio de la ceremonia no está permitido." | Espere a más fideicomisarios. Si el estado es `CONNECTED`, compruebe **Permitir Recuento** de cada elección y cierre el período de votación. |
| "El fideicomisario no forma parte de la ceremonia de claves o tiene un estado no válido" | El fideicomisario no está en la ceremonia de claves de estas elecciones, o ya ha subido el fragmento. |
| No se acepta el archivo de un fideicomisario. | Use la copia del otro dispositivo de almacenamiento. Asegúrese de que es el archivo de este proceso electoral. |
| El recuento no empieza, y el proceso electoral usa votación ponderada y tiene actas de escrutinio aprobadas. | La votación ponderada y las actas de escrutinio no se pueden usar juntas. Contacte con el soporte de Sequent. |
| Una elección muestra `ERROR`. | No inicie un recuento nuevo. Contacte con el soporte de Sequent y dé el nombre del proceso electoral. |

## Más información {#more-information}

- [La ceremonia de recuento en detalle](../01-tutorials/16-admin_portal_tutorials_tally-ceremony.md)
- [Proceso electoral: Recuento](../02-reference/02-election-event/08-01-election_management_election-event_tally.md)
- [Actas de escrutinio](../02-reference/04-contest/02-election_management_contest_tally-sheets.md)
- [Importación de actas de escrutinio](../02-reference/02-election-event/08-03-election_management_election-event_tally-sheet-imports.md)
- [Urnas](../01-tutorials/21-admin_portal_tutorials_ballot-boxes.md)

**Siguiente:** [Obtener los resultados](07-results.md).
