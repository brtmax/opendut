//! Flash an SD card with a modified image for the Raspberry Pi to allow for easier bootstrapping.
//!
//! The modifications don't influence how openDuT is executed.

use anyhow::Context;
use serde::Deserialize;
use std::fs;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
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
    },
}

const OS_LIST_URL: &str = "https://downloads.raspberrypi.com/os_list_imagingutility_v4.json";
const OS_LIST_ENTRY_OTHER: &str = "Raspberry Pi OS (other)";
const IMAGE_NAME: &str = "Raspberry Pi OS Lite (64-bit)";


impl FlashCli {
    #[tracing::instrument(name="flash", skip(self))]
    pub fn default_handling(&self) -> crate::Result {
        match &self.device {
            DeviceKind::RaspberryPi { storage: storage_to_flash, hostname, authorized_keys_file } => {

                let image = Self::determine_image()?;

                let first_run_script_file = std::env::temp_dir().join("opendut_raspberry_pi_flash_firstrun.sh");
                let ssh_address = Ipv4Addr::from_str("192.168.111.111")?;
                Self::template_first_run_script(&first_run_script_file, hostname.clone(), authorized_keys_file, ssh_address)?;

                Self::run_rpi_imager(&image, storage_to_flash, &first_run_script_file)?;

                eprintln!();
                eprintln!("You can now connect an Ethernet cable to your Raspberry Pi and log into it by running:");
                eprintln!("  sudo ip address add 192.168.111.222/24 dev eth0");
                eprintln!("  sudo ip link set eth0 up");
                eprintln!("  ssh pi@{ssh_address}");
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

    fn template_first_run_script(first_run_path: &Path, hostname: String, authorized_keys_file: &Path, ssh_address: Ipv4Addr) -> anyhow::Result<()> {
        let authorized_keys = fs::read_to_string(authorized_keys_file)
            .context("Error while reading specified authorized_keys file")?;

        let file_content = FirstRunScriptTemplate {
            hostname,
            authorized_keys,
            //FIXME this is not persistent
            extra_commands: format!(r#"
ip address add {ssh_address}/24 eth0
"#),
        }.into_string();

        fs::write(first_run_path, file_content)?;
        Ok(())
    }

    fn run_rpi_imager(image: &str, storage_to_flash: &str, first_run_script_file: &Path) -> crate::Result {
        Command::new("rpi-imager")
            .arg("--cli")
            .arg("--first-run-script").arg(first_run_script_file)
            .arg(image)
            .arg(storage_to_flash)
            .run_requiring_success()?;

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

struct FirstRunScriptTemplate {
    hostname: String,
    authorized_keys: String,
    extra_commands: String,
}
impl FirstRunScriptTemplate {
    fn into_string(self) -> String {
        let FirstRunScriptTemplate { hostname, authorized_keys, extra_commands } = self;

        // This script was extracted by running the graphical Raspberry Pi Imager with
        // adjustments made to the Advanced Options. After flashing to an SD card,
        // the `firstrun.sh` is found top-level in bootfs-partition.
        //
        // In the Advanced Options:
        // * "Set hostname" was enabled, with a dummy value for the hostname.
        // * "Enable SSH" was enabled and set to "Allow public-key authentication only", with a dummy value for the authorized_keys.
        format!(r#"
#!/bin/bash

set +e

CURRENT_HOSTNAME=`cat /etc/hostname | tr -d " \t\n\r"`
echo {hostname} >/etc/hostname
sed -i "s/127.0.1.1.*$CURRENT_HOSTNAME/127.0.1.1\t{hostname}/g" /etc/hosts
FIRSTUSER=`getent passwd 1000 | cut -d: -f1`
FIRSTUSERHOME=`getent passwd 1000 | cut -d: -f6`
install -o "$FIRSTUSER" -m 700 -d "$FIRSTUSERHOME/.ssh"
install -o "$FIRSTUSER" -m 600 <(printf "{authorized_keys}") "$FIRSTUSERHOME/.ssh/authorized_keys"
echo 'PasswordAuthentication no' >>/etc/ssh/sshd_config
systemctl enable ssh
{extra_commands}
rm -f /boot/firstrun.sh
sed -i 's| systemd.run.*||g' /boot/cmdline.txt
exit 0
"#)
    }
}

