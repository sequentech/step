---
id: election_management_election_event_templates
title: Plantillas
description: "La gestión de plantillas es esencial para generar informes de forma coherente. Cada tipo de informe se asocia con una plantilla para formar una “receta” que se usa al generar informes."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


La gestión de plantillas es esencial para generar informes de forma coherente. Cada tipo de informe se asocia con una plantilla para formar una “receta” que se usa al generar informes.

### Añadir una nueva plantilla {#adding-a-new-template}

1. **Seleccione Añadir** para crear una plantilla.
2. **Complete los campos**:
   - **Alias de la Plantilla (opcional)**: Nombre visible en el portal de administración.
   - **Nombre de la Plantilla**: Nombre interno de la plantilla en el portal de administración.
   - **Tipo de plantilla**: Categoría o área que usará esta plantilla. (Por ejemplo, Recibo de Voto, Informe Estadístico, etc.)
   - **Email / SMS / Documento**: Elija si la plantilla incluye un mensaje de correo electrónico o SMS, o si adjunta un documento. Seleccione el botón de opción correspondiente.
3. Haga clic en **Guardar** para guardar la plantilla.

Una vez configurada, la plantilla está disponible para los tipos de informe asociados y para otras áreas del sistema.

### Exportar e importar plantillas {#exporting-and-importing-templates}

Use **Exportar** e **Importar** para copiar plantillas, por ejemplo, para conservar una copia de seguridad.

1. Para exportar, haga clic en **Exportar**. El portal de administración crea un archivo CSV con todas las plantillas. Descargue
   el archivo cuando esté listo.
2. Para importar, haga clic en **Importar**. Escriba el hash del archivo en **Verificación de Integridad (SHA 256)**, cargue el
   archivo CSV y haga clic en **Importar**.

Importe solo un archivo que haya creado con **Exportar**. No cree el archivo a mano. La importación añade cada fila como una nueva plantilla. No modifica las plantillas que ya existen.

El archivo tiene una fila de encabezado y diez columnas, en este orden:

```csv
alias,tenant_id,template,created_by,labels,annotations,created_at,updated_at,communication_method,type
```

| Columna | Contenido |
| --- | --- |
| `alias` | El alias de la plantilla. |
| `tenant_id` | El ID del inquilino. La importación añade la plantilla a este inquilino. |
| `template` | El contenido de la plantilla, en JSON. |
| `created_by` | El usuario que creó la plantilla. |
| `labels`, `annotations` | Datos adicionales. Puede dejarlos vacíos. |
| `created_at`, `updated_at` | Fechas. La importación las sustituye por la fecha de la importación. |
| `communication_method` | El método de comunicación de la plantilla (correo electrónico, SMS o documento). |
| `type` | El **Tipo de plantilla**. |

:::caution PRECAUCIÓN
La importación usa el ID de inquilino de cada fila del archivo, no el inquilino en el que ha iniciado sesión. Las filas con un ID de inquilino no válido se omiten. Importe un archivo solo en el inquilino en el que lo exportó.
:::

### Puntos clave {#key-points}

- **Coherencia**: Use formatos predefinidos o predeterminados siempre que sea posible para mantener la coherencia entre los informes.
- **Reutilización**: Una misma plantilla se puede aplicar a varios tipos de informe, si es adecuado.
- **Vista previa**: Después de guardar, previsualice la plantilla en su contexto (por ejemplo, genere un informe de muestra) para confirmar el formato.
- **Actualizaciones**: Al editar una plantilla se verán afectadas todas las generaciones de informes futuras que la usen. Considere crear versiones o cambiar el alias si necesita conservar formatos anteriores.
- **Ejemplos**:  
  - Configuración de la plantilla de Recibo de Voto: seleccione "Recibo de Voto" como Tipo de plantilla, defina el alias y el nombre, elija el botón de opción Documento, configure el diseño y el contenido, y después haga clic en Guardar. Se usará cada vez que se genere un recibo de voto.

### Consejos {#tips}

- Mantenga nombres y alias de plantilla claros y descriptivos para que los administradores identifiquen su finalidad rápidamente.
- Documente cualquier marcador de posición o variable especial que use en las plantillas (por ejemplo, el nombre del votante o la fecha de la elección) en una referencia aparte o en la descripción de la plantilla.
- Pruebe las plantillas de correo electrónico y SMS enviándolas a una dirección o un número de prueba antes de activarlas en producción.
- En las plantillas de documento, asegúrese de que todos los recursos necesarios (logotipos, imágenes) sean accesibles y estén referenciados correctamente.
- Si su sistema admite la previsualización o lenguajes de plantillas (por ejemplo, handlebars o Liquid), incluya datos de muestra para verificar la representación.

