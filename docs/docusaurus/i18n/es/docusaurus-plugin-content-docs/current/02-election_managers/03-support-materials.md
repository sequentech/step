---
sidebar_position: 6
id: support_materials
title: Materiales de Soporte
description: "Los Materiales de Soporte son documentos del proceso electoral (guías, papeletas de muestra, vídeos explicativos) que los votantes pueden revisar antes de votar. La Política de Materiales de Soporte controla si se muestran y si leerlos es un requisito para votar."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Materiales de Soporte

Los Materiales de Soporte son documentos del proceso electoral (guías, papeletas de muestra, vídeos
explicativos) que los votantes pueden revisar antes de votar. La Política de Materiales de Soporte
controla si se muestran y si leerlos es un requisito para votar.

## Política {#policy}

Configure la política en el acordeón **Materiales de Soporte** de la pestaña **Datos** del proceso
electoral:

- **Desactivado** oculta los Materiales de Soporte por completo en el portal de votación y no bloquea la votación.
- **Opcional** muestra los Materiales de Soporte como un enlace genérico desde la lista de votaciones;
  la votación nunca se bloquea.
- **Obligatorio para Votar** exige que cada votante abra cada Material de Soporte y confirme que lo ha
  leído, antes de que se active el botón de iniciar la votación de cualquier papeleta.

La política se aplica a todo el proceso electoral; no hay excepciones por elección. Los procesos
electorales creados antes de que existiera esta política solo tienen el booleano heredado
`presentation.materials.activated`: **Opcional** si es `true`, y **Desactivado** si es `false` o no existe, hasta
que un administrador establezca la política de forma explícita.

## Experiencia del votante con Mandatory for Voting {#voter-experience-under-mandatory-for-voting}

Cuando la política es **Obligatorio para Votar** y un votante todavía no ha confirmado los materiales de
este proceso electoral, la lista de votaciones muestra un aviso ("Debe leer los Materiales de Soporte
antes de poder votar.") y desactiva el inicio de la votación en todas las papeletas. El botón de
Materiales de Soporte sigue disponible para que el votante los abra.

En la pantalla de Materiales de Soporte, cada material debe abrirse al menos una vez con **Vista
previa**. Un material se marca como visto en cuanto se cierra su diálogo de vista previa. La casilla
"He leído los Materiales de Soporte" permanece desactivada hasta que se hayan visto todos los
materiales, y **Continuar** permanece desactivado hasta que se marque la casilla.

Al elegir **Continuar** se registra la confirmación y el votante vuelve a la lista de votaciones con
el inicio de la votación activado. Al elegir **Volver a la Lista de Votaciones**, la lista sigue
bloqueada, porque no se registra nada hasta que **Continuar** tiene éxito.

Una vez confirmada, el bloqueo se omite en las siguientes visitas a ese proceso electoral. El botón
de Materiales de Soporte sigue disponible para volver a abrir los documentos, pero iniciar la votación
ya no requiere la confirmación.

## Almacenamiento de la confirmación {#acknowledgment-storage}

La confirmación se registra como un atributo de usuario de Keycloak (`support-materials-acknowledged`)
en la cuenta del votante. El atributo contiene la lista de ids de los documentos de Materiales de
Soporte que el votante confirmó haber leído. Como cada proceso electoral tiene su propio realm de
Keycloak, el atributo está limitado por naturaleza a cada votante y a cada proceso electoral. Es el
mismo patrón que se usa para `voted-channel`. Volver a abrir los materiales después de confirmarlos
no borra el atributo. Actualmente no hay ninguna acción masiva de reinicio para los administradores.

La lista de votantes del proceso electoral muestra una columna "Support Materials Viewed" con un valor
booleano simple leído de ese mismo atributo, cuando la política es **Obligatorio para Votar**.

## Solicitudes y permisos {#requests-and-permissions}

El portal de votación lee y escribe la confirmación mediante dos Hasura Actions respaldadas por
Harvest. Ambas se autentican con el token bearer del propio votante (`role: user`,
`forward_client_headers: true`):

- `get_support_materials_acknowledgment(election_event_id)` devuelve los ids de documento que el
  votante que hace la llamada ya ha confirmado.
- `acknowledge_support_materials(election_event_id, document_ids)` registra la confirmación del
  votante que hace la llamada, después de comprobar que la política del proceso electoral no es **Desactivado**.

Ambas acciones obtienen la identidad del votante a partir de sus propias claims del JWT. Un votante
solo puede leer o escribir su propia confirmación.

Además, ambas acciones exigen que la sesión de Keycloak del votante tenga el rol de realm
`ack-support-materials`. Esta comprobación se hace igual que la emisión de papeletas comprueba el rol
`user` del votante. Este rol debe existir en el realm propio del proceso electoral
(`tenant-<tenant_id>-event-<election_event_id>`) y estar asignado al grupo `voter`. Sin él, todos los
votantes son rechazados, sin importar la Política de Materiales de Soporte.

### Añadir el rol en la consola de Keycloak {#adding-the-role-in-the-keycloak-console}

1. Abra la consola de administración de Keycloak y cambie al realm del proceso electoral
   (`tenant-<tenant_id>-event-<election_event_id>`).
2. En **Realm roles**, haga clic en **Create role**, escriba `ack-support-materials` en
   **Role name** y guarde.
3. En **Groups**, abra **voter**, vaya a la pestaña **Role mapping**, haga clic en
   **Assign role**, filtre por roles de realm, seleccione `ack-support-materials` y haga clic en
   **Assign**.
4. Los votantes que ya tienen sesión iniciada conservan el conjunto de roles de su token actual. Deben
   cerrar sesión y volver a entrar (o esperar a que el token se renueve) para que el nuevo rol surta
   efecto.
