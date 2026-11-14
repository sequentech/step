---
id: admin_portal_tutorials_tally-ceremony
title: Ceremonia de recuento
description: "La ruta principal está en Realizar la ceremonia de recuento. Esta página da los detalles: los estados, las políticas que bloquean o permiten el recuento, el recuento automático, la resolución de empates, la repetición del recuento y la…"
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La ruta principal está en [Realizar la ceremonia de recuento](../03-procedures/06-tally.md). Esta página da los detalles: los estados, las políticas que bloquean o permiten el recuento, el recuento automático, la resolución de empates, la repetición del recuento y la cancelación.

## Tipos de recuento {#types-of-tally}

Un recuento de la pestaña **Recuento** tiene uno de dos tipos, en la columna **Tipo de Conteo**:

| **Tipo de Conteo** | Se crea con | Contenido |
| --- | --- | --- |
| **Resultados Electorales** | **Iniciar la Ceremonia de Recuento** | El conteo de los votos después del período de votación. |
| **Resultados de Inicialización** | **Generar Informe de Inicialización** | El conteo de la urna vacía antes del período de votación. Consulte [Informe de inicialización](11-admin_portal_tutorials_initialization-report.md). |

La lista de **Recuento** también tiene las columnas **Etiquetas de Permisos**, **Autoridades**, **Número de Elecciones** y **Estado**.

## Los pasos del asistente {#the-steps-of-the-wizard}

La página de recuento muestra sus pasos en la parte superior: **Inicio**, **Ceremonia**, **Recuento** y **Resultados**. En una ceremonia de claves automática, no se muestra el paso **Ceremonia**.

| Paso | Qué hace usted |
| --- | --- |
| **Inicio** | Seleccione las elecciones y la ceremonia de claves. Después, cree el recuento. |
| **Ceremonia** | Observe cómo los fideicomisarios suben su fragmento de clave. Después, inicie el recuento. |
| **Recuento** | Observe el progreso de cada elección. Resuelva los empates, si es necesario. |
| **Resultados** | Consulte los resultados, descargue los documentos y publíquelos. Consulte [Resultados electorales](17-admin_portal_tutorials_election-results.md). |

Cuando abre un recuento desde la lista, la página va al paso que corresponde a su estado.

## Estado del recuento {#tally-status}

| Estado | Significado | Estado siguiente |
| --- | --- | --- |
| `STARTED` | El recuento está creado. Los fideicomisarios pueden subir su fragmento de clave. | `CONNECTED` cuando suben suficientes fideicomisarios, o `CANCELLED`. |
| `CONNECTED` | El número de fideicomisarios que subieron su fragmento es igual o mayor que el umbral. | `IN_PROGRESS` cuando hace clic en **Iniciar Recuento**, o `CANCELLED`. |
| `IN_PROGRESS` | La plataforma mezcla, descifra y cuenta los votos. | `SUCCESS`, `AWAITING_INPUT` o `CANCELLED`. |
| `AWAITING_INPUT` | El conteo se detuvo en un empate que necesita una decisión. | `IN_PROGRESS` después de la resolución, o `CANCELLED`. |
| `SUCCESS` | El recuento está completo. | `IN_PROGRESS` para un nuevo recuento. |
| `CANCELLED` | El recuento está cancelado. | Ninguno. |

La tabla **Progreso del Recuento de Elecciones** muestra un estado para cada elección:

| Estado de la elección | Significado |
| --- | --- |
| `WAITING` | La elección espera su turno. |
| `MIXING` | La plataforma mezcla los votos cifrados, para que ningún voto pueda vincularse con un votante. |
| `DECRYPTING` | La plataforma descifra los votos con los fragmentos de clave. |
| `SUCCESS` | El conteo de la elección está completo. |
| `ERROR` | El conteo de la elección falló. Contacte con el soporte de Sequent. |

## El umbral y los fideicomisarios {#the-threshold-and-the-trustees}

El umbral es el número mínimo de fideicomisarios que estableció la ceremonia de claves. La sección **Autoridades** del paso **Ceremonia** muestra el número de fideicomisarios que importaron su clave y el número necesario. El recuento pasa a `CONNECTED` cuando el primer número alcanza el segundo.

No necesita a todos los fideicomisarios. Necesita un número de fideicomisarios igual o mayor que el umbral.

