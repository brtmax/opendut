use netlink_packet_route::link::{InfoData, LinkAttribute, LinkInfo, LinkMessage};
use rtnetlink::LinkSetRequest;

pub trait UpdateCanInterface {
    /// Copy CAN interface parameters from another device, requires CAN interface to be 'down'
    fn copy_can_parameters(self, source: &LinkMessage) -> Self;
    fn write_can_parameters(self, can_link_info_data: Vec<u8>) -> Self;

}

impl UpdateCanInterface for LinkSetRequest {
    fn copy_can_parameters(mut self, source: &LinkMessage) -> Self {
        let info_data_source = source.attributes
            .iter()
            .find_map(|attribute| {
                if let LinkAttribute::LinkInfo(info) = attribute {
                    info.iter().find_map(|link_info| {
                        if let LinkInfo::Data(info_data) = link_info {
                            if let InfoData::Other(_) = info_data {
                                Some(info_data.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            }).expect("No info data found on source!");

        let can_parameters = LinkAttribute::LinkInfo(Vec::from([LinkInfo::Data(info_data_source.clone())]));
        eprintln!("Writing CAN parameters: {:?}", can_parameters);

        self.message_mut().attributes.push(can_parameters);

        self
    }

    fn write_can_parameters(mut self, can_link_info_data: Vec<u8>) -> Self {
        let can_parameters = LinkAttribute::LinkInfo(Vec::from([LinkInfo::Data(InfoData::Other(can_link_info_data))]));
        self.message_mut().attributes.push(can_parameters);
        self
    }
}