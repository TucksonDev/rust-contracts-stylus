use benches::{access_control, erc20};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let _ = tokio::join!(erc20::bench(), access_control::bench());

    Ok(())
}