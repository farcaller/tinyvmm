use zbus::{dbus_proxy, Result};

#[dbus_proxy(
    interface = "org.freedesktop.network1.Manager",
    default_service = "org.freedesktop.network1",
    default_path = "/org/freedesktop/network1"
)]
pub trait Networkd {
    async fn reload(&self) -> Result<()>;
    async fn get_link_by_name(&self, name: &str) -> Result<(i32, zvariant::OwnedObjectPath)>;
    async fn describe_link(&self, ifindex: i32) -> Result<String>;
}
