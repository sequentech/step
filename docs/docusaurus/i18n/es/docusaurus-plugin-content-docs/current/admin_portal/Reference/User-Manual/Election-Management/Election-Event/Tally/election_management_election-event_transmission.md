---
id: election_management_election_event_transmission
title: Transmisión
description: "Un paquete de transmisión contiene los resultados de una elección en un área. Usted lo crea a partir de un recuento completado, los firmantes lo firman y usted lo envía a los servidores de destino."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Un paquete de transmisión contiene los resultados de una elección en un área. Usted crea el paquete
a partir de un recuento completado. Los firmantes del área pueden firmar el paquete con su archivo
de firma. Después usted envía el paquete a los servidores de destino, o lo descarga.

La función de transmisión solo está disponible cuando Sequent la ha activado para su plataforma.
Sequent también configura los servidores de destino y los firmantes de cada área.

## Antes de empezar {#before-you-start}

- El recuento tiene el tipo **Resultados Electorales** y el estado `SUCCESS`. Consulte
  [Realizar la ceremonia de recuento](../../../../../procedures/06-tally.md).
- Los servidores de destino y los firmantes de cada área están configurados.
- Su rol tiene los permisos para las acciones que hace:

| Permiso | Acción |
| --- | --- |
| **Administrar Ceremonia de Claves**, **Miru Create**, **Miru Sign**, **Miru Send** o **Miru Download** | Ver el icono de visualización del recuento. |
| **Exportar Ceremonia** | Usar el menú **Acciones** de una elección para abrir o crear un paquete. |
| **Miru Create** | Crear un paquete y usar **Regenerar**. |
| **Miru Sign** | Firmar un paquete. |
| **Miru Send** | Usar **Enviar**. |
| **Miru Download** | Usar **Descargar**. |
| **Generar Informe de Transmisión** | Usar **Descargar informe de transmisión**. |

## Abrir un paquete de transmisión {#open-a-transmission-package}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Recuento**.
3. En la fila del recuento, haga clic en el icono de visualización. Su texto de ayuda es **Ver la
   Ceremonia de Cómputo**. Si su rol tiene **Ceremonia de Fideicomisario**, el icono es una antena y
   su texto de ayuda es **Gestionar la Ceremonia de Cómputo**.
4. Abra **Resultados & Participación**.
5. En **Elecciones**, haga clic en la pestaña de la elección.
6. Haga clic en **Acciones**, a la derecha de las pestañas.
7. Haga clic en **Enviar paquete de transmisión para el área '&lt;área&gt;'**. El menú tiene un
   elemento para cada área que tiene resultados en la elección.

**Resultado esperado:** si el paquete existe, se abre la página del paquete. Si no, la plataforma
crea el paquete con la tarea **Crear paquete de transmisión**. La página del paquete se abre cuando
el paquete está listo.

## Página del paquete {#package-page}

El título es "Paquete de Transmisión para el Área '&lt;área&gt;' y Elección '&lt;elección&gt;'". La
página tiene estos botones. Un botón solo se muestra cuando su rol tiene el permiso del botón.

| Botón | Uso |
| --- | --- |
| **Enviar** | Envía el paquete a los servidores de destino. |
| **Descargar** | Descarga los archivos del paquete o el informe de transmisión. |
| **Regenerar** | Crea una versión nueva del paquete. |
| **Atrás** | Vuelve al recuento. |

La página tiene estas secciones:

| Sección | Contenido |
| --- | --- |
| **Subir** | El área de carga para su archivo de firma. Solo se muestra cuando su usuario es un firmante del área. |
| **Firmas SBEI** | Los firmantes del área, con las columnas **Nombre del Fideicomisario** y **Ha Firmado**. Una marca de verificación indica una firma. Un reloj de arena indica que el firmante no ha firmado. |
| **Servidores de Destino** | Los servidores de destino, con las columnas **Nombre del Servidor** y **Estado de Envío**. Una marca de verificación indica que el servidor recibió el paquete. Un reloj de arena indica que no lo recibió. |
| **Logs** | Los eventos del paquete, con **Fecha** y **Entrada**. |

El título de **Firmas SBEI** muestra el estado "&lt;n&gt; de &lt;total&gt; Han Firmado".
El título de **Servidores de Destino** muestra el estado "Enviado a &lt;n&gt; de &lt;total&gt;".

## Firmar el paquete (firmante) {#sign-the-package-signer}

Haga este procedimiento solo si se muestra la sección **Subir**. La sección dice que la firma es
opcional. Pero **Enviar** necesita el número mínimo de firmas.

Tenga a mano su archivo de firma y su contraseña.

:::note
Use el mismo archivo de firma cada vez. Después de su primera firma, la plataforma solo acepta ese
archivo para su usuario. La plataforma rechaza un archivo que firmó un paquete de otra elección.
:::

