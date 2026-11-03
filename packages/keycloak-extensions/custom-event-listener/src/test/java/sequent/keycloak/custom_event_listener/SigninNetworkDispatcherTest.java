// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import java.time.Duration;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

class SigninNetworkDispatcherTest {
  private final SigninNetworkSender.Evidence evidence =
      new SigninNetworkSender.Evidence(
          "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
          "6f1c3a6e-2a61-4d6b-9a52-3f0f8a3c2b10",
          "0b9f6c5e-7d3a-4a1e-8c2f-5e4d3c2b1a09",
          "stable-delivery",
          1778511600123L,
          "198.51.100.7",
          "ES",
          "Synthetic browser");

  @Test
  void stalledTransportNeverHoldsCallerAndCapacityIncludesRunningWork() throws Exception {
    SigninNetworkSender sender = mock(SigninNetworkSender.class);
    CountDownLatch started = new CountDownLatch(1);
    CountDownLatch release = new CountDownLatch(1);
    when(sender.deliver(evidence))
        .thenAnswer(
            call -> {
              started.countDown();
              assertTrue(release.await(5, TimeUnit.SECONDS));
              return SigninNetworkSender.Delivery.COMPLETE;
            });
    try (SigninNetworkDispatcher dispatcher =
        new SigninNetworkDispatcher(sender, 1, 1, 3, Duration.ZERO)) {
      assertTimeoutPreemptively(
          Duration.ofSeconds(1), () -> assertTrue(dispatcher.submit(evidence)));
      assertTrue(started.await(1, TimeUnit.SECONDS));
      assertTimeoutPreemptively(
          Duration.ofSeconds(1), () -> assertFalse(dispatcher.submit(evidence)));
      release.countDown();
    } finally {
      release.countDown();
    }
  }

  @Test
  void retriesKeepTheSameImmutableEvidenceAndStopAtSuccess() throws Exception {
    SigninNetworkSender sender = mock(SigninNetworkSender.class);
    CountDownLatch completed = new CountDownLatch(1);
    AtomicInteger attempts = new AtomicInteger();
    when(sender.deliver(evidence))
        .thenAnswer(
            call -> {
              if (attempts.incrementAndGet() == 3) {
                completed.countDown();
                return SigninNetworkSender.Delivery.COMPLETE;
              }
              return SigninNetworkSender.Delivery.RETRY;
            });
    try (SigninNetworkDispatcher dispatcher =
        new SigninNetworkDispatcher(sender, 1, 1, 6, Duration.ZERO)) {
      assertTrue(dispatcher.submit(evidence));
      assertTrue(completed.await(2, TimeUnit.SECONDS));
    }
    assertEquals(3, attempts.get());
    verify(sender, times(3)).deliver(same(evidence));
  }

  @Test
  void retriesAreFiniteAndShutdownRejectsNewWork() throws Exception {
    SigninNetworkSender sender = mock(SigninNetworkSender.class);
    CountDownLatch last = new CountDownLatch(1);
    AtomicInteger attempts = new AtomicInteger();
    when(sender.deliver(evidence))
        .thenAnswer(
            call -> {
              if (attempts.incrementAndGet() == 3) last.countDown();
              return SigninNetworkSender.Delivery.RETRY;
            });
    SigninNetworkDispatcher dispatcher =
        new SigninNetworkDispatcher(sender, 1, 1, 3, Duration.ZERO);
    assertTrue(dispatcher.submit(evidence));
    assertTrue(last.await(2, TimeUnit.SECONDS));
    dispatcher.close();
    assertFalse(dispatcher.submit(evidence));
    assertEquals(3, attempts.get());
  }

  @Test
  void shutdownCancelsPendingDelayedRetryWithoutCallingSenderAgain() throws Exception {
    SigninNetworkSender sender = mock(SigninNetworkSender.class);
    CountDownLatch first = new CountDownLatch(1);
    when(sender.deliver(evidence))
        .thenAnswer(
            call -> {
              first.countDown();
              return SigninNetworkSender.Delivery.RETRY;
            });
    SigninNetworkDispatcher dispatcher =
        new SigninNetworkDispatcher(sender, 1, 1, 3, Duration.ofHours(1));
    assertTrue(dispatcher.submit(evidence));
    assertTrue(first.await(1, TimeUnit.SECONDS));
    dispatcher.close();
    verify(sender).deliver(evidence);
    assertFalse(dispatcher.submit(evidence));
  }

  @Test
  void evidenceCannotExposePrivateValuesThroughAutomaticStringFormatting() {
    String text = evidence.toString();
    assertFalse(text.contains("198.51.100.7"));
    assertFalse(text.contains(evidence.voterId()));
    assertFalse(text.contains("Synthetic browser"));
  }
}
