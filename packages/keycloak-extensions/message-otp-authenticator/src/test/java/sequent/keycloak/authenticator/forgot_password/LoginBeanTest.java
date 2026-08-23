// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.ArgumentMatchers.isNull;
import static org.mockito.Mockito.lenient;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.util.List;
import java.util.Map;
import java.util.Set;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.models.KeycloakSession;
import org.keycloak.userprofile.AttributeMetadata;
import org.keycloak.userprofile.Attributes;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;

@ExtendWith(MockitoExtension.class)
class LoginBeanTest {

  @Mock private KeycloakSession session;
  @Mock private UserProfileProvider provider;
  @Mock private UserProfile profile;
  @Mock private Attributes profileAttributes;

  @BeforeEach
  void setUp() {
    when(session.getProvider(UserProfileProvider.class)).thenReturn(provider);
    when(provider.create(eq(UserProfileContext.REGISTRATION), isNull(), isNull()))
        .thenReturn(profile);
    when(profile.getAttributes()).thenReturn(profileAttributes);
  }

  @Test
  void nonReadableSelectedAttributeExposesOnlySafeRenderingMetadata() {
    AttributeMetadata phone =
        declare(
            "phone",
            "Phone number",
            Map.of(
                "inputType", "html5-tel",
                "inputHelperTextBefore", "Enter your phone number",
                "html-attribute:autocomplete", "off autofocus onfocus=alert(1)",
                "html-attribute:onfocus", "steal()",
                "disableAttribute", "other",
                "default", "+15551234567"));
    when(profileAttributes.isRequired("phone")).thenReturn(false);

    LoginBean bean = new LoginBean(session, List.of("phone"));
    LoginBean.Attribute rendered = bean.getAttributesByName().get("phone");

    assertEquals("Phone number", rendered.getDisplayName());
    assertEquals("html5-tel", rendered.getAnnotations().get("inputType"));
    assertEquals("Enter your phone number", rendered.getAnnotations().get("inputHelperTextBefore"));
    assertFalse(rendered.getAnnotations().containsKey("html-attribute:autocomplete"));
    assertFalse(rendered.getAnnotations().containsKey("html-attribute:onfocus"));
    assertFalse(rendered.getAnnotations().containsKey("disableAttribute"));
    assertFalse(rendered.getAnnotations().containsKey("default"));
    verify(profileAttributes, never()).getReadable();
    verify(profileAttributes, never()).isRequired("phone");

    assertFalse(rendered.isRequired());
    verify(profileAttributes).isRequired("phone");
    assertEquals("phone", phone.getName());
  }

  @Test
  void exposesOnlySelectedFieldsAndNeverReflectsSubmittedOrStoredValues() {
    declare("phone", "Phone number", Map.of("inputType", "html5-tel"));
    declare("internalRiskScore", "Internal risk score", Map.of());

    LoginBean bean = new LoginBean(session, List.of("phone"));

    assertEquals(Set.of("phone"), bean.getAttributesByName().keySet());
    assertEquals(List.of(), bean.getAttributesByName().get("phone").getValues());
    assertEquals("", bean.getAttributesByName().get("phone").getValue());
    assertFalse(bean.getAttributesByName().containsKey("internalRiskScore"));
  }

  @Test
  void preservesConfiguredNamespacedAndCustomDisplayNameKeys() {
    declare("dateOfBirth", "${profile.attributes.dateOfBirth}", Map.of("inputType", "html5-date"));
    declare("nationalId", "${custom.nationalId}", Map.of());
    declare("phone", "Phone number", Map.of("inputType", "html5-tel"));

    LoginBean bean = new LoginBean(session, List.of("dateOfBirth", "nationalId", "phone"));

    assertEquals(
        "${profile.attributes.dateOfBirth}",
        bean.getAttributesByName().get("dateOfBirth").getDisplayName());
    assertEquals(
        "${custom.nationalId}", bean.getAttributesByName().get("nationalId").getDisplayName());
    assertEquals("Phone number", bean.getAttributesByName().get("phone").getDisplayName());
  }

  @Test
  void dependentFieldTargetMustAlsoBeAnExplicitMatchAttribute() {
    declare(
        "country",
        "Country",
        Map.of("inputType", "select", "filterSelectAttribute", "municipality"));
    declare("municipality", "Municipality", Map.of("inputType", "select"));

    LoginBean countryOnly = new LoginBean(session, List.of("country"));
    LoginBean both = new LoginBean(session, List.of("country", "municipality"));

    assertFalse(
        countryOnly
            .getAttributesByName()
            .get("country")
            .getAnnotations()
            .containsKey("filterSelectAttribute"));
    assertEquals(
        "municipality",
        both.getAttributesByName().get("country").getAnnotations().get("filterSelectAttribute"));
  }

  @Test
  void multivaluedControlsBecomeScalarControlsForMatching() {
    declare("regions", "Regions", Map.of("inputType", "multiselect"));
    declare("channels", "Channels", Map.of("inputType", "multiselect-checkboxes"));

    LoginBean bean = new LoginBean(session, List.of("regions", "channels"));

    assertEquals(
        "select", bean.getAttributesByName().get("regions").getAnnotations().get("inputType"));
    assertEquals(
        "select-radiobuttons",
        bean.getAttributesByName().get("channels").getAnnotations().get("inputType"));
    assertFalse(bean.getAttributesByName().get("regions").isMultivalued());
    assertFalse(bean.getAttributesByName().get("channels").isMultivalued());
  }

  private AttributeMetadata declare(
      String name, String displayName, Map<String, Object> annotations) {
    AttributeMetadata metadata = mock(AttributeMetadata.class);
    lenient().when(metadata.getName()).thenReturn(name);
    lenient().when(metadata.getAttributeDisplayName()).thenReturn(displayName);
    lenient().when(metadata.getAnnotations()).thenReturn(annotations);
    lenient().when(metadata.getValidators()).thenReturn(List.of());
    lenient().when(profileAttributes.getMetadata(name)).thenReturn(metadata);
    return metadata;
  }
}
