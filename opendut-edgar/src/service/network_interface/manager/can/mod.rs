use netlink_packet_utils::byteorder::{ByteOrder, NativeEndian};

// mod can;  // TODO: can support for rtnetlink
pub mod update_can;

// https://github.com/iproute2/iproute2/blob/866e1d107b7de68ca1fcd1d4d5ffecf9d96bff30/include/uapi/linux/can/netlink.h#L122
#[derive(Debug)]
pub struct BitTiming {
    bitrate: u32,
    sample_point: u32,
    tq: u32,
    prop_seq: u32,
    phase_seg1: u32,
    phase_seg2: u32,
    sjw: u32,
    brp: u32,
}

impl BitTiming {
    pub fn create(buffer: &[u8]) -> BitTiming {
        assert_eq!(buffer.len(), 8 * 4);

        let bitrate = NativeEndian::read_u32(&buffer[0..4]);
        let sample_point = NativeEndian::read_u32(&buffer[4..8]);
        let tq = NativeEndian::read_u32(&buffer[8..12]);
        let prop_seq = NativeEndian::read_u32(&buffer[12..16]);
        let phase_seg1 = NativeEndian::read_u32(&buffer[16..20]);
        let phase_seg2 = NativeEndian::read_u32(&buffer[20..24]);
        let sjw = NativeEndian::read_u32(&buffer[24..28]);
        let brp = NativeEndian::read_u32(&buffer[28..32]);
            
        BitTiming {
            bitrate,
            sample_point,
            tq,
            prop_seq,
            phase_seg1,
            phase_seg2,
            sjw,
            brp,
        }
    }
}

#[derive(Debug)]
pub struct BitTimingConst {
    name: String,
    tseg1_min: u32,
    tseg1_max: u32,
    tseg2_min: u32,
    tseg2_max: u32,
    sjw_min: u32,  // implicitly '1'
    sjw_max: u32,
    brp_min: u32,
    brp_max: u32,
    brp_inc: u32,
}

impl BitTimingConst {
    pub fn create(buffer: &[u8]) -> BitTimingConst {
        assert_eq!(buffer.len(), 16 + 9 * 4);  // 52 bytes

        let name: String = String::from_utf8_lossy(&buffer[0..16]).parse().unwrap();
        let name = name.replace('\0', "");
        let tseg1_min = NativeEndian::read_u32(&buffer[16..20]);
        let tseg1_max = NativeEndian::read_u32(&buffer[20..24]);
        let tseg2_min = NativeEndian::read_u32(&buffer[24..28]);
        let tseg2_max = NativeEndian::read_u32(&buffer[28..32]);
        let sjw_min   = 1;
        let sjw_max   = NativeEndian::read_u32(&buffer[32..36]);
        let brp_min   = NativeEndian::read_u32(&buffer[36..40]);
        let brp_max   = NativeEndian::read_u32(&buffer[40..44]);
        let brp_inc   = NativeEndian::read_u32(&buffer[44..48]);

        Self {
            name,
            tseg1_min,
            tseg1_max,
            tseg2_min,
            tseg2_max,
            sjw_min,
            sjw_max,
            brp_min,
            brp_max,
            brp_inc,
        } 
        
    }
}

#[derive(Debug, Clone)]
pub struct CtrlMode {
    mask: u32,
    flags: CtrlModeFlags,
}

#[derive(Debug, Clone)]
pub struct CtrlModeFlags(u32);

bitflags::bitflags! {
    impl CtrlModeFlags: u32 {
        const CAN_CTRLMODE_LOOPBACK = 0x01;           /* Loopback mode */
        const CAN_CTRLMODE_LISTENONLY = 0x02;	      /* Listen-only mode */
        const CAN_CTRLMODE_3_SAMPLES = 0x04;          /* Triple sampling mode */
        const CAN_CTRLMODE_ONE_SHOT = 0x08;           /* One-Shot mode */
        const CAN_CTRLMODE_BERR_REPORTING = 0x10;	  /* Bus-error reporting */
        const CAN_CTRLMODE_FD = 0x20;	              /* CAN FD mode */
        const CAN_CTRLMODE_PRESUME_ACK = 0x40;        /* Ignore missing CAN ACKs */
        const CAN_CTRLMODE_FD_NON_ISO = 0x80;	      /* CAN FD in non-ISO mode */
        const CAN_CTRLMODE_CC_LEN8_DLC = 0x100;       /* Classic CAN DLC option */
        const CAN_CTRLMODE_TDC_AUTO = 0x200;          /* CAN transiver automatically calculates TDCV */
        const CAN_CTRLMODE_TDC_MANUAL = 0x400;        /* TDCV is manually set up by user */
    }
}

