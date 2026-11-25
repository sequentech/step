---
id: election_management_election_event_transmission
title: Transmisión
description: "Un paquete de transmisión contiene los resultados de una elección en un área. Usted crea el paquete a partir de un recuento completado, los firmantes lo firman y usted lo envía a los servidores de destino."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Un paquete de transmisión contiene los resultados de una elección en un área. Usted crea el paquete
a partir de un recuento completado. Los firmantes del área pueden firmar el paquete con su archivo
de firma. Después, envíe el paquete a los servidores de destino o descárguelo.

La función de transmisión solo está disponible si Sequent la ha activado para su plataforma.
Sequent también configura los servidores de destino y los firmantes de cada área.

## Antes de empezar {#before-you-start}

- El recuento tiene el tipo **Resultados Electorales** y el estado `SUCCESS`. Consulte
  [Realizar la ceremonia de recuento](../../03-procedures/06-tally.md).
- Los servidores de destino y los firmantes de cada área están configurados.
- Su rol tiene los permisos para las acciones que realiza:

| Permiso | Acción |
| --- | --- |
| **Administrar Ceremonia de Claves**, **Miru Create**, **Miru Sign**, **Miru Send** o **Miru Download** | Consulte el icono de visualización del recuento. |
| **Exportar Ceremonia** | Use el menú **Acciones** de una elección para abrir o crear un paquete. |
| **Miru Create** | Cree un paquete y use **Regenerar**. |
| **Miru Sign** | Firme un paquete. |
| **Miru Send** | Use **Enviar**. |
| **Miru Download** | Use **Descargar**. |
| **Generar Informe de Transmisión** | Use **Descargar informe de transmisión**. |

## Abrir un paquete de transmisión {#open-a-transmission-package}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Recuento**.
3. En la fila del recuento, haga clic en el icono de ver. Su texto emergente es **Ver la Ceremonia de Cómputo**. Si su rol tiene el permiso **Ceremonia de Fideicomisario**, el icono es una antena y su texto emergente es **Gestionar la Ceremonia de Cómputo**.
4. Abra **Resultados & Participación**.
5. En **Elecciones**, seleccione la elección.
6. Haga clic en **Acciones** de la elección.
7. Haga clic en **Enviar paquete de transmisión para el área '&lt;area&gt;'**. El menú tiene un elemento por cada área que tenga resultados en la elección.

**Resultado esperado:** si el paquete existe, se abre la página del paquete. Si no existe, la plataforma lo crea con la tarea **Crear paquete de transmisión**. La página del paquete se abre cuando el paquete está listo.

## Página del paquete {#package-page}

El título es "Paquete de Transmisión para el Área '&lt;area&gt;' y Elección '&lt;election&gt;'". La página tiene estos botones. Un botón solo se muestra si su rol tiene el permiso correspondiente.

| Botón | Uso |
| --- | --- |
| **Enviar** | Envía el paquete a los servidores de destino. |
| **Descargar** | Descarga los archivos del paquete o el informe de transmisión. |
| **Regenerar** | Crea una nueva versión del paquete. |
| **Atrás** | Vuelve al recuento. |

La página tiene estas secciones:

| Sección | Contenido |
| --- | --- |
| **Firmar paquete de transmisión** | El área de carga para su archivo de firma. Solo se muestra si su usuario es firmante del área. |
| **Firmas SBEI** | Los firmantes del área, con las columnas **Nombre del Fideicomisario** y **Ha Firmado**. Una marca de verificación indica una firma. Un reloj de arena indica que el firmante no ha firmado. |
| **Servidores de Destino** | Los servidores de destino, con las columnas **Nombre del Servidor** y **Estado de Envío**. Una marca de verificación indica que el servidor recibió el paquete. Un reloj de arena indica que no lo recibió. |
| **Logs** | Los eventos del paquete, con **Fecha** y **Entrada**. |

El título de **Firmas SBEI** muestra el estado "&lt;n&gt; de &lt;total&gt; Han Firmado, &lt;minimum&gt; mínimo".
El título de **Servidores de Destino** muestra el estado "Enviado a &lt;n&gt; de &lt;total&gt;".

## Firmar el paquete (firmante) {#sign-the-package-signer}

Realice este procedimiento solo si se muestra la sección **Firmar paquete de transmisión**. La
sección indica que la firma es opcional. Pero **Enviar** necesita el número mínimo de firmas que
indica el estado.

Tenga a mano su archivo de firma y su contraseña.

:::note
Use siempre el mismo archivo de firma. Después de su primera firma, la plataforma solo acepta ese
archivo para su usuario. La plataforma rechaza un archivo que firmó un paquete de otra elección.
:::

