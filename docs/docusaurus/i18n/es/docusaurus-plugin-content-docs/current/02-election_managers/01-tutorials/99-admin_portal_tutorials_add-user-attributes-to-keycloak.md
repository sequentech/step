---
id: admin_portal_tutorials_add_user_attributes_to_keycloak
title: Añadir atributos de usuario a Keycloak
description: "El sistema admite añadir atributos de usuario adicionales que aparecen como campos nuevos en los datos del usuario."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Descripción general {#overview}

El sistema admite añadir atributos de usuario adicionales que aparecen como campos nuevos en los datos del usuario. Una vez configurados, estos atributos aparecen en las acciones **Añadir** y **Editar** de la pestaña **Votantes** del portal de administración.

**Nota:** Esta configuración debe hacerse desde la consola de administración de Keycloak.

## Pasos de configuración {#configuration-steps}

1. Inicie sesión en Keycloak y seleccione el realm del proceso electoral que quiere editar.
2. Vaya a **Realm settings** > **Perfil de usuario** > **Create attribute**.
3. Escriba un nombre de atributo, por ejemplo `sex` en el primer ejemplo.
4. Establezca el nombre para mostrar como `${sex}` si quiere reemplazar las traducciones en **Localización** > **Realm overrides**.
5. Continúe con la configuración de las **Annotations** y de otros parámetros (vea los ejemplos siguientes).

Para ver la lista completa de anotaciones y lo que hace cada una con el campo mostrado, consulte [Configurar los campos de inicio de sesión y registro](../02-reference/10-user-profile-login-registration-fields.md).

## Tipos de atributo admitidos {#supported-attribute-types}

Se admiten los siguientes tipos de atributo:

### Sexo {#sex}

Para activar la selección de sexo:

1. En **Annotations** > **Add annotation**, establezca Clave: `Input type`, Valor: `select`.
2. **Add validator** > **Validator type**: `options`, y añada las opciones deseadas (por ejemplo, M, F).
3. De forma opcional, describa cada opción, para que quien edite a un votante no tenga que interpretar los valores almacenados. En **Annotations** > **Add annotation**, establezca Clave: `inputOptionLabels` y un valor JSON que asocie cada opción con su descripción, por ejemplo `{"M": "Male", "F": "Female"}`. Una descripción escrita como `${sex_male}` se resuelve mediante los reemplazos de localización del propio portal de administración (vea más abajo) y, si no existe ningún reemplazo, se muestra en una forma legible de la clave. La anotación `inputOptionLabelsI18nPrefix` solo se aplica en los formularios de inicio de sesión del portal de votación; el portal de administración la ignora, porque lee otro conjunto de traducciones.

El desplegable del portal de administración muestra la opción almacenada seguida de su descripción, por ejemplo `M - Male`, para que el valor guardado en el votante siga visible.

### Fecha de nacimiento {#birth-date}

Para mostrar un campo de fecha:

1. En **Annotations**, establezca Clave: `Input type`, Valor: `html5-date`.
2. Añada validación si lo desea.

### Casillas de verificación {#checkboxes}

1. En **Annotations** > **Add annotation**, establezca Clave: `Input type`, Valor: `multiselect-checkboxes`.
2. Añada una anotación `inputOptionLabels` que contenga los valores y las etiquetas de las opciones.

## Orden y grupos de atributos {#ordering-and-attribute-groups}

El portal de administración muestra los atributos admitidos en el orden que aparece en **Realm settings** > **Perfil de usuario**. Arrastre los atributos hasta el orden necesario en Keycloak; no hace falta CSS del inquilino para controlar el orden semántico ni el orden de teclado.

Para organizar campos relacionados:

1. En **Realm settings** > **Perfil de usuario**, cree un grupo de atributos.
2. Establezca el nombre del grupo, el encabezado que se muestra y, si lo desea, la descripción que se muestra.
3. Asigne los atributos al grupo al editar cada atributo.

Solo se muestran los grupos que contienen un campo visible para el administrador actual. Los atributos consecutivos de un mismo grupo comparten una sección. Si el mismo grupo aparece en partes separadas del perfil, cada tramo consecutivo se muestra como una sección aparte, para conservar el orden de los campos configurado. Los atributos sin grupo se muestran en una sección sin título.

Los títulos de grupo usan el encabezado de visualización de Keycloak y, si no existe, el nombre del grupo. Los valores escritos como expresiones de Keycloak, por ejemplo `${personal_details}`, se muestran como etiquetas legibles. Las descripciones de grupo se muestran debajo del título.

## Personalizar el editor de votantes con CSS del inquilino {#customizing-the-voter-editor-with-tenant-css}

El CSS del inquilino configurado en **Configuración** > **Personalización de apariencia** puede usar los siguientes selectores compatibles:

- `.voter-editor` y `[data-mode="create"|"edit"]`: raíz del editor y modo.
- `.voter-editor__groups`: conjunto de secciones de atributos.
- `.voter-attribute-group` y `[data-group-name="..."]`: un tramo consecutivo de un grupo de atributos. El valor del dato es el nombre bruto del grupo en Keycloak.
- `.voter-attribute-group__legend`: título del grupo.
- `.voter-attribute-group__description`: descripción del grupo.
- `.voter-attribute-group__grid`: campos dentro de un grupo.
- `.voter-field`, `[data-field-name="..."]`, `[data-input-type="..."]` y `[data-required="true"|"false"]`: contenedor del campo y sus metadatos. Los nombres de los campos son los nombres brutos y canónicos de Keycloak.

Los campos propios de Step también tienen nombres estables: `enabled`, `area`, `password`, `confirm_password` y `password_temporary`.

El siguiente ejemplo completo crea tarjetas con fondo suave y borde para los grupos de atributos con nombre, usa una cuadrícula responsiva de dos columnas, mantiene blancos los controles de formulario y permite que la dirección postal ocupe las dos columnas. La exclusión de los nombres vacíos deja sin recuadro la sección sin título que contiene los campos propios de Step.

```css
.voter-editor .voter-attribute-group[data-group-name]:not([data-group-name=""]) {
  min-width: 0;
  margin: 0 0 24px;
  border: 1px solid rgba(15, 5, 76, 0.16);
  border-radius: 12px;
  padding: 20px 22px 24px;
  background-color: rgba(15, 5, 76, 0.02);
}

.voter-editor .voter-attribute-group__legend {
  padding: 0 8px;
  color: #0f054c;
  font-size: 16px;
  font-weight: 700;
  line-height: 1.4;
}

.voter-editor .voter-attribute-group__description {
  margin-bottom: 24px;
  color: #666670;
}

.voter-editor .voter-attribute-group__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  column-gap: 16px;
  row-gap: 20px;
}

.voter-editor [data-field-name="postal_address"] {
  grid-column: 1 / -1;
}

@media (max-width: 700px) {
  .voter-editor .voter-attribute-group[data-group-name]:not([data-group-name=""]) {
    padding: 16px;
  }

  .voter-editor .voter-attribute-group__grid {
    grid-template-columns: 1fr;
  }
}

.voter-editor .voter-attribute-group[data-group-name]:not([data-group-name=""]) .MuiInputBase-root {
  background-color: #fff;
}
```

No use como selectores los nombres de clase generados por MUI o Emotion, como `.css-abc123`; son detalles de implementación y pueden cambiar durante las actualizaciones. `.MuiInputBase-root` del ejemplo es una clase estable de componente MUI, con alcance dentro del selector estable del grupo de votantes. La propiedad `order` de CSS solo puede cambiar el orden visual, no el orden del teclado ni el orden de lectura de los lectores de pantalla, por eso configure el orden de los campos en Keycloak.

## Ocultar un atributo {#hiding-an-attribute}

Un atributo que se transporta en el votante, pero que no debe verse ni editarse, se puede marcar como oculto: en **Annotations** > **Add annotation**, establezca Clave: `hidden`, Valor: `true`.

El valor debe ser exactamente `true`. La comparación es literal, así que `TRUE` o un valor con espacios alrededor no oculta nada: los formularios del votante leen la anotación de la misma forma, y una comparación más flexible aquí ocultaría un atributo a los administradores mientras los votantes seguirían viéndolo.

Un atributo oculto se omite de los formularios de inscripción e inicio de sesión del votante, y de la lista de votantes del portal de administración y de sus formularios de creación y edición: no aparece como columna, no se ofrece en el selector de columnas ni en los filtros, y no se muestra como campo. Las pantallas de **Aprobaciones** no se ven afectadas y siguen mostrándolo.

Ocultar un atributo no lo elimina ni lo altera. Su valor sigue almacenado, se mantiene al editar un votante y se incluye al exportar votantes.

Por tanto, ocultar sirve para mantener un formulario legible, no para restringir el acceso: el valor sigue enviándose al navegador y aparece en las exportaciones. Para controlar quién puede leer o escribir un atributo, configure sus permisos en **Realm settings** > **Perfil de usuario**.

No marque un atributo como oculto y obligatorio a la vez: en ese caso sería imposible crear un votante desde el portal de administración, porque el campo que exige no lo muestra el formulario.

Tenga en cuenta que esto es distinto de establecer la anotación `Input type` como `hidden`, que es la forma propia de Keycloak de mostrar un atributo como entrada oculta en un formulario, en lugar de mantenerlo fuera del formulario.

## Proteger un atributo del votante como secreto {#protecting-a-voter-attribute-as-secret}

Un atributo del proceso electoral se puede almacenar como **atributo secreto del votante** cifrado. Use esta opción para datos que necesitan los administradores o los resultados a nivel de votante, pero que no deben exponerse en las listas ordinarias de votantes ni en las exportaciones predeterminadas. Un secreto personalizado que cumpla los requisitos también puede servir como credencial de los autenticadores **Multi-Attribute + Password** configurados explícitamente.

En **Annotations** > **Add annotation**, establezca:

| Clave | Valor |
|---|---|
| `sequent.secret` | `true` |

El valor también puede ser el booleano JSON `true` cuando el perfil se configura mediante una API. Los atributos secretos solo se admiten en realms de procesos electorales.

Antes de activar la anotación:

- Restrinja los permisos de visualización y edición del atributo del perfil de usuario a los administradores. No use un atributo secreto en el registro de votantes, en la coincidencia de inicio de sesión por atributo de identidad, en asignaciones de tokens, en la conciliación, en filtros, en la ordenación, en comprobaciones de unicidad ni en el autoservicio del votante. Keycloak almacena texto cifrado, así que estas funciones no pueden usar su valor original. La excepción de autenticación admitida es la [verificación de credenciales con atributo cifrado](./101-admin_portal_tutorials_multi-attribute-password-login.md#optional-use-an-encrypted-voter-attribute-as-the-credential): la extensión lo descifra en el servidor y verifica la contraseña introducida. Esto requiere la política `SECRET_ATTRIBUTE` y la clave maestra correspondiente en Keycloak; la anotación por sí sola no cambia el comportamiento del inicio de sesión.
- No configure un validador de valores de Keycloak en el atributo. El validador `person-name-prohibited-characters` es la única excepción admitida, porque acepta el sobre cifrado. Una regla de **Required field** sí se admite.
- Mantenga cada valor en texto plano en 150 bytes o menos. El sobre cifrado debe caber en la columna de valores de atributo de Keycloak de 255 caracteres que usan las importaciones y los listados de votantes.
- Proteja o migre los valores en texto plano existentes antes de activar la anotación. Los valores existentes no se cifran de forma retroactiva, y Step se niega a mostrar un valor sin cifrar como alternativa.

Solo los atributos personalizados del votante pueden ser secretos. Los siguientes atributos de identidad y operativos no pueden serlo:

- `username`, `email`, `first_name`, `last_name`, `dateOfBirth`, `area-id` y `tenant-id`
- `authorized-election-ids`, `authorized-to-election-alias` y `permission_labels`
- `vote-weight`, `voted-channel` y `disable-comment`
- `sequent.read-only.id-card-number-validated` y `sequent.read-only.mobile-number`

Step lee la configuración de atributos secretos a través de una caché breve, así que un cambio en la anotación puede tardar hasta 30 segundos en reflejarse en la lista y en el editor de votantes. Si la configuración no es válida (un atributo prohibido o un validador de valores en un atributo secreto), las listas de votantes siguen ocultando el atributo, pero mostrarlo, editarlo, importarlo, exportarlo y los resultados a nivel de votante se niegan a usarlo hasta que se corrija el perfil.

Secreto y oculto son clasificaciones distintas. Un atributo oculto se omite del editor, pero su texto en claro puede llegar al navegador y a las exportaciones ordinarias. Un atributo secreto se cifra en reposo, se elimina de las rutas ordinarias de listas, filtros y exportaciones, y se representa con un valor enmascarado en el editor de votantes.

Los operadores autorizados pueden incluir de forma explícita los secretos descifrados en las [exportaciones CSV de votantes y del proceso electoral protegidas con contraseña](./20-admin_portal_tutorials_export-data.md). Quien pueda mostrar o exportar un secreto usado para iniciar sesión puede autenticarse como ese votante; conceda el permiso de lectura de secretos en consecuencia. Las importaciones CSV aceptan texto en claro y lo cifran para almacenarlo.

Combinar `hidden=true` con `sequent.secret=true` mantiene el campo fuera del editor, pero sigue disponible para las exportaciones de secretos autorizadas y para las plantillas de comunicación declaradas explícitamente.

Los usuarios necesitan permisos independientes para mostrar o modificar valores secretos. Consulte [Permisos](../02-reference/user-manual/users-and-roles/users-and-roles_permissions.md) y [Crear votantes](./07-admin_portal_tutorials_create-voters.md#secret-voter-fields).

## Limitar el número de caracteres {#limiting-the-number-of-characters}

Para acotar el texto que acepta un atributo, en **Add validator** > **Validator type**, seleccione `length` y establezca un mínimo, un máximo o ambos. Keycloak aplica los límites al guardar el registro.

El portal de administración indica los límites debajo del campo, para que se conozcan antes de incumplirlos, y comprueba el valor al salir del campo. Un valor que incumple un límite marca el campo e indica qué límite ha incumplido, y el votante no se puede guardar hasta corregirlo.

Los límites que nadie escribe no se indican, para mantener el formulario legible: un máximo de cientos, que los atributos base de Keycloak llevan como estructura, y un mínimo de uno, que solo indica que el valor está presente. Aun así se comprueban y se informan si se incumplen.

Un campo con máximo también deja de aceptar caracteres al alcanzarlo, así que el máximo no se puede superar escribiendo. Ese recuento es de los caracteres tal como se escriben, así que un valor con espacios de relleno puede dejar de aceptarse un poco antes de que el validador lo rechace. Tenga en cuenta que pegar un valor más largo en ese campo conserva solo lo que cabe, sin aviso, y que un valor almacenado que ya supera el máximo solo puede acortarse, nunca ampliarse: en ese caso el campo informa de que el valor es demasiado largo hasta que se ajuste al límite. Un mínimo no se puede aplicar mientras se escribe, así que solo se comprueba al salir del campo.

De forma predeterminada, Keycloak mide el valor sin los espacios iniciales y finales, y el portal de administración lo mide igual. Establecer la opción `trim-disabled` del validador cambia ambos.

Un límite que se incumple de todos modos —por ejemplo, un valor escrito por una importación o un atributo que el formulario no muestra— se rechaza al guardar, e informa indicando cada campo y el límite que incumplió. En los procesos electorales de tipo Datafix, el guardado lo realiza una tarea en segundo plano, así que el rechazo se informa a través de esa tarea en lugar de en el formulario.

## Reemplazos de localización {#localization-overrides}

Para personalizar la etiqueta visible de un atributo de usuario, añada un reemplazo de traducción en el portal de administración en **Configuración** > **Localización** > **Añadir**.

Las claves de los atributos de usuario deben usar el prefijo `usersAndRolesScreen.users.fields.` seguido del nombre del atributo.

**Ejemplo:** para mostrar el atributo `personal_administrative_number` como "PAN", añada lo siguiente a la configuración de localización del inquilino:

Clave: `usersAndRolesScreen.users.fields.personal_administrative_number`
Valor: `PAN`

Esto funciona con cualquier nombre de atributo, incluidos los atributos personalizados que no están en las traducciones predeterminadas.
