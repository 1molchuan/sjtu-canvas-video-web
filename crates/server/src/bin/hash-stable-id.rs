use std::{
    error::Error,
    io::{self, IsTerminal, Read},
};

use server::auth::whitelist::hash_stable_id;

const MAX_STABLE_ID_BYTES: u64 = 4_096;

fn main() -> Result<(), Box<dyn Error>> {
    if io::stdin().is_terminal() {
        return Err(io::Error::other(
            "refusing echoed input; pipe a hidden prompt into this command",
        )
        .into());
    }
    let mut input = String::new();
    io::stdin()
        .take(MAX_STABLE_ID_BYTES + 1)
        .read_to_string(&mut input)?;
    if input.len() as u64 > MAX_STABLE_ID_BYTES {
        return Err(io::Error::other("stable identifier input is too long").into());
    }
    println!("{}", hash_stable_id(&input)?);
    Ok(())
}
