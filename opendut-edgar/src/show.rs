use futures::TryStreamExt;
use netlink_packet_route::link::{LinkMessage};
use opendut_types::util::net::NetworkInterfaceName;
use crate::service::network_interface::manager::NetworkInterfaceManager;
use crate::service::network_interface::manager::can::update_can::UpdateCanInterface;

/*
    #[clap(hide = true)]
    Show(ShowCli),


    Commands::Show(implementation) => {
        implementation.default_handling().await
    }

 */

#[derive(clap::Parser, Debug)]
pub struct ShowCli {
    #[command(subcommand)]
    pub(crate) task: ShowTaskCli,
}

#[derive(clap::Subcommand, Debug)]
pub enum ShowTaskCli {
    /// Show network interfaces
    Interfaces,
    CreateDummy,
    // TODO: remove modifying interface options
    WriteCanParams,
}

impl ShowCli {
    pub(crate) async fn default_handling(self) -> anyhow::Result<()> {
        let (connection, handle, _) = rtnetlink::new_connection().expect("Could not get rtnetlink handle.");
        tokio::spawn(connection);
        let manager = NetworkInterfaceManager { handle };
        
        match self.task {
            ShowTaskCli::Interfaces => {
                let interfaces = manager.list_interfaces().await?;
                println!("{:?}", interfaces);
            }
            ShowTaskCli::WriteCanParams => {
                let can0 = NetworkInterfaceName::try_from("can0")?;
                let can1 = NetworkInterfaceName::try_from("can1")?;
                let can0_interface = manager.find_interface(&can0).await?.expect("Could not find can0 interface.");
                let can1_interface = manager.find_interface(&can1).await?.expect("Could not find can1 interface.");

                let can0_message: LinkMessage = manager.handle
                    .link()
                    .get()
                    .match_index(can0_interface.index)
                    .execute()
                    .try_next().await?.expect("foo");

                println!("CAN0: {:?}", can0_message);
                manager.handle
                    .link()
                    .set(can1_interface.index)
                    .copy_can_parameters(&can0_message)
                    .execute().await?;


            },
            ShowTaskCli::CreateDummy => {
                let name = NetworkInterfaceName::try_from("bridge-dummy")?;
                let interface = manager.create_empty_bridge(&name).await?;
                println!("Bridge interface: {:?}", interface);

            }
        }
        
        Ok(())
    }
    
}
