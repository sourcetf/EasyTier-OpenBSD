use std::{collections::BTreeSet, net::Ipv4Addr};

use super::{
    Error, IfConfiguerTrait, cidr_to_subnet_mask, ignore_removed_shell_cmd_not_found, run_shell_cmd,
};
use async_trait::async_trait;
use cidr::{Ipv4Inet, Ipv6Inet};
use tokio::sync::Mutex;

#[derive(Default)]
pub struct MacIfConfiger {
    configured_ipv4: Mutex<BTreeSet<Ipv4Inet>>,
}

impl MacIfConfiger {
    fn build_add_ipv4_cmd(name: &str, addr: Ipv4Inet, has_configured_ipv4: bool) -> String {
        let address = addr.address();
        if has_configured_ipv4 {
            format!(
                "ifconfig {} alias {:?} {:?} netmask {}",
                name,
                address,
                address,
                cidr_to_subnet_mask(addr.network_length())
            )
        } else {
            format!(
                "ifconfig {} {:?}/{:?} {:?} up",
                name,
                address,
                addr.network_length(),
                address,
            )
        }
    }
}

async fn run_delete_shell_cmd(cmd: &str) -> Result<(), Error> {
    ignore_removed_shell_cmd_not_found(run_shell_cmd(cmd).await)
}

// On OpenBSD, an interface route needs the gateway to be an address assigned to
// the interface (route(8) -iface flag), and does not support -hopcount.
#[cfg(target_os = "openbsd")]
fn get_iface_ipv4_cmd(name: &str) -> String {
    format!("ifconfig {} | awk '/inet /{{print $2; exit}}'", name)
}

#[cfg(target_os = "openbsd")]
fn get_iface_ipv6_cmd(name: &str) -> String {
    format!("ifconfig {} | awk '/inet6 /{{print $2; exit}}'", name)
}

#[async_trait]
impl IfConfiguerTrait for MacIfConfiger {
    async fn add_ipv4_route(
        &self,
        name: &str,
        address: Ipv4Addr,
        cidr_prefix: u8,
        cost: Option<i32>,
    ) -> Result<(), Error> {
        self.add_ipv4_route_with_source_hint(name, address, cidr_prefix, cost, None)
            .await
    }

    async fn add_ipv4_route_with_source_hint(
        &self,
        name: &str,
        address: Ipv4Addr,
        cidr_prefix: u8,
        cost: Option<i32>,
        source_hint: Option<Ipv4Addr>,
    ) -> Result<(), Error> {
        #[cfg(target_os = "openbsd")]
        let cmd = format!(
            "route -n add {}/{} $( {} ) -iface",
            address,
            cidr_prefix,
            get_iface_ipv4_cmd(name)
        );
        #[cfg(not(target_os = "openbsd"))]
        let cmd = {
            let source_hint = source_hint
                .map(|source| format!(" -ifa {}", source))
                .unwrap_or_default();
            format!(
                "route -n add {} -netmask {} -interface {}{} -hopcount {}",
                address,
                cidr_to_subnet_mask(cidr_prefix),
                name,
                source_hint,
                cost.unwrap_or(7)
            )
        };
        let _ = (cost, source_hint);
        run_shell_cmd(cmd.as_str()).await
    }

    async fn remove_ipv4_route(
        &self,
        name: &str,
        address: Ipv4Addr,
        cidr_prefix: u8,
    ) -> Result<(), Error> {
        #[cfg(target_os = "openbsd")]
        let cmd = format!("route -n delete {}/{}", address, cidr_prefix);
        #[cfg(not(target_os = "openbsd"))]
        let cmd = format!(
            "route -n delete {} -netmask {} -interface {}",
            address,
            cidr_to_subnet_mask(cidr_prefix),
            name
        );
        let _ = name;
        run_delete_shell_cmd(cmd.as_str()).await
    }