## Variables del votante {#voter-variables}

Las plantillas de correo electrónico y SMS, y las plantillas de documento de los informes que muestran un votante cada vez (la carta de información para el votante y el informe de verificación manual), pueden usar estas variables estándar del votante:

- `user.first_name`
- `user.last_name`
- `user.username`
- `user.email`

También están disponibles los atributos personalizados de usuario de Keycloak. El primer valor se expone como `user.<attribute>`. La lista completa de valores sigue disponible como `user.attributes.<attribute>`. Las variables estándar tienen prioridad si un atributo personalizado usa el mismo nombre. El nombre `attributes` está reservado para el mapa completo de atributos. Las listas de valores personalizados vacías solo aparecen en `user.attributes`.

Los informes agregados, como los de participación, actividad, recuento y resultados, muestran muchos votantes a la vez y no reciben un objeto `user`.

Los atributos secretos (`sequent.secret=true`) son la excepción: solo están disponibles cuando la plantilla los declara en `secret_attribute_names`. Consulte [Variables secretas del votante](../user-manual/templates/admin_portal_reference_user-manual_templates.md#secret-voter-variables).

La notación con punto funciona con nombres sencillos, como `dateOfBirth`. Use `lookup` de Handlebars para nombres que contengan puntos o guiones. Por ejemplo:

```handlebars
{{lookup user "sequent.read-only.mobile-number"}}
{{#each (lookup user.attributes "sequent.read-only.mobile-number")}}{{this}} {{/each}}
```

Por ejemplo, un atributo `reference` con los valores `ABC-123` y `legacy-456` se puede representar así:

```handlebars
Primary reference: {{user.reference}}
All references: {{#each user.attributes.reference}}{{this}} {{/each}}
```

### Variables de la carta de información para el votante {#voter-information-letter-variables}

Además de `user`, la plantilla de la carta de información para el votante recibe estos campos:

| Variable | Contenido |
| --- | --- |
| `election_event_name` | El nombre del proceso electoral en el idioma predeterminado del proceso. |
| `issue_date` | La fecha de generación, escrita en el idioma predeterminado del proceso electoral. |
| `voter_first_name` | El nombre del votante. |
| `voter_last_name` | El apellido del votante. |
| `voter_full_name` | Nombre y apellido, separados por un espacio. |
| `username` | El nombre de usuario del votante. |
| `password` | La nueva credencial del votante. Cuando el realm usa un patrón de credencial estructurado, se formatea con ese patrón, por ejemplo `1234-5678-9012-3456`. |
| `voting_portal_url` | La URL de inicio de sesión del portal de votación para el proceso electoral. |
| `logo_url` | La URL del logotipo predeterminado. |

### Funciones de texto {#text-helpers}

Estos auxiliares facilitan la impresión de datos del votante que pueden estar incompletos. Nunca fallan: un valor que falta se muestra como una cadena vacía. Úselos como subexpresiones para combinarlos.

| Auxiliar | Ejemplo | Resultado |
| --- | --- | --- |
| `concat` | `{{concat user.first_name user.middle_names user.last_name}}` | `Jane Doe` cuando `middle_names` falta o está vacío. |
| `concat` con `sep` | `{{concat user.corr_city user.corr_province sep=", "}}` | `Denbigh, ON` |
| `format_pattern` | `{{format_pattern user.corr_postal_code "### ###"}}` | `K0H 1L0` a partir de `K0H1L0`. |
| `upper` | `{{upper user.corr_country_name}}` | `CA` a partir de `Ca`. |

`concat` une cualquier número de valores con un separador, un espacio de forma predeterminada. Recorta cada valor y omite los que faltan, son nulos, están vacíos o solo contienen espacios, de modo que el separador solo aparece entre valores presentes. Los arrays, como `user.attributes.<attribute>`, se aplanan. También se aceptan números y literales entre comillas.

`format_pattern` rellena cada `#` del patrón con el siguiente carácter que no sea un espacio del valor. Cada otro carácter del patrón se copia tal cual. Si el valor no tiene exactamente tantos caracteres como `#` tiene el patrón, el valor recortado se imprime sin cambios.

Por ejemplo, una dirección postal que omite las partes que un votante no tiene:

```handlebars
{{concat user.corr_unit user.corr_street_number user.corr_street_number_suffix user.corr_street}}
{{concat user.corr_city user.corr_province (format_pattern user.corr_postal_code "### ###")}}
{{upper user.corr_country_name}}
```

### Enlaces de voto con datos precargados {#prefilled-voting-links}

Los enlaces `/login` y `/enroll` del portal de votación aceptan hasta cinco campos precargados con el nombre `login_hint__<field>`. Los nombres de campo pueden contener letras, números, `.`, `_` y `-`; los nombres están limitados a 128 caracteres y los valores a 255 caracteres.

Use el auxiliar `url_encode` alrededor de cada valor dinámico de la consulta. Mantenga estáticos los nombres de los parámetros y la estructura de la URL:

```handlebars
https://vote.example/tenant/TENANT_ID/event/EVENT_ID/login?login_hint__username={{url_encode user.username}}&login_hint__reference={{url_encode user.reference}}
```

```handlebars
https://vote.example/tenant/TENANT_ID/event/EVENT_ID/enroll?login_hint__username={{url_encode user.username}}&login_hint__dateOfBirth={{url_encode user.dateOfBirth}}
```

El portal de votación elimina de la URL visible los parámetros de sugerencia aceptados antes de redirigir a Keycloak. Los conjuntos de sugerencias no válidos, duplicados o que superen el límite se rechazan en su totalidad.

#### Política de precarga por campo {#per-field-prefill-policy}

Cada campo de registro decide cómo acepta un valor precargado mediante la anotación `loginHintPrefillPolicy` de su atributo de perfil de usuario de Keycloak:

| Política | Comportamiento |
| --- | --- |
| `EDITABLE` | Precarga el campo y permite al votante cambiar el valor. Se aplica cuando falta la anotación. |
| `READ_ONLY` | Precarga el campo, lo muestra como de solo lectura y rechaza el registro si se ha cambiado el valor enviado. |
| `IGNORE` | Nunca precarga el campo desde un enlace de voto. |

Establezca la anotación en **Realm settings → User profile → Attributes → *(atributo)* → Annotations**, por ejemplo `loginHintPrefillPolicy` = `READ_ONLY`. Un valor no reconocido se trata como `IGNORE`, de modo que un error tipográfico nunca precarga un campo.

Algunos atributos nunca se precargan, independientemente de lo que indique la política: los campos de credenciales, los atributos no gestionados, los atributos que el votante no puede escribir y los atributos que el votante no podría ver ni editar en el formulario. El último grupo incluye un atributo anotado como `hidden`, un atributo que se muestra como campo oculto o de contraseña, y un atributo cuyas anotaciones establecen `html-attribute:disabled`, `hidden`, `inert`, `readonly`, `aria-hidden="true"` o un `html-attribute:style` en línea que lo oculta (`display:none`, `visibility:hidden`, `visibility:collapse` u `opacity:0`).

Las anotaciones cosméticas no afectan a la precarga: `html-attribute:class` y los estilos en línea que solo cambian la apariencia dejan el campo precargable. Cuando se omite un atributo, Keycloak registra el nombre del atributo y el motivo en el nivel `DEBUG` bajo `sequent.keycloak.voter_enrollment.LoginHintPrefill`; los valores de las sugerencias nunca se registran.

La política se aplica a los formularios de registro cuando la acción **Sequent: Login hint registration prefill** forma parte del flujo de registro.

#### Bloquear el nombre de usuario en la página de inicio de sesión {#locking-the-username-on-the-login-page}

La página de inicio de sesión no se genera a partir del perfil de usuario, por lo que no puede leer las anotaciones de atributos. Establezca el atributo del realm `loginHintUsernamePolicy` en `READ_ONLY` (valor predeterminado `EDITABLE`) para mostrar un nombre de usuario precargado como de solo lectura. Un nombre de usuario restaurado mediante *remember me* sigue siendo editable, de modo que los votantes puedan seguir iniciando sesión como otra persona.

Es un bloqueo solo de presentación: evita que un votante edite el campo por accidente, pero Keycloak sigue autenticando el nombre de usuario que se envíe. Es la misma garantía que una página de inicio de sesión sin precarga: la contraseña decide qué cuenta se introduce. El registro es diferente: allí el valor se guarda, por lo que `READ_ONLY` también se aplica en el servidor.

:::warning
Los valores precargados son datos de conveniencia, no afirmaciones de identidad verificadas. Nunca eluden la autenticación, la validación del registro ni las acciones requeridas. `READ_ONLY` impide que un votante cambie un campo precargado; no hace que el valor sea fiable, porque quien creó el enlace lo eligió. No incluya contraseñas, contraseñas de un solo uso (OTP), tokens, secretos, identificadores gubernamentales ni otros valores sensibles inapropiados. La codificación de porcentaje protege la estructura de la URL; no proporciona confidencialidad ni autenticidad.
:::

> **Nota:** Para más información sobre los campos o la sintaxis de las plantillas, consulte la sección Informes de la guía, donde se detalla el uso de plantillas en la configuración de informes.
