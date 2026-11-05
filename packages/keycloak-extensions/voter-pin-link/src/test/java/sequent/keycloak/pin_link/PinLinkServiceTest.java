// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.pin_link;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import java.nio.charset.StandardCharsets;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import sequent.keycloak.pin_link.PinLinkService.Outcome;
import sequent.keycloak.pin_link.PinLinkService.Reason;
import sequent.keycloak.pin_link.PinLinkService.Voter;

/** Runs the decision against a real SQL database, with one connection per simulated replica. */
class PinLinkServiceTest {
  private static final String MASTER =
      "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
  private static final String TENANT = "11111111-1111-4111-8111-111111111111";
  private static final String EVENT = "22222222-2222-4222-8222-222222222222";
  private static final String VOTER_ID = "33333333-3333-4333-8333-333333333333";
  private static final String USERNAME = "member-0042";
  private static final String PIN = "123456789012";
  private static final Voter VOTER = new Voter(TENANT, EVENT, VOTER_ID, USERNAME);

  private final PinLinkConfig config =
      PinLinkConfig.from(
              java.util.Map.of(
                  PinLinkConfig.POLICY, "ONE_TIME_LINK",
                  PinLinkConfig.EXPIRES_AT, "2099-01-01T00:00:00Z",
                  PinLinkConfig.MAX_ATTEMPTS, "3"))
          .orElseThrow();
  private final PinLinkStore store = new PinLinkStore("user_attribute", "user_entity");
  private final AtomicInteger audits = new AtomicInteger();
  private String url;
  private Connection admin;
  private String token;

  @BeforeEach
  void createVoter() throws Exception {
    url =
        "jdbc:h2:mem:"
            + UUID.randomUUID()
            + ";MODE=PostgreSQL;NON_KEYWORDS=VALUE;DB_CLOSE_DELAY=-1;LOCK_TIMEOUT=30000";
    admin = DriverManager.getConnection(url);
    try (Statement statement = admin.createStatement()) {
      statement.execute(
          "CREATE TABLE user_entity (id VARCHAR(36) PRIMARY KEY, realm_id VARCHAR(36),"
              + " username VARCHAR(255))");
      statement.execute(
          "CREATE TABLE user_attribute (id VARCHAR(36) PRIMARY KEY, user_id VARCHAR(36),"
              + " name VARCHAR(255), value VARCHAR(255))");
    }
    insertUser(VOTER_ID, USERNAME);
    insertAttribute(VOTER_ID, "pin", envelope(TENANT, EVENT, VOTER_ID, "pin", PIN));
    token = deriveToken(TENANT, EVENT, VOTER_ID, PIN);
    insertAttribute(VOTER_ID, config.tokenHashAttribute(), PinLinkToken.lookupHash(token));
  }

  @AfterEach
  void close() throws Exception {
    admin.close();
  }

  @Test
  void theRightUsernameRevealsThePinAndDeletesTheHash() throws Exception {
    Outcome outcome = post(USERNAME);
    assertThat(outcome.reason()).isEqualTo(Reason.REVEALED);
    assertThat(new String(outcome.pin(), StandardCharsets.UTF_8)).isEqualTo(PIN);
    assertThat(values(config.tokenHashAttribute())).isEmpty();
    assertThat(audits).hasValue(1);
  }

  @Test
  void aUsernameIsComparedIgnoringCaseAndSurroundingSpace() throws Exception {
    assertThat(post("  MEMBER-0042 ").reason()).isEqualTo(Reason.REVEALED);
  }

  @Test
  void theLinkIsDeadAfterASuccessEvenForANewReplicaOrAfterARestart() throws Exception {
    assertThat(post(USERNAME).reason()).isEqualTo(Reason.REVEALED);
    assertThat(post(USERNAME).reason()).isEqualTo(Reason.NOT_AVAILABLE);
    assertThat(audits).hasValue(1);
  }

  /**
   * Sixteen simultaneous POSTs, each on its own connection like separate replicas, spend the link
   * once and record one audit entry; the rest find it gone.
   */
  @Test
  void concurrentPostsAcrossReplicasHaveExactlyOneWinner() throws Exception {
    int replicas = 16;
    ExecutorService pool = Executors.newFixedThreadPool(replicas);
    CountDownLatch start = new CountDownLatch(1);
    List<Future<Reason>> results = new ArrayList<>();
    for (int i = 0; i < replicas; i++) {
      results.add(
          pool.submit(
              () -> {
                start.await();
                return post(USERNAME).reason();
              }));
    }
    start.countDown();
    int winners = 0;
    for (Future<Reason> result : results) {
      if (result.get() == Reason.REVEALED) {
        winners++;
      } else {
        assertThat(result.get()).isEqualTo(Reason.NOT_AVAILABLE);
      }
    }
    pool.shutdown();
    assertThat(winners).isEqualTo(1);
    assertThat(audits).hasValue(1);
  }