### Lo que ven los fideicomisarios {#what-the-trustees-see}

- En la pestaña **Recuento**, un fideicomisario ve el mensaje "Ha sido invitado a participar en una ceremonia de recuento." mientras el recuento espera los fragmentos de clave.
- El icono de llave **Agregar Clave de Cómputo** solo se muestra mientras el fideicomisario no ha subido un fragmento y el recuento está en `STARTED` o `CONNECTED`. En los demás casos, el fideicomisario ve **Ver la Ceremonia de Cómputo**.
- El fideicomisario no inicia el recuento. El administrador electoral hace clic en **Iniciar Recuento**.

## Políticas que bloquean o permiten el recuento {#policies-that-block-or-permit-the-tally}

Cada elección tiene el campo **Permitir Recuento** en su pestaña **Datos**.

| **Permitir Recuento** | Efecto |
| --- | --- |
| **Permitido** (por defecto) | Puede crear e iniciar el recuento en cualquier momento, también mientras los votantes votan. |
| **No permitido** | No puede hacer el recuento de la elección. |
| **Permitir cuando termine el periodo de votación** | Puede hacer el recuento de la elección solo cuando la votación en línea está `CLOSED` y cada otro canal permitido está `CLOSED` o `NOT_STARTED`. |

El evento programado **Permitir Recuento** cambia el campo a **Permitido** a una hora determinada. Consulte [Eventos programados](13-admin_portal_tutorials_scheduled-events.md).

:::danger ADVERTENCIA
Use **Permitir cuando termine el periodo de votación** en cada elección. Con **Permitido**, una persona puede iniciar el conteo mientras los votantes siguen votando.
:::

La plataforma comprueba estas reglas dos veces: cuando crea el recuento y cuando hace clic en **Iniciar Recuento**. Cada elección seleccionada también debe estar publicada.

Otras reglas:

- Todas las elecciones seleccionadas deben usar la misma ceremonia de claves. Haga un recuento para cada ceremonia de claves.
- Solo ve y cuenta las elecciones con una etiqueta de permiso que tiene su cuenta.
- El voto delegado y el voto ponderado por votante no pueden estar activados a la vez en el mismo proceso electoral. El voto ponderado por votante tiene otros límites en el método de conteo y en las actas de escrutinio. La plataforma muestra la causa cuando rechaza el recuento.

## Recuento automático {#automatic-tally}

Si la ceremonia de claves es automática, el recuento no necesita fideicomisarios:

- El botón del paso **Inicio** es **Iniciar Recuento**.
- El mensaje es "Selecciona 'Start Tally' para ejecutar el proceso de recuento y mostrar los resultados, o 'Close' para cancelar."
- El recuento empieza con el estado `IN_PROGRESS`. No se muestra el paso **Ceremonia**.

Una ceremonia de claves automática solo es posible cuando la **Política de ceremonias de llaves/recuento** del proceso electoral permite ceremonias automáticas.

## Resolver un empate {#resolve-a-tie}

Una pregunta puede detenerse en un empate, por ejemplo una pregunta de **Segunda Vuelta Instantánea** con la **Política de desempate** **Procedimiento externo**. El estado del recuento cambia entonces a `AWAITING_INPUT`. La página del recuento muestra el panel **Resoluciones pendientes** y el mensaje "Recuento pausado por empate sin resolver (Ronda ...)".

Su cuenta necesita el permiso **Enviar resolución del escrutinio**.

1. En la lista **Resoluciones pendientes**, haga clic en un empate. Use **Filtrar** para encontrarlo por **Elección**, **Pregunta**, **Área** o **Estado**.
2. Lea los candidatos empatados y sus votos.
3. En **Seleccione el candidato a avanzar**, seleccione el candidato que da su decisión.
4. Haga clic en **Guardar**. El empate muestra el estado **Cálculo pendiente**.
5. Repita los pasos 1 a 4 para cada empate.
6. Haga clic en **Aplicar resoluciones y recalcular**.

**Resultado esperado:** aparece el mensaje "Resoluciones enviadas. El recuento está reanudando...". El estado cambia a `IN_PROGRESS`. Los empates resueltos muestran "Recuento reanudado tras aplicar la resolución", con la fecha y el usuario.

Para cambiar una decisión antes de aplicarla, haga clic en **Deshacer resolución**. La plataforma registra cada pausa, cada resolución y cada reinicio en el registro electoral.

