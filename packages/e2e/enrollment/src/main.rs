mod test;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ElectionApp", about = "An application to manage elections")]
struct Args {
    #[arg(short, long)]
    election_event_id: String,

    #[arg(short, long)]
    voters_count: u64,

    #[arg(short, long)]
    otp_code: String,
}

fn main() {
    // Parse the command-line arguments
    let args = Args::parse();

    // Call the function with the election_event_id
    match test::test::run_enrollment_test(
        &args.election_event_id,
        args.voters_count,
        &args.otp_code,
    ) {
        Ok(_) => println!("Test ran successfully."),
        Err(e) => eprintln!("Error running test: {}", e),
    }
}