  /**
   * Simultaneous wrong usernames all count: the row lock keeps one from overwriting another's
   * attempt.
   */
  @Test
  void concurrentWrongAttemptsAreAllCounted() throws Exception {
    int attempts = 3;
    ExecutorService pool = Executors.newFixedThreadPool(attempts);
    List<Future<Reason>> results = new ArrayList<>();
    for (int i = 0; i < attempts; i++) {
      results.add(pool.submit(() -> post("wrong").reason()));
    }
    for (Future<Reason> result : results) {
      assertThat(result.get()).isEqualTo(Reason.WRONG_USERNAME);
    }
    pool.shutdown();
    assertThat(values(config.attemptsAttribute())).containsExactly("3");
  }

  /**
   * At the attempt limit the link is locked: even the right username is refused, the link is not
   * spent and nothing is audited.
   */
  @Test
  void tooManyWrongUsernamesLockTheLinkEvenForTheRightOne() throws Exception {
    for (int i = 0; i < config.maxAttempts(); i++) {
      assertThat(post("not-" + USERNAME).reason()).isEqualTo(Reason.WRONG_USERNAME);
    }
    assertThat(post(USERNAME).reason()).isEqualTo(Reason.LOCKED);
    assertThat(post("not-" + USERNAME).reason()).isEqualTo(Reason.LOCKED);
    assertThat(values(config.tokenHashAttribute())).hasSize(1);
    assertThat(values(config.attemptsAttribute())).containsExactly("3");
    assertThat(audits).hasValue(0);
  }

  @Test
  void aCorruptCounterLocksRatherThanRestarting() throws Exception {
    insertAttribute(VOTER_ID, config.attemptsAttribute(), "not-a-number");
    assertThat(post(USERNAME).reason()).isEqualTo(Reason.LOCKED);
  }

  @Test
  void anAttemptIsCountedForTheVoterTheTokenBelongsToOnly() throws Exception {
    String other = "44444444-4444-4444-8444-444444444444";
    insertUser(other, "member-0043");
    assertThat(post("member-0043").reason()).isEqualTo(Reason.WRONG_USERNAME);
    assertThat(values(config.attemptsAttribute())).containsExactly("1");
    assertThat(valuesOf(other, config.attemptsAttribute())).isEmpty();
  }

  @Test
  void aFailedAuditRollsBackTheConsumeAndShowsNothing() throws Exception {
    try (Connection connection = DriverManager.getConnection(url)) {
      connection.setAutoCommit(false);
      assertThatThrownBy(
              () ->
                  service()
                      .retrieve(
                          connection,
                          VOTER,
                          token,
                          USERNAME,
                          () -> {
                            throw new IllegalStateException("broker down");
                          }))
          .isInstanceOf(PinLinkService.AuditFailedException.class);
      connection.rollback();
    }
    assertThat(values(config.tokenHashAttribute())).hasSize(1);
    assertThat(post(USERNAME).reason()).isEqualTo(Reason.REVEALED);
  }

  @Test
  void anEnvelopeMovedFromAnotherVoterFailsAndKeepsTheLink() throws Exception {
    replacePin(envelope(TENANT, EVENT, "other-voter", "pin", PIN));
    assertThat(post(USERNAME).reason()).isEqualTo(Reason.INVALID_ENVELOPE);
    assertThat(values(config.tokenHashAttribute())).hasSize(1);
    assertThat(audits).hasValue(0);
  }

  @Test
  void anEnvelopeMovedFromAnotherEventFails() throws Exception {
    replacePin(envelope(TENANT, "other-event", VOTER_ID, "pin", PIN));
    assertThat(post(USERNAME).reason()).isEqualTo(Reason.INVALID_ENVELOPE);
  }

  @Test
  void anEnvelopeMovedFromAnotherTenantFails() throws Exception {
    replacePin(envelope("other-tenant", EVENT, VOTER_ID, "pin", PIN));
    assertThat(post(USERNAME).reason()).isEqualTo(Reason.INVALID_ENVELOPE);
  }

  @Test
  void anEnvelopeMovedFromAnotherAttributeFails() throws Exception {
    replacePin(envelope(TENANT, EVENT, VOTER_ID, "password-hint", PIN));
    assertThat(post(USERNAME).reason()).isEqualTo(Reason.INVALID_ENVELOPE);
  }

  @Test
  void aPinThatChangedWithoutItsTokenRefusesTheOldLink() throws Exception {
    replacePin(envelope(TENANT, EVENT, VOTER_ID, "pin", "999999999999"));
    assertThat(post(USERNAME).reason()).isEqualTo(Reason.INVALID_ENVELOPE);
    assertThat(values(config.tokenHashAttribute())).hasSize(1);
  }

  @Test
  void aMissingOrMultiValuedPinRefuses() throws Exception {
    insertAttribute(VOTER_ID, "pin", envelope(TENANT, EVENT, VOTER_ID, "pin", PIN));
    assertThat(post(USERNAME).reason()).isEqualTo(Reason.INVALID_ENVELOPE);
    execute("DELETE FROM user_attribute WHERE name = 'pin'");
    assertThat(post(USERNAME).reason()).isEqualTo(Reason.INVALID_ENVELOPE);
  }

