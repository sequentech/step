---
id: admin_portal_tutorials_voter-communication
title: Comunicación con los votantes
description: "Envíe mensajes a los votantes por correo electrónico o SMS desde el portal de administración, con plantillas y variables por votante."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Enviar una comunicación a los votantes {#send-a-voter-communication}

1. Abra un proceso electoral y seleccione la pestaña **Votantes**.
2. Seleccione los votantes previstos o elija un público, como todos los votantes, los votantes que han votado o los votantes que no han votado.
3. Abra **Enviar comunicación**, seleccione correo electrónico o SMS, y escriba el contenido localizado.
4. Envíe el mensaje de inmediato o programe la entrega.

Se necesita el permiso normal de comunicación. La entrega se realiza por votante, así que las variables de la plantilla se resuelven por separado para cada destinatario.

## Uso de campos secretos de votante {#using-secret-voter-fields}

Los campos secretos de votante pueden usarse en el contenido de correo electrónico y de SMS sin que sean visibles en la lista de votantes. Para un atributo llamado `customerReference`, use las mismas formas de Handlebars que para un atributo personalizado de votante normal:

```handlebars
Your reference is {{user.customerReference}}.
```

`user.<attribute>` contiene el primer valor. El conjunto completo de valores está disponible mediante:

```handlebars
{{lookup user.attributes "customerReference"}}
```

Cuando se envía la comunicación, el portal de administración declara cada atributo secreto configurado cuyo nombre aparece en el contenido de la plantilla. El proceso de trabajo valida esa declaración, descifra solo esos campos para el votante actual, genera el mensaje y descarta el contexto en texto sin cifrar. Los campos secretos que no se mencionan en el contenido no se descifran ni se añaden a las variables de la plantilla.

Enviar o programar contenido que declara un campo secreto requiere `voter-secret-attribute-read`, además del permiso normal de notificación. Si el campo ya no está configurado como secreto, su envoltorio no es válido o el operador no tiene permiso, el trabajo falla sin sustituir el valor almacenado.

Consulte [Permisos](../02-reference/user-manual/users-and-roles/users-and-roles_permissions.md#secret-voter-field-permissions)
y [Plantillas](../02-reference/user-manual/templates/admin_portal_reference_user-manual_templates.md#secret-voter-variables).
