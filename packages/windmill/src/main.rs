#![allow(non_upper_case_globals)]

use anyhow::Result;
use async_trait::async_trait;
use celery::prelude::*;
use structopt::StructOpt;
use tracing::{event, instrument, Level};

// This generates the task struct and impl with the name set to the function name "add"
#[instrument]
#[celery::task]
fn add(x: i32, y: i32) -> TaskResult<i32> {
    Ok(x + y)
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "celery_app",
    about = "Run a Rust Celery producer or consumer.",
    setting = structopt::clap::AppSettings::ColoredHelp,
)]
enum CeleryOpt {
    Consume,
    Produce {
        #[structopt(possible_values = &["add"])]
        tasks: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = CeleryOpt::from_args();

    let my_app = celery::app!(
        // broker = RedisBroker { std::env::var("REDIS_ADDR").unwrap_or_else(|_| "redis://127.0.0.1:6379/".into()) },
        broker = AMQPBroker { std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rabbitmq:5672".into()) },
        tasks = [
            add,
        ],
        // This just shows how we can route certain tasks to certain queues based
        // on glob matching.
        task_routes = [
            "add" => "test_task",
        ],
        prefetch_count = 2,
        heartbeat = Some(10),
    ).await?;

    match opt {
        CeleryOpt::Consume => {
            my_app.display_pretty().await;
            my_app.consume_from(&["test_task"]).await?;
        }
        CeleryOpt::Produce { tasks } => {
            if tasks.is_empty() {
                event!(Level::INFO, "Task is empty, adding new tasks");
                // Basic task sending.
                let task1 = my_app.send_task(add::new(1, 2)).await?;
                event!(Level::INFO, "Sent task {}", task1.task_id);

                // Sending a task with additional options like `countdown`.
                let task2 = my_app
                    .send_task(add::new(1, 3).with_countdown(3).with_time_limit(20))
                    .await?;
                event!(Level::INFO, "Sent task {}", task2.task_id);
    
            } else {
                for task in tasks {
                    match task.as_str() {
                        "add" => {
                            let task = my_app.send_task(add::new(1, 2)).await?;
                            event!(Level::INFO, "Sent task {}", task.task_id);
                            task
                        },
                        _ => panic!("unknown task"),
                    };
                }
            }
        }
    };

    my_app.close().await?;
    Ok(())
}