  @Test
  void aTokenForAnotherVoterDoesNotOpenThisOne() throws Exception {
    String otherToken = deriveToken(TENANT, EVENT, "someone-else", PIN);
    try (Connection connection = DriverManager.getConnection(url)) {
      assertThat(
              service()
                  .retrieve(connection, VOTER, otherToken, USERNAME, audits::incrementAndGet)
                  .reason())
          .isEqualTo(Reason.NOT_AVAILABLE);
    }
  }

  @Test
  void lookupIsByHashAndIsLimitedToTheRealm() throws Exception {
    String hash = PinLinkToken.lookupHash(token);
    execute("INSERT INTO user_entity VALUES ('other', 'realm-b', 'x')");
    execute("UPDATE user_entity SET realm_id = 'realm-a' WHERE id = '" + VOTER_ID + "'");
    assertThat(store.findVoterId(admin, "realm-a", config.tokenHashAttribute(), hash))
        .contains(VOTER_ID);
    assertThat(store.findVoterId(admin, "realm-b", config.tokenHashAttribute(), hash)).isEmpty();
    assertThat(store.findVoterId(admin, "realm-a", config.tokenHashAttribute(), USERNAME))
        .isEmpty();
  }

  @Test
  void twoVotersHoldingOneHashCountAsNobody() throws Exception {
    String hash = PinLinkToken.lookupHash(token);
    execute("UPDATE user_entity SET realm_id = 'realm-a'");
    execute("INSERT INTO user_entity VALUES ('other', 'realm-a', 'x')");
    insertAttribute("other", config.tokenHashAttribute(), hash);
    assertThat(store.findVoterId(admin, "realm-a", config.tokenHashAttribute(), hash)).isEmpty();
  }

  @Test
  void aNewPinArmsItsOwnLinkAndKillsTheOldOne() throws Exception {
    String newToken = deriveToken(TENANT, EVENT, VOTER_ID, "999999999999");
    replacePin(envelope(TENANT, EVENT, VOTER_ID, "pin", "999999999999"));
    execute("DELETE FROM user_attribute WHERE name = '" + config.tokenHashAttribute() + "'");
    insertAttribute(VOTER_ID, config.tokenHashAttribute(), PinLinkToken.lookupHash(newToken));
    assertThat(post(USERNAME).reason()).isEqualTo(Reason.NOT_AVAILABLE);
    token = newToken;
    assertThat(new String(post(USERNAME).pin(), StandardCharsets.UTF_8)).isEqualTo("999999999999");
  }

  private PinLinkService service() {
    return new PinLinkService(store, config, MASTER);
  }

  /** One request in its own transaction, committed like the provider's job does. */
  private Outcome post(String username) throws Exception {
    try (Connection connection = DriverManager.getConnection(url)) {
      connection.setAutoCommit(false);
      Outcome outcome =
          service().retrieve(connection, VOTER, token, username, audits::incrementAndGet);
      connection.commit();
      return outcome;
    }
  }

  private static String envelope(
      String tenant, String event, String user, String attribute, String value) throws Exception {
    return TestEnvelopes.encrypt(MASTER, tenant, event, user, attribute, value);
  }

  private static String deriveToken(String tenant, String event, String user, String pin)
      throws Exception {
    return PinLinkToken.derive(MASTER, tenant, event, user, pin.getBytes(StandardCharsets.UTF_8));
  }

  private void replacePin(String envelope) throws Exception {
    execute("UPDATE user_attribute SET value = '" + envelope + "' WHERE name = 'pin'");
  }

  private void insertUser(String id, String username) throws Exception {
    execute("INSERT INTO user_entity VALUES ('" + id + "', 'realm', '" + username + "')");
  }

  private void insertAttribute(String userId, String name, String value) throws Exception {
    try (PreparedStatement statement =
        admin.prepareStatement("INSERT INTO user_attribute VALUES (?, ?, ?, ?)")) {
      statement.setString(1, UUID.randomUUID().toString());
      statement.setString(2, userId);
      statement.setString(3, name);
      statement.setString(4, value);
      statement.executeUpdate();
    }
  }

  private List<String> values(String name) throws Exception {
    return valuesOf(VOTER_ID, name);
  }

  /** The values a voter has stored under an attribute name, read straight from the database. */
  private List<String> valuesOf(String userId, String name) throws Exception {
    try (PreparedStatement statement =
        admin.prepareStatement("SELECT value FROM user_attribute WHERE user_id = ? AND name = ?")) {
      statement.setString(1, userId);
      statement.setString(2, name);
      try (ResultSet rows = statement.executeQuery()) {
        List<String> values = new ArrayList<>();
        while (rows.next()) {
          values.add(rows.getString(1));
        }
        return values;
      }
    }
  }

  private void execute(String sql) throws Exception {
    try (Statement statement = admin.createStatement()) {
      statement.execute(sql);
    }
  }
}