1. Abra el paquete. Consulte [Abrir un paquete de transmisión](#open-a-transmission-package).
2. En **Firmar paquete de transmisión**, arrastre su archivo de firma al área de carga. O haga clic en **Examinar** y seleccione el archivo.
3. En el cuadro "¿Desea firmar el paquete de transmisión?", escriba la contraseña del archivo en **Ingrese su contraseña**.
4. Haga clic en **Firmar paquete de transmisión**.

**Resultado esperado:** aparece el mensaje "Signing Successful". En **Firmas SBEI**, su fila muestra una marca de verificación.

## Enviar el paquete {#send-the-package}

:::caution PRECAUCIÓN
Compruebe los resultados antes de enviar el paquete. No puede cancelar una transmisión.
:::

1. Asegúrese de que el número de firmas sea igual o superior al mínimo.
2. Haga clic en **Enviar**.
3. Lea el mensaje "¿Quieres enviar el Paquete de Transmisión?".
4. Haga clic en **Enviar Paquete de Transmisión**.

**Resultado esperado:** aparece el mensaje "Enviando Paquete de Transmisión...". En **Servidores de Destino**, cada servidor que recibió el paquete muestra una marca de verificación.

**Enviar** no está disponible cuando el paquete tiene menos firmas que el mínimo, o cuando todos los servidores de destino ya han recibido el paquete. En ese caso, la información emergente muestra "No se ha alcanzado el número mínimo de firmas de miembros SBEI o el Paquete de Transmisión ya se ha enviado a todos los servidores".

## Descargar el paquete o el informe {#download-the-package-or-the-report}

1. Haga clic en **Descargar**.
2. Haga clic en un elemento del menú:

| Elemento | Contenido |
| --- | --- |
| **Download EML** | El archivo de resultados del paquete. El elemento solo se muestra cuando el paquete tiene este archivo. |
| **Descargar paquete de transmisión &lt;fecha&gt;** | Un archivo ZIP con el paquete para todos los servidores de destino. |
| **Descargar informe de transmisión** | Un informe PDF de la transmisión. La plataforma lo crea con la tarea **Generar Informe de Transmisión**. |

La fecha es la fecha y hora de la versión más reciente del paquete.

3. En los dos primeros elementos, lea el mensaje "¿Desea descargar el paquete de transmisión?".
4. Haga clic en **Descargar Paquete de Transmisión**.

**Resultado esperado:** el navegador guarda el archivo.

## Regenerar el paquete {#regenerate-the-package}

:::caution PRECAUCIÓN
Regenere el paquete solo si es necesario. La nueva versión no tiene firmas y no se envía. Los firmantes deben firmarla de nuevo, y usted debe enviarla de nuevo.
:::

1. Haga clic en **Regenerar**.
2. Lea el mensaje "¿Desea regenerar el paquete de transmisión?".
3. Haga clic en **Regenerar paquete de transmisión**.

**Resultado esperado:** la plataforma crea una nueva versión del paquete. **Firmas SBEI** y **Servidores de Destino** muestran el estado de la nueva versión.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| El menú **Acciones** no tiene el elemento **Enviar paquete de transmisión para el área**. | Compruebe que el recuento tiene el tipo **Resultados Electorales** y que el área tiene resultados. Si el elemento sigue sin aparecer, contacte con el soporte de Sequent. |
| "Error al crear el Paquete de Transmisión" | Vuelva a intentarlo. Si el error continúa, contacte con el soporte de Sequent. |
| "Error al enviar el Paquete de Transmisión" | Lea la sección **Logs**. Después, contacte con el soporte de Sequent. |
| La sección **Firmar paquete de transmisión** no se muestra. | Su usuario no es firmante del área. Contacte con el soporte de Sequent. |
| "Copa de Seguridad de la Clave Encriptada Privada inválida, por favor inténtalo de nuevo" después de añadir el archivo de firma. | El archivo no se ha subido. Añada el archivo de nuevo. |
| "Hubo un error al subir la firma" | Compruebe que el archivo y la contraseña son correctos. Use el archivo de su primera firma. |
| **Enviar** no está disponible. | Espere hasta que firmen suficientes firmantes. Si todos los servidores muestran una marca de verificación, el paquete ya se ha enviado. |
| Un servidor muestra un reloj de arena después de enviar. | La plataforma no vuelve a enviar a un servidor después de un intento fallido. Lea la sección **Logs**. Después, contacte con el soporte de Sequent. |

## Páginas relacionadas {#related-pages}

- [Realizar la ceremonia de recuento](../../03-procedures/06-tally.md)
- [Obtener los resultados](../../03-procedures/07-results.md)
- [Proceso electoral: Recuento](08-01-election_management_election-event_tally.md)
- [Proceso electoral: Tareas](10-election_management_election-event_tasks.md)
- [Roles](../user-manual/users-and-roles/users-and-roles_roles.md)
