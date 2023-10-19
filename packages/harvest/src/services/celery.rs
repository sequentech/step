// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use celery;
use celery::error::CeleryError;
use celery::export::Arc;
use celery::Celery;
use windmill::tasks::set_public_key::set_public_key_task;

pub async fn create_app() -> Result<Arc<Celery>, CeleryError> {
    celery::app!(
        // broker = RedisBroker { std::env::var("REDIS_ADDR").unwrap_or_else(|_| "redis://127.0.0.1:6379/".into()) },
        broker = AMQPBroker { std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rabbitmq:5672".into()) },
        tasks = [
            set_public_key_task,
        ],
        // Route certain tasks to certain queues based on glob matching.
        task_routes = [
            "set_public_key_task" => "short_queue",
        ],
        prefetch_count = 2,
        heartbeat = Some(10),
    ).await
}
