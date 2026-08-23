// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.ArgumentMatchers.isNull;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.models.KeycloakSession;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPAttributePermissions;
import org.keycloak.representations.userprofile.config.UPConfig;
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
  void nonReadableConfiguredMatchAttributeStillExposesSanitizedRenderingMetadata() {
    UPAttribute phone = new UPAttribute("phone");
    phone.setDisplayName("Phone number");
    phone.setPermissions(new UPAttributePermissions(Set.of(), Set.of("admin")));
    phone.setAnnotations(
        Map.of(
            "inputType", "html5-tel",
            "inputHelperTextBefore", "Enter your phone number",
            "html-attribute:autocomplete", "tel",
            "html-attribute:onfocus", "steal()",
            "default", "+15551234567"));
    configure(phone);
    when(profileAttributes.isRequired("phone")).thenReturn(false);

    LoginBean bean = new LoginBean(new MultivaluedHashMap<>(), session, List.of("phone"));
    LoginBean.Attribute rendered = bean.getAttributesByName().get("phone");

    assertEquals("Phone number", rendered.getDisplayName());
    assertEquals("html5-tel", rendered.getAnnotations().get("inputType"));
    assertEquals("tel", rendered.getAnnotations().get("html-attribute:autocomplete"));
    assertFalse(rendered.getAnnotations().containsKey("html-attribute:onfocus"));
    assertFalse(rendered.getAnnotations().containsKey("default"));
    assertFalse(rendered.isRequired());
    verify(profileAttributes, never()).getReadable();
  }

  @Test
  void exposesOnlySelectedFieldsAndOnlyCurrentSubmittedValues() {
    UPAttribute phone = new UPAttribute("phone");
    phone.setDefaultValue("persisted-looking-default");
    UPAttribute internal = new UPAttribute("internalRiskScore");
    internal.setAnnotations(Map.of("inputHelperTextBefore", "sensitive schema detail"));
    configure(phone, internal);
    when(profileAttributes.isRequired("phone")).thenReturn(true);
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add("phone", "+573116611420");
    formData.add("password", "must-not-be-exposed-by-profile");

    LoginBean bean = new LoginBean(formData, session, List.of("phone"));

    assertEquals(Set.of("phone"), bean.getAttributesByName().keySet());
    assertEquals(List.of("+573116611420"), bean.getAttributesByName().get("phone").getValues());
    assertTrue(bean.getAttributesByName().get("phone").isRequired());
    assertFalse(bean.getAttributesByName().containsKey("internalRiskScore"));
    assertFalse(bean.getAttributesByName().containsKey("password"));
  }

  @Test
  void normalizesOnlyKeycloakGeneratedProfileTranslationKeys() {
    UPAttribute dateOfBirth = new UPAttribute("dateOfBirth");
    dateOfBirth.setDisplayName("${profile.attributes.dateOfBirth}");
    UPAttribute nationalId = new UPAttribute("nationalId");
    nationalId.setDisplayName("${custom.nationalId}");
    UPAttribute phone = new UPAttribute("phone");
    phone.setDisplayName("Phone number");
    UPAttribute reference = new UPAttribute("reference");
    reference.setDisplayName(" ");
    configure(dateOfBirth, nationalId, phone, reference);

    LoginBean bean =
        new LoginBean(
            new MultivaluedHashMap<>(),
            session,
            List.of("dateOfBirth", "nationalId", "phone", "reference"));

    assertEquals("${dateOfBirth}", bean.getAttributesByName().get("dateOfBirth").getDisplayName());
    assertEquals(
        "${custom.nationalId}", bean.getAttributesByName().get("nationalId").getDisplayName());
    assertEquals("Phone number", bean.getAttributesByName().get("phone").getDisplayName());
    assertEquals("reference", bean.getAttributesByName().get("reference").getDisplayName());
  }

  @Test
  void dependentFieldTargetMustAlsoBeAnExplicitMatchAttribute() {
    UPAttribute country = new UPAttribute("country");
    country.setAnnotations(Map.of("inputType", "select", "filterSelectAttribute", "municipality"));
    UPAttribute municipality = new UPAttribute("municipality");
    configure(country, municipality);
    when(profileAttributes.isRequired("country")).thenReturn(false);

    LoginBean countryOnly = new LoginBean(new MultivaluedHashMap<>(), session, List.of("country"));
    LoginBean both =
        new LoginBean(new MultivaluedHashMap<>(), session, List.of("country", "municipality"));

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

  private void configure(UPAttribute... attributes) {
    UPConfig configuration = new UPConfig();
    configuration.setAttributes(List.of(attributes));
    when(provider.getConfiguration()).thenReturn(configuration);
  }
}
