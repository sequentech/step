// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyInt;
import static org.mockito.ArgumentMatchers.anyMap;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.lenient;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.Config;
import org.keycloak.common.ClientConnection;
import org.keycloak.credential.CredentialInput;
import org.keycloak.credential.hash.PasswordHashProvider;
import org.keycloak.executors.ExecutorsProvider;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.KeycloakTransaction;
import org.keycloak.models.KeycloakTransactionManager;
import org.keycloak.models.KeycloakUriInfo;
import org.keycloak.models.RealmModel;
import org.keycloak.models.RealmProvider;
import org.keycloak.models.SubjectCredentialManager;
import org.keycloak.models.UserCredentialModel;
import org.keycloak.models.UserLoginFailureModel;
import org.keycloak.models.UserLoginFailureProvider;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.keycloak.services.managers.BruteForceProtector;
import org.keycloak.services.managers.DefaultBlockingBruteForceProtector;
import org.keycloak.services.managers.DefaultBruteForceProtectorFactory;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.junit.jupiter.MockitoExtension;

/**
 * Charging a failure the request did not reserve, against Keycloak's real default protector.
 *
 * <p>The blocking default tracks which request holds each account and, once a failure is processed
 * off-thread, cuts the {@code #brute_force_started} suffix off whatever entry is then stored for
 * that account. A failure charged for an account this request never reserved must not do that to a
 * request that reserved it in between.
 */
@ExtendWith(MockitoExtension.class)
class MultiAttributeCredentialResolverBlockingProtectorTest {

  private static final Map<String, String> DOB = Map.of("dateOfBirth", "19900101");

  private final KeycloakSession session = mock(KeycloakSession.class);
  private final RealmModel realm = mock(RealmModel.class);
  private final UserProvider users = mock(UserProvider.class);
  private final UserLoginFailureProvider loginFailures = mock(UserLoginFailureProvider.class);
  private final KeycloakSessionFactory sessionFactory = mock(KeycloakSessionFactory.class);
  private final Map<String, UserLoginFailureModel> failureModels = new HashMap<>();
  // Work the real protector hands to its "bruteforce" executor, held until the test runs it.
  private final List<Runnable> queuedJobs = new ArrayList<>();
  // The after-completion transactions of the sessions those jobs run in.
  private final List<KeycloakTransaction> jobTransactions = new ArrayList<>();
  private BruteForceProtector protector;
  private UserModel alice;
  private UserModel bob;

  @BeforeEach
  void setUp() {
    DefaultBruteForceProtectorFactory factory = new DefaultBruteForceProtectorFactory();
    Config.Scope config = mock(Config.Scope.class);
    lenient().when(config.getBoolean(eq("allowConcurrentRequests"), any())).thenReturn(false);
    factory.init(config);
    lenient().when(sessionFactory.create()).thenAnswer(invocation -> jobSession());
    factory.postInit(sessionFactory);
    protector = factory.create(session);
    assertTrue(
        protector instanceof DefaultBlockingBruteForceProtector,
        "Keycloak 26.7's default protector blocks concurrent requests unless configured otherwise");

    lenient().when(realm.getId()).thenReturn("realm");
    lenient().when(realm.isBruteForceProtected()).thenReturn(true);
    lenient()
        .when(loginFailures.getUserLoginFailure(any(), any()))
        .thenAnswer(invocation -> failureModels.get(invocation.<String>getArgument(1)));
    PasswordHashProvider hasher = mock(PasswordHashProvider.class);
    lenient().when(session.getProvider(PasswordHashProvider.class)).thenReturn(hasher);
    lenient().when(session.getProvider(BruteForceProtector.class)).thenReturn(protector);
    lenient().when(session.getKeycloakSessionFactory()).thenReturn(sessionFactory);
    lenient().when(session.users()).thenReturn(users);
    lenient().when(session.loginFailures()).thenReturn(loginFailures);
    lenient().when(session.singleUseObjects()).thenReturn(new FakeSingleUseObjectProvider());
    KeycloakContext context = mock(KeycloakContext.class);
    lenient().when(session.getContext()).thenReturn(context);
    lenient().when(context.getConnection()).thenReturn(mock(ClientConnection.class));
    lenient().when(context.getUri()).thenReturn(mock(KeycloakUriInfo.class));

    alice = user("alice");
    bob = user("bob");
    lenient()
        .when(users.searchForUserStream(eq(realm), anyMap(), eq(0), anyInt()))
        .thenAnswer(invocation -> Stream.of(alice, bob));
  }

  private UserModel user(String id) {
    UserModel user = mock(UserModel.class);
    lenient().when(user.getId()).thenReturn(id);
    lenient().when(user.isEnabled()).thenReturn(true);
    SubjectCredentialManager credentials = mock(SubjectCredentialManager.class);
    lenient().when(user.credentialManager()).thenReturn(credentials);
    lenient()
        .when(credentials.isValid(any(CredentialInput.class)))
        .thenAnswer(
            invocation -> id.equals(((UserCredentialModel) invocation.getArgument(0)).getValue()));
    failureModels.put(id, mock(UserLoginFailureModel.class));
    return user;
  }