impl CtrlMode {
    pub fn create(buffer: &[u8]) -> Self {
        assert_eq!(buffer.len(), 8);
        let mask = NativeEndian::read_u32(&buffer[0..4]);
        let flags = NativeEndian::read_u32(&buffer[4..8]);
        let flags = CtrlModeFlags(flags);
        Self { mask, flags }
    }
}

#[derive(Debug)]
pub(crate) struct CanLinkInfoData {
    // IFLA_CAN_UNSPEC
    unspec: u32,
    // IFLA_CAN_BITTIMING
    bit_timing: BitTiming,
    // IFLA_CAN_BITTIMING_CONST
    bit_timing_const: BitTimingConst,
    // IFLA_CAN_CLOCK
    clock: u32,
    // IFLA_CAN_STATE enum
    state: u32,
    // IFLA_CAN_CTRLMODE
    ctrl_mode: CtrlMode,  // position 100-108

    // IFLA_CAN_RESTART_MS,
    restart_ms: u32,  // position 120-124
    // IFLA_CAN_DATA_BITTIMING
    data_bit_timing: BitTiming,
    // IFLA_CAN_DATA_BITTIMING_CONST
    //data_bit_timing_const: BitTimingConst,
    
    /*
    Restart,                // IFLA_CAN_RESTART
    BerrCounter,            // IFLA_CAN_BERR_COUNTER,
    Termination,            // IFLA_CAN_TERMINATION,
    TerminationConst,       // IFLA_CAN_TERMINATION_CONST,
    BitrateConst,           // IFLA_CAN_BITRATE_CONST,
    BitrateConstConst,      // IFLA_CAN_DATA_BITRATE_CONST,
    BitrateMax,            // IFLA_CAN_BITRATE_MAX,
    TDC,                    // IFLA_CAN_TDC,
    CtrlmodeExt,            // IFLA_CAN_CTRLMODE_EXT,
    */
}

impl CanLinkInfoData {
    fn create(buffer: &[u8]) -> Self {
        let unspec = NativeEndian::read_u32(&buffer[0..4]);
        let bit_timing = BitTiming::create(&buffer[4..36]);

        let bit_timing_const = BitTimingConst::create(&buffer[40..92]);
        let clock = NativeEndian::read_u32(&buffer[92..96]);
        let state = NativeEndian::read_u32(&buffer[96..100]);
        let ctrl_mode = CtrlMode::create(&buffer[108..116]);
        
        let restart_ms = NativeEndian::read_u32(&buffer[120..124]);
        
        let data_bit_timing = BitTiming::create(&buffer[136..136 + 32]);
        //let data_bit_timing_const = BitTimingConst::create(&buffer[172..172+52]);
        
        Self {
            unspec,
            bit_timing,
            bit_timing_const,
            clock,
            state,
            ctrl_mode,
            restart_ms,
            data_bit_timing,
            //data_bit_timing_const,
            
        }
    }
}


#[cfg(test)]
mod tests {
    use netlink_packet_utils::byteorder::{ByteOrder, NativeEndian};
    use tracing::debug;
    use crate::service::network_interface::manager::can::{BitTiming, BitTimingConst, CanLinkInfoData, CtrlMode, CtrlModeFlags};