    async fn add_ipv4_ip(
        &self,
        name: &str,
        address: Ipv4Addr,
        cidr_prefix: u8,
    ) -> Result<(), Error> {
        let addr = Ipv4Inet::new(address, cidr_prefix).map_err(|err| {
            anyhow::anyhow!("invalid IPv4 address {address}/{cidr_prefix}: {err:?}")
        })?;
        let mut configured_ipv4 = self.configured_ipv4.lock().await;
        let cmd = Self::build_add_ipv4_cmd(name, addr, !configured_ipv4.is_empty());

        run_shell_cmd(cmd.as_str()).await?;
        configured_ipv4.insert(addr);
        Ok(())
    }

    async fn set_link_status(&self, name: &str, up: bool) -> Result<(), Error> {
        run_shell_cmd(format!("ifconfig {} {}", name, if up { "up" } else { "down" }).as_str())
            .await
    }

    async fn remove_ip(&self, name: &str, ip: Option<Ipv4Inet>) -> Result<(), Error> {
        let mut configured_ipv4 = self.configured_ipv4.lock().await;
        if let Some(ip) = ip {
            run_delete_shell_cmd(
                format!("ifconfig {} inet {} delete", name, ip.address()).as_str(),
            )
            .await?;
            configured_ipv4.remove(&ip);
        } else {
            run_delete_shell_cmd(format!("ifconfig {} inet delete", name).as_str()).await?;
            configured_ipv4.clear();
        }
        Ok(())
    }

    async fn set_mtu(&self, name: &str, mtu: u32) -> Result<(), Error> {
        run_shell_cmd(format!("ifconfig {} mtu {}", name, mtu).as_str()).await
    }

    async fn add_ipv6_ip(
        &self,
        name: &str,
        address: std::net::Ipv6Addr,
        cidr_prefix: u8,
    ) -> Result<(), Error> {
        #[cfg(target_os = "openbsd")]
        let cmd = format!("ifconfig {} inet6 {}/{} alias", name, address, cidr_prefix);
        #[cfg(not(target_os = "openbsd"))]
        let cmd = format!("ifconfig {} inet6 {}/{} add", name, address, cidr_prefix);
        run_shell_cmd(cmd.as_str()).await
    }

    async fn remove_ipv6(&self, name: &str, ip: Option<Ipv6Inet>) -> Result<(), Error> {
        if let Some(ip) = ip {
            run_delete_shell_cmd(
                format!("ifconfig {} inet6 {} delete", name, ip.address()).as_str(),
            )
            .await
        } else {
            // Remove all IPv6 addresses is more complex on macOS, just succeed
            Ok(())
        }
    }

    async fn add_ipv6_route(
        &self,
        name: &str,
        address: std::net::Ipv6Addr,
        cidr_prefix: u8,
        cost: Option<i32>,
    ) -> Result<(), Error> {
        #[cfg(target_os = "openbsd")]
        let cmd = format!(
            "route -n add -inet6 {}/{} $( {} ) -iface",
            address,
            cidr_prefix,
            get_iface_ipv6_cmd(name)
        );
        #[cfg(not(target_os = "openbsd"))]
        let cmd = if let Some(cost) = cost {
            format!(
                "route -n add -inet6 {}/{} -interface {} -hopcount {}",
                address, cidr_prefix, name, cost
            )
        } else {
            format!(
                "route -n add -inet6 {}/{} -interface {}",
                address, cidr_prefix, name
            )
        };
        let _ = cost;
        run_shell_cmd(cmd.as_str()).await
    }

    async fn remove_ipv6_route(
        &self,
        name: &str,
        address: std::net::Ipv6Addr,
        cidr_prefix: u8,
    ) -> Result<(), Error> {
        #[cfg(target_os = "openbsd")]
        let cmd = format!("route -n delete -inet6 {}/{}", address, cidr_prefix);
        #[cfg(not(target_os = "openbsd"))]
        let cmd = format!(
            "route -n delete -inet6 {}/{} -interface {}",
            address, cidr_prefix, name
        );
        let _ = name;
        run_delete_shell_cmd(cmd.as_str()).await
    }
}
