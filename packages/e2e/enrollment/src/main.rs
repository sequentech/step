#[macro_use]
extern crate rocket;

mod routes;    // Tells Rust to look in `src/routes/mod.rs`
mod services;  // Tells Rust to look in `src/services/mod.rs`
mod types;     // Tells Rust to look in `src/types/mod.rs`

use rocket::fairing::AdHoc;
use services::user::load_users;


// Import your routes

#[get("/")]
fn index() -> &'static str {
    "Server is running!"
}

// If you have a CSV load at startup or anything, you can do it in main or a fairing

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    // For example, if you want to load CSV before launching, do it here:
    // load_csv_into_db("voters.csv").expect("Failed to load CSV");

    if let Err(e) = load_users("./voters.csv") {
        eprintln!("Failed to load CSV: {:?}", e);
        // Exit early if CSV loading is critical and you want to fail fast.
        std::process::exit(1);
    }

    let _rocket = rocket::build()
        .mount("/", routes![
            index,
            routes::user::users_list,
        ])
        .launch()
        .await?;

    Ok(())
}
