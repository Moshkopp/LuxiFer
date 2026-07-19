fn main() -> std::io::Result<()> {
    let config = hub::ServerConfig::from_env()?;
    println!(
        "{} {} lauscht auf http://{}",
        studio_core::branding::HUB_NAME,
        env!("CARGO_PKG_VERSION"),
        config.bind
    );
    hub::serve(config)
}
