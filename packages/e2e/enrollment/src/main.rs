#[macro_use]
extern crate rocket;

mod routes;    // Tells Rust to look in `src/routes/mod.rs`
mod services;  // Tells Rust to look in `src/services/mod.rs`
mod types;     // Tells Rust to look in `src/types/mod.rs`

use rocket::fairing::AdHoc;


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

    let _rocket = rocket::build()
        .mount("/", routes![
            index,
            routes::user::users_list,
        ])
        .attach(AdHoc::on_liftoff("Liftoff Message", |_| {
            Box::pin(async move {
                println!("Rocket has launched!");
            })
        }))
        .launch()
        .await?;

    Ok(())
}
