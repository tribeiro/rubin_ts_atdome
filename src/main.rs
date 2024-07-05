use rubin_ts_atdome::mock_controller::mock_controller::run_mock_controller;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    run_mock_controller(8887).await?;

    Ok(())
}