  /** A session as the protector's executor job gets it: its own transaction manager. */
  private KeycloakSession jobSession() {
    KeycloakSession job = mock(KeycloakSession.class);
    KeycloakTransactionManager transactions = mock(KeycloakTransactionManager.class);
    lenient()
        .doAnswer(
            invocation -> {
              jobTransactions.add(invocation.getArgument(0));
              return null;
            })
        .when(transactions)
        .enlistAfterCompletion(any(KeycloakTransaction.class));
    ExecutorService executor = mock(ExecutorService.class);
    lenient()
        .doAnswer(
            invocation -> {
              queuedJobs.add(invocation.getArgument(0));
              return null;
            })
        .when(executor)
        .execute(any(Runnable.class));
    ExecutorsProvider executors = mock(ExecutorsProvider.class);
    lenient().when(executors.getExecutor("bruteforce")).thenReturn(executor);
    lenient().when(job.getProvider(ExecutorsProvider.class)).thenReturn(executors);
    lenient().when(job.getTransactionManager()).thenReturn(transactions);
    lenient().when(job.getContext()).thenReturn(mock(KeycloakContext.class));
    lenient().when(job.realms()).thenReturn(mock(RealmProvider.class));
    lenient().when(job.loginFailures()).thenReturn(loginFailures);
    return job;
  }

  /** A request on its own thread, as Keycloak's HTTP workers are, asking about one account. */
  private final class OtherRequest {
    private final String threadName;
    private final KeycloakSession requestSession = mock(KeycloakSession.class);

    OtherRequest(String threadName) {
      this.threadName = threadName;
      KeycloakContext context = mock(KeycloakContext.class);
      lenient()
          .when(context.getAuthenticationSession())
          .thenReturn(mock(AuthenticationSessionModel.class));
      lenient().when(requestSession.getContext()).thenReturn(context);
      lenient()
          .when(requestSession.getTransactionManager())
          .thenReturn(mock(KeycloakTransactionManager.class));
      lenient().when(requestSession.loginFailures()).thenReturn(loginFailures);
    }

    /** Reserves the account, as the browser form and Direct Grant do through the protector. */
    boolean reserves(UserModel user) {
      AtomicBoolean disabled = new AtomicBoolean(true);
      Thread thread =
          new Thread(
              () -> disabled.set(protector.isTemporarilyDisabled(requestSession, realm, user)),
              threadName);
      thread.start();
      try {
        thread.join();
      } catch (InterruptedException e) {
        throw new IllegalStateException(e);
      }
      return !disabled.get();
    }
  }

  private MultiAttributeCredentialResolver.Resolution wrongPinForSharedDateOfBirth() {
    return MultiAttributeCredentialResolver.resolveAuthenticatedUser(
        session,
        realm,
        List.of("dateOfBirth"),
        DOB,
        "wrong",
        new MultiAttributeCredentialResolver.ThrottleConfig(10, 10, 60),
        MultiAttributeCredentialResolver.MatchPolicy.REJECT_AMBIGUOUS);
  }

  /** Runs the queued off-thread jobs and the transactions they enlist, as Keycloak would. */
  private void finishOffThreadWork() {
    new ArrayList<>(queuedJobs).forEach(Runnable::run);
    queuedJobs.clear();
    for (KeycloakTransaction transaction : jobTransactions) {
      transaction.begin();
      transaction.commit();
    }
    jobTransactions.clear();
  }

  /**
   * Quarkus names its workers {@code executor-thread-N}: below 1000 that is shorter than the 20
   * characters the blocking protector's removal cuts off, so the removal throws.
   */
  @Test
  void sharedFailure_overlappingAnotherRequestsReservation_doesNotBreakIt_shortThreadName() {
    assertSharedFailureLeavesReservationIntact("executor-thread-7");
  }

  /** From worker 1000 on the removal does not throw: it truncates the owner's entry instead. */
  @Test
  void sharedFailure_overlappingAnotherRequestsReservation_doesNotBreakIt_longThreadName() {
    assertSharedFailureLeavesReservationIntact("http-executor-thread-1000");
  }

  private void assertSharedFailureLeavesReservationIntact(String otherThreadName) {
    // This request charges both candidates; their failures are queued for the off-thread job.
    assertTrue(wrongPinForSharedDateOfBirth().authenticatedUser().isEmpty());
    assertFalse(queuedJobs.isEmpty(), "the failures are processed off-thread");

    // Another login for Alice starts before that job commits and reserves her.
    OtherRequest other = new OtherRequest(otherThreadName);
    assertTrue(other.reserves(alice));

    assertDoesNotThrow(this::finishOffThreadWork);

    // Both failures were recorded, and the other request still holds Alice.
    verify(failureModels.get("alice"), times(1)).incrementFailures();
    verify(failureModels.get("bob"), times(1)).incrementFailures();
    assertTrue(other.reserves(alice), "the other request still owns its reservation");
    assertFalse(
        new OtherRequest("another-request").reserves(alice),
        "a third request still finds Alice held");
  }
}