## Repetir un recuento {#recount-a-tally}

Una repetición cuenta de nuevo un recuento completo y crea un nuevo conjunto de resultados. Úselo, por ejemplo, después de la aprobación de nuevas actas de escrutinio.

- La acción **Repetir el recuento** solo se muestra para un recuento con el estado `SUCCESS`.
- Su cuenta necesita el permiso **Ejecutar recuento de resultados**.
- El estado cambia a `IN_PROGRESS`, el progreso de cada elección vuelve a empezar en `WAITING` y, después, el estado cambia a `SUCCESS`.

## Cancelar un recuento {#cancel-a-tally}

**Cancelar la Ceremonia de Cómputo** solo aparece en la lista cuando el estado es `STARTED` o `CONNECTED`. El cuadro de diálogo dice "Estás a punto de cancelar la ceremonia de recuento. Esta acción no se puede deshacer." Haga clic en **Cancelar recuento** para confirmar.

:::caution PRECAUCIÓN
No puede deshacer una cancelación. En un recuento nuevo, los fideicomisarios deben subir de nuevo su fragmento de clave.
:::

## Permisos {#permissions}

| Permiso | Le permite |
| --- | --- |
| **Ver Escrutinio del Evento Electoral** y **Leer Recuento** | Ver la pestaña **Recuento**. |
| **Crear Ceremonia** | Ver **Iniciar la Ceremonia de Recuento** y **Generar Informe de Inicialización**. |
| **Administrar Ceremonia de Claves** | Crear, iniciar, ver y cancelar recuentos. |
| **Ceremonia de Fideicomisario** | Subir un fragmento de clave como fideicomisario. |
| **Enviar resolución del escrutinio** | Resolver empates. |
| **Ejecutar recuento de resultados** | Repetir un recuento completo. |
| **Exportar Ceremonia** | Descargar los documentos de resultados. |

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| **Iniciar la Ceremonia de Recuento** no está disponible. | Complete la ceremonia de claves y haga una publicación. Espere hasta que la página no muestre ninguna advertencia. |
| "Selecciona al menos una elección." | Seleccione una o más elecciones. |
| "Publica cada elección seleccionada antes de crear su recuento." | Publique la papeleta. Consulte [Publicar y gestionar el período de votación](../03-procedures/05-publish.md). |
| "El recuento está deshabilitado para una elección seleccionada." | **Permitir Recuento** de la elección es **No permitido**. Cámbielo, o espere al evento programado **Permitir Recuento**. |
| "Finaliza la votación en cada elección seleccionada y detén sus canales activos antes de crear el recuento." | Cierre la votación en línea y cada otro canal abierto o en pausa de la elección. |
| "Elections have different keys ceremonies" | Haga un recuento para cada ceremonia de claves. |
| "Some elections don't have the required permission label or are not published" | Publique las elecciones. Pida la etiqueta de permiso de las elecciones. |
| "No puedes continuar con la ceremonia porque la sesión de recuento no está conectada o el inicio de la ceremonia no está permitido." | Espere a más fideicomisarios. Si el estado es `CONNECTED`, compruebe **Permitir Recuento** y los canales de votación. |
| "Insufficient number of connected trustees ..." | Espere hasta que suficientes fideicomisarios suban su fragmento. |
| "Trustee not part of the keys ceremony or has invalid state" | El fideicomisario no está en la ceremonia de claves de estas elecciones, o ya subió el fragmento. |
| "Copa de Seguridad de la Clave Encriptada Privada inválida, por favor inténtalo de nuevo" | El fideicomisario subió un archivo incorrecto. Use el archivo del fragmento de clave de este proceso electoral. |
| "Tu clave ya se había restaurado." | El fideicomisario ya subió el fragmento. No es necesaria ninguna acción. |
| "Error al enviar las resoluciones. Por favor, inténtelo de nuevo." | Vuelva a intentarlo. Asegúrese de que su cuenta tiene **Enviar resolución del escrutinio**. |
| "No se ha podido iniciar el recuento" | Asegúrese de que el estado es `SUCCESS` y vuelva a intentarlo. |
| Una elección muestra `ERROR`. | No inicie un recuento nuevo. Contacte con el soporte de Sequent y dé el nombre del proceso electoral. |
