---
title: Realizar la ceremonia de recuento
sidebar_position: 6
description: "La ceremonia de recuento descifra y cuenta los votos. El administrador electoral crea el recuento. Los fideicomisarios cargan sus fragmentos de clave."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Realizar la ceremonia de recuento

La ceremonia de recuento descifra y cuenta los votos. El administrador electoral crea el recuento.
Los fideicomisarios cargan sus fragmentos de clave. Cuando suficientes fideicomisarios han cargado
su fragmento, el administrador electoral inicia el recuento.

## Antes de empezar {#before-you-start}

- El estado de la ceremonia de claves es `SUCCESS`. Consulte [Realizar la ceremonia de claves](04-keys.md).
- El proceso electoral tiene una publicación. Consulte [Publicar y gestionar el período de votación](05-publish.md).
- El período de votación está cerrado, salvo si hace un informe de inicialización.
- **Permitir Recuento** de cada elección permite el recuento. Consulte [Crear el proceso electoral](02-event.md#3-create-the-elections).
- Está disponible un número de fideicomisarios igual o mayor que el umbral, con una copia de su
  fragmento de clave.
- Usted ha acordado una fecha y una hora con los fideicomisarios.

:::danger ADVERTENCIA
Inicie el recuento de los resultados de la elección solo después del final oficial del período de
votación. Si **Permitir Recuento** es **Permitido**, el portal de administración le permite
iniciar el recuento mientras los votantes todavía pueden votar.
:::

## 1. Crear el recuento (administrador electoral) {#1-create-the-tally-election-administrator}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Recuento**.
3. Haga clic en **Iniciar la Ceremonia de Recuento**.
4. En **Elecciones para el Recuento**, seleccione las elecciones que quiere contar.
5. En **Ceremonia de Llaves**, seleccione la ceremonia de claves de estas elecciones. Si solo hay
   una ceremonia de claves, el portal de administración la selecciona.
6. Haga clic en **Iniciar Ceremonia de Recuento**.
7. Lea el mensaje "Esta acción notificará a los trustees para importar sus fragmentos de clave."
8. Haga clic en **Aceptar**.

**Resultado esperado:** aparece el mensaje "Recuento creado". El recuento aparece en la lista de
la pestaña **Recuento** con el estado `STARTED`.

9. Avise a los fideicomisarios de que la ceremonia de recuento ha empezado.

## 2. Cargar el fragmento de clave (cada fideicomisario) {#2-upload-the-key-fragment-each-trustee}

Cada fideicomisario hace estos pasos con su propia cuenta. Necesita una copia de su fragmento de
clave de su dispositivo de almacenamiento.

1. Inicie sesión en el portal de administración.
2. Abra el proceso electoral.
3. Haga clic en la pestaña **Recuento**. Aparece el mensaje "Ha sido invitado a participar a una
   Ceremonia de Recuento."
4. En la fila del recuento, haga clic en **Agregar Clave de Cómputo** en la columna **Acciones**.
5. Lea la lista de **Elecciones para el Recuento**. Asegúrese de que es la lista que espera.
6. Suelte el archivo de su fragmento de clave en el área de carga.
7. Asegúrese de que aparece el mensaje "Copia de seguridad verificada correctamente."
8. Haga clic en **Siguiente**.
9. Retire el dispositivo de almacenamiento y guárdelo en su lugar seguro.

**Resultado esperado:** el fideicomisario ha cargado el fragmento de clave.

## 3. Iniciar el recuento (administrador electoral) {#3-start-the-tally-election-administrator}

1. En la pestaña **Recuento**, haga clic en el icono de ver del recuento.
2. Mire la sección **Trustee**. Muestra cuántos fideicomisarios han importado su clave y cuántos
   son necesarios.
3. Espere hasta que el estado sea `CONNECTED`. Este estado indica que suficientes fideicomisarios
   han cargado su fragmento de clave.
4. Haga clic en **Iniciar Recuento**.
5. Lea el mensaje "¿Desea iniciar el Recuento?".
6. Haga clic en **Iniciar recuento**.

**Resultado esperado:** aparece el mensaje "Recuento iniciado". El estado cambia a `IN_PROGRESS`.
La tabla **Progreso del Recuento de Elecciones** muestra el progreso de cada elección.

7. Espere hasta que el estado sea `SUCCESS`. El tiempo depende del número de votos.
8. Haga clic en **Resultados**. Consulte [Obtener los resultados](07-results.md).
9. Registre la fecha, la hora, los fideicomisarios presentes y el resultado de la ceremonia.

## Estado del recuento {#tally-status}

| Estado | Significado |
| --- | --- |
| `STARTED` | El recuento está creado. Los fideicomisarios pueden cargar sus fragmentos de clave. |
| `CONNECTED` | Suficientes fideicomisarios han cargado su fragmento de clave. Puede iniciar el recuento. |
| `IN_PROGRESS` | La plataforma mezcla, descifra y cuenta los votos. |
| `SUCCESS` | El recuento está completo. Los resultados están disponibles. |
| `CANCELLED` | El recuento está cancelado. |

La tabla **Progreso del Recuento de Elecciones** muestra estos pasos para cada elección:
`WAITING`, `MIXING`, `DECRYPTING`, `SUCCESS` o `ERROR`.

## Cancelar un recuento {#cancel-a-tally}

Puede cancelar un recuento antes de que se inicie, cuando su estado es `STARTED` o `CONNECTED`.

:::caution PRECAUCIÓN
No puede deshacer una cancelación. Los fideicomisarios deben cargar de nuevo su fragmento de clave
para un nuevo recuento.
:::

1. En la pestaña **Recuento**, haga clic en **Cancelar la Ceremonia de Cómputo** en la fila del
   recuento.
2. Haga clic en **Cancelar recuento**.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "La Ceremonia de Recuento no puede comenzar hasta que la Ceremonia de Claves se haya completado con éxito." | Complete la ceremonia de claves. Consulte [Realizar la ceremonia de claves](04-keys.md). |
| "La Ceremonia de Escrutinio no puede comenzar hasta que crees una publicación en la pestaña Publicar." | Publique la papeleta. Consulte [Publicar y gestionar el período de votación](05-publish.md). |
| "No puedes continuar con la ceremonia porque no se han seleccionado elecciones o las elecciones no están publicadas." | Seleccione una o más elecciones publicadas. |
| "Elections have different keys ceremonies" (las elecciones tienen ceremonias de claves diferentes) | Cree un recuento para cada ceremonia de claves. |
| "No puedes continuar con la ceremonia porque la sesión de recuento no está conectada o el inicio de la ceremonia no está permitido." | Espere a más fideicomisarios. Si el estado es `CONNECTED`, compruebe **Permitir Recuento** de cada elección y cierre el período de votación. |
| "Trustee not part of the keys ceremony or has invalid state" (el fideicomisario no forma parte de la ceremonia de claves o tiene un estado no válido) | El fideicomisario no está en la ceremonia de claves de estas elecciones, o ya ha cargado el fragmento. |
| No se acepta el archivo de un fideicomisario. | Use la copia del otro dispositivo de almacenamiento. Asegúrese de que es el archivo de este proceso electoral. |
| Una elección muestra `ERROR`. | No inicie un nuevo recuento. Contacte con el soporte de Sequent y dé el nombre del proceso electoral. |

## Más información {#more-information}

- [Proceso electoral: Recuento](../Reference/User-Manual/Election-Management/Election-Event/Tally/election_management_election-event_tally.md)
- [Pregunta: Hojas de recuento](../Reference/User-Manual/Election-Management/Contest/Tally-Sheets/election_management_contest_tally-sheets.md)
- [Elección: Datos](../Reference/User-Manual/Election-Management/Election/Data/election_management_election_data.md)
- [Proceso electoral: Logs](../Reference/User-Manual/Election-Management/Election-Event/Logs/election_management_election-event_logs.md)

**Siguiente:** [Obtener los resultados](07-results.md).