    const CAN0_LINK_INFO_DATA: [u8; 204] = [36, 0, 1, 0, 32, 161, 7, 0, 188, 2, 0, 0, 25, 0, 0, 0, 27, 0, 0, 0, 28, 0, 0, 0, 24, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 52, 0, 2, 0, 109, 99, 112, 50, 53, 49, 120, 102, 100, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 8, 0, 3, 0, 0, 90, 98, 2, 8, 0, 4, 0, 2, 0, 0, 0, 12, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 6, 0, 0, 0, 0, 0, 8, 0, 8, 0, 128, 0, 0, 0, 52, 0, 10, 0, 109, 99, 112, 50, 53, 49, 120, 102, 100, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 32, 0, 0, 0, 1, 0, 0, 0, 16, 0, 0, 0, 16, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 8, 0, 15, 0, 0, 0, 0, 0, 12, 0, 17, 128, 8, 0, 1, 0, 179, 1, 0, 0];
    const CAN1_LINK_INFO_DATA: [u8; 240] = [36, 0, 1, 0, 64, 66, 15, 0, 238, 2, 0, 0, 25, 0, 0, 0, 14, 0, 0, 0, 15, 0, 0, 0, 10, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 52, 0, 2, 0, 109, 99, 112, 50, 53, 49, 120, 102, 100, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 8, 0, 3, 0, 0, 90, 98, 2, 8, 0, 4, 0, 0, 0, 0, 0, 12, 0, 5, 0, 0, 0, 0, 0, 48, 0, 0, 0, 8, 0, 6, 0, 232, 3, 0, 0, 8, 0, 8, 0, 0, 0, 0, 0, 36, 0, 9, 0, 0, 18, 122, 0, 88, 2, 0, 0, 25, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 52, 0, 10, 0, 109, 99, 112, 50, 53, 49, 120, 102, 100, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 32, 0, 0, 0, 1, 0, 0, 0, 16, 0, 0, 0, 16, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 8, 0, 15, 0, 0, 0, 0, 0, 12, 0, 17, 128, 8, 0, 1, 0, 179, 1, 0, 0];
    #[test_log::test]
    fn test_convert_bitrate() {
        let bitrate_slice = &CAN1_LINK_INFO_DATA[4..8];
        let bitrate_array: [u8; 4] = [64, 66, 15, 0];
        let result_slice = NativeEndian::read_u32(bitrate_slice);
        let result_array = NativeEndian::read_u32(&bitrate_array);
        debug!("bitrate_slice: {:?}", result_slice);
        assert_eq!(1000000, result_array);
        assert_eq!(result_slice, result_array);
    }
    #[test_log::test]
    fn test_decode_unspec() {
        let bitrate_slice = &CAN1_LINK_INFO_DATA[0..4];
        let result_slice = NativeEndian::read_u32(bitrate_slice);
        debug!("bitrate_slice: {:?}", result_slice);
    }

    #[test_log::test]
    fn test_decode_sample_point() {
        let sample_point_slice = &CAN1_LINK_INFO_DATA[8..12];
        let result_slice = NativeEndian::read_u32(sample_point_slice) as f32 / 1000.0;
        debug!("sample_point_slice: {:?}", result_slice);
        assert_eq!(result_slice, 0.75);
    }

    #[test_log::test]
    fn test_bit_timing() {
        let bit_timing = BitTiming::create(&CAN1_LINK_INFO_DATA[4..36]);
        
        debug!("Bit timing: {:?}", bit_timing);
        assert_eq!(bit_timing.sample_point, 750);
    }

    #[test_log::test]
    fn test_bit_timing_const() {
        let bit_timing = BitTimingConst::create(&CAN1_LINK_INFO_DATA[40..92]);

        debug!("Bit timing const: {:?}", bit_timing);
        assert_eq!(bit_timing.name, "mcp251xfd".to_string());
    }

    #[test_log::test]
    fn test_ctrl_mode_flags() {
        for i in 0..190 {
            let ctrl_mode = CtrlMode::create(&CAN1_LINK_INFO_DATA[i..i + 8]);
            let mut result = ctrl_mode.flags.contains(CtrlModeFlags::CAN_CTRLMODE_BERR_REPORTING);
            result &= ctrl_mode.flags.contains(CtrlModeFlags::CAN_CTRLMODE_FD);
            result &= (
                ctrl_mode.flags.clone() & 
                    CtrlModeFlags::CAN_CTRLMODE_LOOPBACK &
                    CtrlModeFlags::CAN_CTRLMODE_LISTENONLY &
                    CtrlModeFlags::CAN_CTRLMODE_3_SAMPLES &
                    CtrlModeFlags::CAN_CTRLMODE_ONE_SHOT &
                    CtrlModeFlags::CAN_CTRLMODE_PRESUME_ACK &
                    CtrlModeFlags::CAN_CTRLMODE_FD_NON_ISO &
                    CtrlModeFlags::CAN_CTRLMODE_CC_LEN8_DLC &
                    CtrlModeFlags::CAN_CTRLMODE_TDC_AUTO &
                    CtrlModeFlags::CAN_CTRLMODE_TDC_MANUAL
            ).is_empty();
            if result {
                debug!("Ctrl mode found at position: {:?}", i);
            }
        }
        
    }
    
    #[test_log::test]
    fn test_read_entire_buffer() {
        let can1_link_info_data = CanLinkInfoData::create(&CAN0_LINK_INFO_DATA);
        debug!("Link info data CAN1: {:?}", can1_link_info_data);

        //let can0_link_info_data = CanLinkInfoData::create(&CAN0_LINK_INFO_DATA);
        //debug!("Link info data CAN0: {:?}", can0_link_info_data);
    }

}