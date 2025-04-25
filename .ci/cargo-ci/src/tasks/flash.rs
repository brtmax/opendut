//! Flash an SD card with a modified image for the Raspberry Pi to allow for easier bootstrapping.
//!
//! The modifications don't influence how openDuT is executed.

use anyhow::Context;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;
use crate::core::util::RunRequiringSuccess;

///Flash a device with an operating system image
#[derive(clap::Parser)]
pub struct FlashCli {
    /// The kind of device to flash
    #[command(subcommand)]
    device: DeviceKind,
}

#[derive(clap::Subcommand)]
enum DeviceKind {
    /// Flash an SD card for a Raspberry Pi
    #[command(alias="rpi", alias="raspi")]
    RaspberryPi {
        /// The storage device to flash, e.g. /dev/mmcblk0
        #[arg(long)]
        storage: String,
        /// The hostname to set on the Raspberry Pi
        #[arg(long)]
        hostname: String,
        /// File to use as the ~/.ssh/authorized_keys file for SSH login
        #[arg(long)]
        authorized_keys_file: PathBuf,
        #[arg(long, default_value = "192.168.0.42/24")]
        ip_address: String,
        #[arg(long)]
        wifi_ssid: Option<String>,
    },
}

const OS_LIST_URL: &str = "https://downloads.raspberrypi.com/os_list_imagingutility_v4.json";
const OS_LIST_ENTRY_OTHER: &str = "Raspberry Pi OS (other)";
const IMAGE_NAME: &str = "Raspberry Pi OS Lite (64-bit)";


impl FlashCli {
    #[tracing::instrument(name="flash", skip(self))]
    pub fn default_handling(&self) -> crate::Result {
        match &self.device {
            DeviceKind::RaspberryPi { storage: storage_to_flash, hostname, authorized_keys_file, ip_address, wifi_ssid } => {

                let image = Self::determine_image()?;
                let wifi_config = wifi_ssid.as_ref().map(|wifi_ssid| WifiConfig { ssid: wifi_ssid.to_string() });

                let first_run_script_file = std::env::temp_dir().join("opendut_raspberry_pi_flash_firstrun.sh");
                Self::template_first_run_script(&first_run_script_file, hostname.clone(), authorized_keys_file, ip_address, wifi_config)?;

                Self::run_rpi_imager(&image, storage_to_flash, &first_run_script_file)?;

                eprintln!();
                eprintln!("You can now connect an Ethernet cable to your Raspberry Pi and log into it by running:");
                eprintln!("  ssh pi@{ip_address}");
            }
        }

        Ok(())
    }

    fn determine_image() -> anyhow::Result<String> {
        let os_list: OsListJson = reqwest::blocking::get(OS_LIST_URL)?
            .json()?;

        let other = os_list.os_list.into_iter()
            .find(|entry| entry.name == OS_LIST_ENTRY_OTHER)
            .context(format!("List of operating system images does not contain entry with name '{OS_LIST_ENTRY_OTHER}'"))?;

        let image = other.subitems
            .context("Entry for other operating systems contains no subitems.")?
            .into_iter()
            .find(|entry| entry.name == IMAGE_NAME)
            .context(format!("List of operating system images does not contain entry with desired image name '{IMAGE_NAME}'"))?;

        let image = image.url
            .context("Entry for desired image does not contain a download URL.")?;

        Ok(image)
    }

    fn template_first_run_script(first_run_path: &Path, hostname: String, authorized_keys_file: &Path, ip_address: &str, wifi: Option<WifiConfig>) -> anyhow::Result<()> {
        let authorized_keys = fs::read_to_string(authorized_keys_file)
            .context("Error while reading specified authorized_keys file")?;
        
        let mut file_content = FirstRunScriptTemplate::create();
        file_content = file_content
            .hostname(hostname)
            .ssh(authorized_keys)
            .key_map_config()
            .ethernet_static_config(ip_address.to_string());
        if let Some(wifi) = wifi {
            file_content = file_content.wireless_config_without_passphrase(&wifi);
        }
        let file_content = file_content
            .add_rpi_info_script()
            .into_string();

        fs::write(first_run_path, file_content)?;
        debug!("Rpi-imager first run script was written to: {:?}", first_run_path);
        Ok(())
    }

