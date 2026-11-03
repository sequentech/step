// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import java.time.Duration;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.ScheduledThreadPoolExecutor;
import java.util.concurrent.Semaphore;
import java.util.concurrent.TimeUnit;
import lombok.extern.jbosslog.JBossLog;

/** Bounded, volatile delivery; no token or HTTP request runs on the login thread. */
@JBossLog
final class SigninNetworkDispatcher implements AutoCloseable {
  private final SigninNetworkSender sender;
  private final ScheduledThreadPoolExecutor executor;
  private final Semaphore slots;
  private final int attempts;
  private final Duration delay;

  SigninNetworkDispatcher(SigninNetworkSender sender) {
    this(sender, 4, 1024, 6, Duration.ofSeconds(1));
  }

  SigninNetworkDispatcher(
      SigninNetworkSender sender, int workers, int capacity, int attempts, Duration delay) {
    if (workers < 1 || capacity < 1 || attempts < 1 || attempts > 10 || delay.isNegative()) {
      throw new IllegalArgumentException("Invalid sign-in delivery bounds");
    }
    this.sender = sender;
    this.slots = new Semaphore(capacity);
    this.attempts = attempts;
    this.delay = delay;
    executor =
        new ScheduledThreadPoolExecutor(
            workers,
            task -> {
              Thread thread = new Thread(task, "signin-forensics");
              thread.setDaemon(true);
              return thread;
            });
    executor.setExecuteExistingDelayedTasksAfterShutdownPolicy(false);
  }

  boolean submit(SigninNetworkSender.Evidence evidence) {
    if (evidence == null || executor.isShutdown()) return false;
    // Counts queued, delayed and active deliveries together, bounding sensitive memory.
    if (!slots.tryAcquire()) {
      log.warn("Sign-in forensic delivery capacity exhausted; attribution was dropped");
      return false;
    }
    try {
      executor.execute(() -> deliver(evidence, 1));
      return true;
    } catch (RejectedExecutionException closed) {
      slots.release();
      return false;
    }
  }

  private void deliver(SigninNetworkSender.Evidence evidence, int attempt) {
    boolean rescheduled = false;
    try {
      if (sender.deliver(evidence) == SigninNetworkSender.Delivery.RETRY
          && !executor.isShutdown()) {
        if (attempt < attempts && !Thread.currentThread().isInterrupted()) {
          executor.schedule(
              () -> deliver(evidence, attempt + 1),
              delay.multipliedBy(1L << (attempt - 1)).toMillis(),
              TimeUnit.MILLISECONDS);
          rescheduled = true;
        } else {
          log.warn("Sign-in forensic retries exhausted; attribution may be missing");
        }
      }
    } catch (RejectedExecutionException closed) {
      // Shutdown discards volatile records rather than publishing them elsewhere.
    } catch (RuntimeException failure) {
      log.warn("Sign-in forensic delivery could not finish");
    } finally {
      if (!rescheduled) slots.release();
    }
  }

  @Override
  public void close() {
    executor.shutdownNow();
    try {
      if (!executor.awaitTermination(2, TimeUnit.SECONDS)) {
        log.warn("Sign-in forensic workers are still stopping");
      }
    } catch (InterruptedException interrupted) {
      Thread.currentThread().interrupt();
    }
  }
}