1. Abra el paquete. Consulte [Abrir un paquete de transmisión](#open-a-transmission-package).
2. En **Subir**, arrastre su archivo de firma al área de carga. O haga clic en **Cargar fichero** y
   seleccione el archivo.
3. En "¿Desea firmar el paquete de transmisión?", escriba la contraseña del archivo en **Ingrese su
   contraseña**.
4. Haga clic en **Firmar paquete de transmisión**.

**Resultado esperado:** aparece el mensaje "Signing Successful". En **Firmas SBEI**, su fila
muestra una marca de verificación.

## Enviar el paquete {#send-the-package}

:::caution PRECAUCIÓN
Compruebe los resultados antes de enviar el paquete. No puede cancelar una transmisión.
:::

1. Asegúrese de que el número de firmas es igual o mayor que el mínimo.
2. Haga clic en **Enviar**.
3. Lea "¿Quieres enviar el Paquete de Transmisión?".
4. Haga clic en **Enviar Paquete de Transmisión**.

**Resultado esperado:** aparece el mensaje "Paquete de Transmisión Enviado". En **Servidores de
Destino**, cada servidor que recibió el paquete muestra una marca de verificación.

**Enviar** no está disponible cuando el paquete tiene menos firmas que el mínimo, o cuando todos los
servidores de destino recibieron el paquete. El texto de ayuda muestra entonces "Have not reached
minimum number of SBEI Member signatures or Transmission Package has already been sent to all
servers" (no se ha alcanzado el número mínimo de firmas, o el paquete ya se envió a todos los
servidores).

## Descargar el paquete o el informe {#download-the-package-or-the-report}

1. Haga clic en **Descargar**.
2. Haga clic en un elemento del menú:

| Elemento | Contenido |
| --- | --- |
| **Download EML** | El archivo de resultados del paquete. El elemento solo se muestra cuando el paquete tiene este archivo. |
| **Descargar el Paquete de Transmisión &lt;fecha&gt;** | Un archivo ZIP con el paquete para todos los servidores de destino. |
| **Descargar informe de transmisión** | Un informe PDF de la transmisión. La plataforma lo crea con la tarea **Generar Informe de Transmisión**. |

La fecha es la fecha y la hora de la versión más reciente del paquete.

3. Para los dos primeros elementos, lea "¿Quieres descargar el Paquete de Transmisión?".
4. Haga clic en **Descargar Paquete de Transmisión**.

**Resultado esperado:** el navegador guarda el archivo.

## Regenerar el paquete {#regenerate-the-package}

:::caution PRECAUCIÓN
Regenere el paquete solo si es necesario. La versión nueva no tiene firmas y no está enviada. Los
firmantes deben firmarla otra vez, y usted debe enviarla otra vez.
:::

1. Haga clic en **Regenerar**.
2. Lea "¿Desea regenerar el paquete de transmisión?".
3. Haga clic en **Regenerar paquete de transmisión**.

**Resultado esperado:** la plataforma crea una versión nueva del paquete. **Firmas SBEI** y
**Servidores de Destino** muestran el estado de la versión nueva.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| El menú **Acciones** no tiene el elemento **Enviar paquete de transmisión para el área**. | Asegúrese de que el recuento tiene el tipo **Resultados Electorales** y de que el área tiene resultados. Si el elemento sigue sin aparecer, contacte con el soporte de Sequent. |
| "Error al crear el Paquete de Transmisión" | Inténtelo otra vez. Si el error continúa, contacte con el soporte de Sequent. |
| "Error al enviar el Paquete de Transmisión" | Lea la sección **Logs**. Después contacte con el soporte de Sequent. |
| La sección **Subir** no se muestra. | Su usuario no es un firmante del área. Contacte con el soporte de Sequent. |
| "Copa de Seguridad de la Clave Encriptada Privada inválida, por favor inténtalo de nuevo" después de añadir el archivo de firma. | El archivo no se subió. Añada el archivo otra vez. |
| "Hubo un error al subir la firma" | Asegúrese de que el archivo y la contraseña son correctos. Use el archivo de su primera firma. |
| **Enviar** no está disponible. | Espere hasta que suficientes firmantes hayan firmado. Si todos los servidores muestran una marca de verificación, el paquete ya está enviado. |
| Un servidor muestra un reloj de arena después del envío. | La plataforma no vuelve a enviar a un servidor después de un intento fallido. Lea la sección **Logs**. Después contacte con el soporte de Sequent. |

## Páginas relacionadas {#related-pages}

- [Realizar la ceremonia de recuento](../../../../../procedures/06-tally.md)
- [Obtener los resultados](../../../../../procedures/07-results.md)
- [Proceso electoral: Recuento](election_management_election-event_tally.md)
- [Proceso electoral: Tareas](../Tasks/election_management_election-event_tasks.md)
- [Roles](../../../Users-and-Roles/Roles/admin_portal_reference_user-manual_users-and-roles_roles.md)
