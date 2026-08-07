use std::{env, net::{TcpListener, TcpStream}};

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let env = env::var("WAYFLEET_SERVER").context("coudln't get the 'WAYFLEET_SERVER' env variable or it contains non utf8 characters")?;

    // let tcp

    Ok(())
}