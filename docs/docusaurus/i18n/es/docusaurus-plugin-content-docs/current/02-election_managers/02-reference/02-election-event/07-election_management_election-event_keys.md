---
id: election_management_election_event_keys
title: Claves
description: "La ceremonia de claves establece la clave privada colectiva que se usa para descifrar los votos y publica una clave pública correspondiente para que los votantes cifren sus papeletas."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


La ceremonia de claves establece la clave privada colectiva que se usa para descifrar los votos y publica la clave pública correspondiente con la que los votantes cifran sus papeletas. Durante esta ceremonia, los fideicomisarios generan conjuntamente fragmentos de la clave privada, de modo que ninguna parte tiene la clave completa. Cada fideicomisario descarga, guarda de forma segura y verifica su fragmento de clave. Solo cuando todos los fideicomisarios completan estos pasos puede el proceso electoral avanzar a etapas como la publicación de la papeleta, la votación y el recuento. Este enfoque distribuido garantiza que los votos cifrados con la clave pública compartida sigan siendo confidenciales y resistentes a manipulaciones, y que la clave privada combinada —que solo se reconstruye cuando un umbral de fideicomisarios colabora— preserve la integridad y la disponibilidad del descifrado durante todo el ciclo de vida de la elección.


## Apertura de la ceremonia de distribución de claves {#opening-key-distribution-ceremony}

Durante la ceremonia de claves criptográficas de un proceso electoral, los fideicomisarios se reúnen para generar las claves criptográficas esenciales que protegen el proceso de votación. Esta ceremonia suele incluir:

- **Generación de claves**: Creación de nuevas claves según los requisitos de seguridad del sistema.
- **Procedimientos de copia de seguridad**: Copia de seguridad de cada clave para garantizar la redundancia y evitar su pérdida.
- **Pasos de verificación**: Realización de pruebas y auditorías criptográficas para validar la integridad y el funcionamiento de las claves.
- **Almacenamiento seguro**: Almacenamiento de las claves de forma segura para que estén listas para su uso durante los procesos electorales, con el fin de cifrar y descifrar datos sensibles de votación.
- **Confidencialidad y autenticidad**: Garantía de que todas las claves siguen siendo confidenciales e inalteradas, lo que mantiene la confianza en el proceso electoral.

---

## Creación de claves criptográficas {#creating-cryptographic-keys}

Para iniciar la ceremonia de claves en el Sistema de Votación en Línea (OVS) de Sequent:

1. **Seleccionar proceso electoral**
   - En el portal de administración, elija el proceso electoral correspondiente.

2. **Vaya a la pestaña Claves**
   - Vaya a la sección “Claves” de ese proceso electoral.

3. **Iniciar ceremonia de claves**
   - Haga clic en **Crear Ceremonia de Claves** (o en el botón o pantalla equivalente).

4. **Ceremonia de claves automática (opcional)**
   - Solo está disponible para procesos electorales con la política “Permitir ceremonias automáticas”.
   - Al activar esta opción, la ceremonia se automatiza y los fideicomisarios no tienen que realizar ninguna acción.

5. **Configurar umbral y fideicomisarios**
   - **Introducir umbral**: Indique el número mínimo de fideicomisarios necesarios para reconstruir o descifrar (el umbral).
   - **Seleccionar fideicomisarios**: Marque los fideicomisarios que participarán en esta ceremonia del proceso electoral.

6. **Crear Ceremonia de Claves**
   - Confirme y envíe para generar las claves.
   - Apruebe cualquier mensaje de confirmación que aparezca.

7. **Acciones del fideicomisario**
   - Cada fideicomisario debe:
     - **Descargar** la clave privada encriptada generada.
     - **Asegurar** varias copias de seguridad de su clave.
     - **Verificar** la clave con el sistema (véase la siguiente sección).

> _Nota_: En la interfaz puede ver indicadores (iconos o estados) que muestran qué fideicomisarios han completado la descarga, la copia de seguridad y la verificación. Por ejemplo, un icono verde indica que se ha completado.

---

## Proceso de distribución de claves criptográficas de los fideicomisarios {#trustee-cryptographic-key-distribution-process}

Cada fideicomisario sigue este procedimiento para descargar, guardar en copia de seguridad y verificar su clave. Cuando todos los fideicomisarios completan estos pasos, la ceremonia de claves queda finalizada y el proceso electoral puede continuar.

1. **Inicie sesión y seleccione el proceso electoral**
   - Inicie sesión en el portal de administración.
   - Seleccione el proceso electoral correspondiente.

2. **Acceda a la invitación a la ceremonia de claves**
   - Haga clic en la acción de invitación a la ceremonia de claves, que puede aparecer destacada (por ejemplo, en un recuadro naranja) o mediante un icono de llave.

3. **Siga las instrucciones de la pantalla**
   - Avance por los pasos guiados para la distribución de claves.

4. **Descargar clave privada encriptada**
   - Descargue el archivo de clave privada encriptada que proporciona el sistema.

5. **Cree copias de seguridad seguras**
   - Haga **varias copias de seguridad seguras** de la clave privada encriptada (por ejemplo, en soportes o dispositivos de almacenamiento seguros independientes).
   - Confirme que existen copias de seguridad antes de continuar.

6. **Confirme la creación de las copias de seguridad**
   - Haga clic en **Siguiente** (o en el equivalente) y confirme que se han creado las copias de seguridad.

7. **Verifique la clave con el sistema**
   - Arrastre y suelte (o suba) el archivo de clave privada encriptada en la interfaz para verificar que coincide con la huella de clave esperada.
   - El sistema confirmará que la verificación se ha realizado correctamente.

8. **Finalización**
   - Cuando todos los fideicomisarios hayan descargado, guardado en copia de seguridad y verificado sus claves, la ceremonia de claves se habrá completado correctamente.
   - El estado del proceso electoral se actualizará para indicar que las claves se han distribuido y verificado.
   - Un fideicomisario participante puede volver a abrir la ceremonia y seleccionar **Verificar clave** para comprobar de nuevo una copia de seguridad existente. Esta opción sigue disponible después de completar la ceremonia y no genera ni descarga una clave nueva.