    fn run_rpi_imager(image: &str, storage_to_flash: &str, first_run_script_file: &Path) -> crate::Result {
        let mut command = Command::new("rpi-imager");
        command.arg("--cli")
            .arg("--first-run-script").arg(first_run_script_file)
            .arg(image)
            .arg(storage_to_flash);
        debug!("rpi-command: {:?}", command);
        command.run_requiring_success()?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct OsListJson {
    os_list: Vec<OsListEntry>,
}
#[derive(Debug, Deserialize)]
struct OsListEntry {
    name: String,
    subitems: Option<Vec<OsListEntrySubitems>>,
}

#[derive(Debug, Deserialize)]
struct OsListEntrySubitems {
    name: String,
    url: Option<String>,
}

#[derive(Clone)]
struct WifiConfig {
    pub ssid: String,
}

struct FirstRunScriptTemplate {
    script: String,
}
impl FirstRunScriptTemplate {
    fn create() -> Self {
        // This script was extracted by running the graphical Raspberry Pi Imager with
        // adjustments made to the Advanced Options. After flashing to an SD card,
        // the `firstrun.sh` is found top-level in bootfs-partition.
        //
        // In the Advanced Options:
        // * "Set hostname" was enabled, with a dummy value for the hostname.
        // * "Enable SSH" was enabled and set to "Allow public-key authentication only", with a dummy value for the authorized_keys.

        let script_header = r#"
#!/bin/bash

set +e

FIRSTUSER=`getent passwd 1000 | cut -d: -f1`
FIRSTUSERHOME=`getent passwd 1000 | cut -d: -f6`

"#.to_string();
        Self { script: script_header }
    }

    fn hostname(mut self, hostname: String) -> Self {
        let hostname_script = format!(r#"
# hostname configuration script
CURRENT_HOSTNAME=`cat /etc/hostname | tr -d " \t\n\r"`
if [ -f /usr/lib/raspberrypi-sys-mods/imager_custom ]; then
   /usr/lib/raspberrypi-sys-mods/imager_custom set_hostname {hostname}
else
   echo {hostname} >/etc/hostname
   sed -i "s/127.0.1.1.*$CURRENT_HOSTNAME/127.0.1.1\t{hostname}/g" /etc/hosts
fi
"#);
        self.script.push_str(&hostname_script);
        self
    }

    fn ssh(mut self, authorized_keys: String) -> Self {
        // authorized keys are debug print because there should not be a line break (this also adds double quotes)
        let ssh_script = format!(r#"
# Add authorized keys for remote access
install -o "$FIRSTUSER" -m 700 -d "$FIRSTUSERHOME/.ssh"
install -o "$FIRSTUSER" -m 600 <(printf {authorized_keys:?}) "$FIRSTUSERHOME/.ssh/authorized_keys"
echo 'PasswordAuthentication no' >>/etc/ssh/sshd_config
systemctl enable ssh
"#);
        self.script.push_str(&ssh_script);
        self
    }

    fn key_map_config(mut self) -> Self {
        let key_map_script = r#"
if [ -f /usr/lib/raspberrypi-sys-mods/imager_custom ]; then
   /usr/lib/raspberrypi-sys-mods/imager_custom set_keymap 'de'
   /usr/lib/raspberrypi-sys-mods/imager_custom set_timezone 'Europe/Berlin'
else
   rm -f /etc/localtime
   echo "Europe/Berlin" >/etc/timezone
   dpkg-reconfigure -f noninteractive tzdata
cat >/etc/default/keyboard <<'KBEOF'
XKBMODEL="pc105"
XKBLAYOUT="de"
XKBVARIANT=""
XKBOPTIONS=""

KBEOF
   dpkg-reconfigure -f noninteractive keyboard-configuration
fi
"#.to_string();
        self.script.push_str(&key_map_script);
        self
    }

    fn ethernet_static_config(mut self, ipv4net: String) -> Self {
        let ethernet_script = format!(r#"
# Ethernet static IP configuration script
cat >/etc/NetworkManager/system-connections/static-eth0.nmconnection <<'EOF'
[connection]
id=static-eth0
uuid=8b82ba44-1f60-4362-8aa9-9a7733c988c2
type=ethernet
interface-name=eth0

[ethernet]

[ipv4]
address1={ipv4net}
method=manual

[ipv6]
addr-gen-mode=default
method=auto

[proxy]

EOF

chmod 600 /etc/NetworkManager/system-connections/static-eth0.nmconnection
nmcli con up static-eth0
"#);
        self.script.push_str(&ethernet_script);
        self
    }

    fn wireless_config_without_passphrase(mut self, wifi: &WifiConfig) -> Self {
        let ssid = wifi.ssid.clone();
        let wifi_script = format!(r#"
# Wireless configuration without a passphrase
raspi-config nonint do_wifi_country DE

cat >/etc/NetworkManager/system-connections/{ssid}.nmconnection <<'EOF'
[connection]
id={ssid}
uuid=6c521f09-9c19-4584-8b47-d297f3a7fd40
type=wifi
interface-name=wlan0

[wifi]
mode=infrastructure
ssid={ssid}

[ipv4]
method=auto

[ipv6]
addr-gen-mode=default
method=auto

[proxy]

EOF

chmod 600 /etc/NetworkManager/system-connections/{ssid}.nmconnection
nmcli con up {ssid}
"#);
        self.script.push_str(&wifi_script);
        self
    }

    fn into_string(mut self) -> String {
        self.script.push_str(r#"

rm -f /boot/firstrun.sh
sed -i 's| systemd.run.*||g' /boot/cmdline.txt
exit 0

"#);
        self.script
    }

    fn add_rpi_info_script(mut self) -> Self {
        let key_map_script = r#"
cat >/usr/bin/rpi-info <<'EOF'
#!/usr/bin/env python3
import json
import os
import subprocess

ip_link = subprocess.run("ip -json address".split(), stdout=subprocess.PIPE)
ip_link_json = json.loads(ip_link.stdout.decode())
mac_addresses = {item["ifname"]: item["address"] for item in ip_link_json if item["ifname"] != "lo"}
ip_addresses = {item["ifname"]: item["addr_info"][0]["local"] for item in ip_link_json if item["ifname"] != "lo" and len(item["addr_info"]) > 0}

serial_file="/sys/firmware/devicetree/base/serial-number"
model_file="/sys/firmware/devicetree/base/model"

def read_file(path):
    if os.path.exists(path):
        with open(path) as _file:
            return _file.read().strip().replace("\u0000", "")

serial = read_file(serial_file)
model = read_file(model_file)
hostname = read_file("/etc/hostname")


rpi_info = {
    "hostname": hostname,
    "serial": serial,
    "model": model,
    "macs": mac_addresses,
    "ips": ip_addresses,
}

print(json.dumps(rpi_info, indent=2))
EOF
chmod a+x /usr/bin/rpi-info
"#.to_string();
        self.script.push_str(&key_map_script);
        self
    }
    
}

#[cfg(test)]
mod tests {
    use crate::tasks::flash::{FirstRunScriptTemplate, WifiConfig};

    #[test]
    fn test_create_firstrun_script() {
        let hostname = "bootstrap".to_string();
        let ip_address = "192.168.0.42/24".to_string();
        let authorized_keys = "none".to_string();

        let mut file_content = FirstRunScriptTemplate::create();
        file_content = file_content
            .hostname(hostname)
            .ssh(authorized_keys)
            .ethernet_static_config(ip_address.clone())
            .wireless_config_without_passphrase(&WifiConfig { ssid: "test".to_string() });
        let file_content = file_content.into_string();

        assert!(file_content.contains(&ip_address));
        std::fs::write("/tmp/test", file_content).unwrap();
    }
}