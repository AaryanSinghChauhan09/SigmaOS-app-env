//! SigmaOS component binary: app-env

use sigmaos_app-env::Component;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Component::new().operate()?;
    println!("sigmaos-app-env: ok");
    Ok(())
}
