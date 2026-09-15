<#--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#--  jQuery + the filterSelectAttribute() function user-profile-commons.ftl's selectTag macro
      wires up via an attribute's filterSelectAttribute annotation (onchange="filterSelectAttribute(...)").
      Shared by register.ftl and login.ftl - any select-typed attribute rendered through
      user-profile-commons.ftl's macros (including a matchAttributes entry) needs this loaded,
      not just ones reached via register.ftl's own top-level userProfileFormFields call.  -->

<#macro assets>
    <#-- jQuery -->
    <script type="text/javascript" src="${url.resourcesPath}/js/jquery-3.7.1.slim.min.js"></script>

    <#-- Filter for select inputs -->
    <script>
        function filterSelectAttribute(e, elementId) {
            e = e || window.event;
            var selectElement = e.target;
            var value = selectElement.value;

            let first = null;
            $('#' + elementId + ' option').hide();
            $('#' + elementId).find('option').filter(function() {
                var optionValue = $(this)[0].value;
                let found = optionValue.indexOf(value) != -1;
                if (found && first === null) {
                    first = optionValue;
                }
                return found;
            }).show();

            // Set default value
            $('#' + elementId).val(first);
        }
    </script>
</#macro>
