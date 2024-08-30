package sequent.keycloak.admin.ui;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.keycloak.Config;
import org.keycloak.component.ComponentModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.provider.ProviderConfigurationBuilder;
import org.keycloak.services.ui.extend.UiTabProvider;
import org.keycloak.services.ui.extend.UiTabProviderFactory;

public class ThemeUiTab implements UiTabProvider, UiTabProviderFactory<ComponentModel> {

  private KeycloakSession session;

  @Override
  public String getId() {
    return "Attributes";
  }

  @Override
  public String getHelpText() {
    return null;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}

  @Override
  public void onCreate(KeycloakSession session, RealmModel realm, ComponentModel model) {
    realm.setAttribute("logo", model.get("logo"));
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    final ProviderConfigurationBuilder builder = ProviderConfigurationBuilder.create();
    builder
        .property()
        .name("customCss")
        .label("Set custom CSS")
        .helpText("The CSS to override login pages")
        .type(ProviderConfigProperty.TEXT_TYPE)
        .add();
    return builder.build();
  }

  @Override
  public String getPath() {
    return "/:realm/realm-settings/:tab?";
  }

  @Override
  public Map<String, String> getParams() {
    Map<String, String> params = new HashMap<>();
    params.put("tab", "attributes");
    return params;
  }
}
