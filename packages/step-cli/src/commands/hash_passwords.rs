use clap::Args;
use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::num::NonZeroU32;
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use rand::Rng;
use rand::thread_rng;
use ring::{digest,pbkdf2};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};


const CREDENTIAL_LEN: usize = digest::SHA256_OUTPUT_LEN;
pub type Credential = [u8; CREDENTIAL_LEN];
static PBKDF2_ALGORITHM: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA256;


#[derive(Args)]
#[command(about = "Process a CSV file to hash passwords and generate salts")]
pub struct HashPasswords {
    #[arg(long)]
    input_file: String,

    #[arg(long)]
    output_file: String,

    #[arg(long)]
    iterations: NonZeroU32,
}

impl HashPasswords {

    pub fn run(&self) {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        match runtime.block_on(self.run_hash_password()) {
            Ok(_) => println!("Successfully generated hashed passwords"),
            Err(err) => eprintln!("Error! Failed to generate hashed password: {err:?}"),
        }
    }
    
    pub async fn run_hash_password(&self) -> Result<(), Box<dyn std::error::Error>> {

        let input = File::open(&self.input_file)?;
        let mut rdr = ReaderBuilder::new().from_reader(BufReader::new(input));


        let output = File::create(&self.output_file)?;
        let mut wtr = WriterBuilder::new().from_writer(BufWriter::new(output));

        let original_headers = rdr.headers()?.clone();
        let password_index = original_headers.iter()
            .position(|h| h == "password")
            .ok_or_else(|| anyhow::anyhow!("CSV does not contain a 'password' column"))?;

        let mut new_headers = StringRecord::new();
        for (i, header) in original_headers.iter().enumerate() {
            if i != password_index {
                    new_headers.push_field(header);
                }
        }
        new_headers.push_field("password_salt");
        new_headers.push_field("hashed_password");
        new_headers.push_field("num_of_iterations");

            
        wtr.write_record(&new_headers)?;


        for result in rdr.records() {
            let record = result?;
            let mut new_record = record.clone();

            let password = record.get(password_index).unwrap_or("");
            print!("password: {}", password);

            let mut salt_bytes: Credential = Default::default();
            thread_rng().fill(&mut salt_bytes);
            let password_salt = BASE64_STANDARD.encode(salt_bytes);

            let hashed_password = hash_password(&password.to_string(), &salt_bytes, &self.iterations)?;

            let new_fields: Vec<&str> = record.iter()
            .enumerate()
            .filter(|(i, _)| *i != password_index)
            .map(|(_, field)| field)
            .collect();
            let mut new_record = StringRecord::from(new_fields);

            new_record.push_field(&password_salt);
            new_record.push_field(&hashed_password);
            new_record.push_field(&self.iterations.to_string());

            wtr.write_record(&new_record)?;
        }

        wtr.flush()?;
        Ok(())
    }
}

fn hash_password(password: &String, salt: &[u8], iterations: &NonZeroU32) -> Result<String> {
    let mut output: Credential = [0u8; CREDENTIAL_LEN];
    pbkdf2::derive(
        PBKDF2_ALGORITHM,
        *iterations,
        salt,
        password.as_bytes(),
        &mut output,
    );

    let generated_hash = BASE64_STANDARD.encode(&output);
    Ok(generated_hash)
}
