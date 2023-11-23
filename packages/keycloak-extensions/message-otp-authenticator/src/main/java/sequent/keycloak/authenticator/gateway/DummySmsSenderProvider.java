package sequent.keycloak.authenticator.gateway;

import lombok.extern.jbosslog.JBossLog;
import java.util.HashMap;
import java.util.Map;

@JBossLog
public class DummySmsSenderProvider implements SmsSenderProvider
{
	DummySmsSenderProvider() {
	}

	@Override
	public void send(String phoneNumber, String message)
	{
        log.infov(
            "**Sending dummy sms**:\n\t- phoneNumber={0}\n\t- message={1}",
            phoneNumber,
            message
        );
	}

	@Override
	public void close() {
	}
}
