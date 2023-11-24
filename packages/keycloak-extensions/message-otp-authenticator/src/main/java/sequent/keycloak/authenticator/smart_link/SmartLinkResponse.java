package sequent.keycloak.authenticator.smart_link;

import com.fasterxml.jackson.annotation.JsonProperty;
import lombok.Data;

@Data
public class SmartLinkResponse {
  @JsonProperty("user_id")
  private String userId;

  @JsonProperty("link")
  private String link;

  @JsonProperty("sent")
  private boolean sent;
}
