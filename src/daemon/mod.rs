use log::info;
use std::error::Error;
use zbus::connection::Builder;

mod dbus_iface;

pub async fn run() -> Result<(), Box<dyn Error>> {
    info!("Starting lapctld daemon...");
    unsafe {
        std::env::set_var("LAPCTL_DAEMON_INTERNAL", "1");
    }
    let _connection = Builder::system()?
        .name("org.lapctl")?
        .serve_at("/org/lapctl", dbus_iface::LapctlInterface::default())?
        .build()
        .await?;

    info!("Daemon is running on the system bus.");
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
}
