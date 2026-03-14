use spin::Mutex;
use x86_64::instructions::port::Port;

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;
const MAX_TEXT: usize = 48;
const RTL8139_IDR0: u16 = 0x00;
const RTL8139_COMMAND: u16 = 0x37;
const RTL8139_RBSTART: u16 = 0x30;
const RTL8139_CAPR: u16 = 0x38;
const RTL8139_CBR: u16 = 0x3A;
const RTL8139_IMR: u16 = 0x3C;
const RTL8139_ISR: u16 = 0x3E;
const RTL8139_TCR: u16 = 0x40;
const RTL8139_RCR: u16 = 0x44;
const RTL8139_TX_STATUS0: u16 = 0x10;
const RTL8139_TX_ADDR0: u16 = 0x20;
const RTL8139_CONFIG1: u16 = 0x52;
const RTL8139_MSR: u16 = 0x58;
const RTL8139_RESET: u8 = 0x10;
const RTL8139_RX_ENABLE: u8 = 0x08;
const RTL8139_TX_ENABLE: u8 = 0x04;
const RTL8139_ACCEPT_BROADCAST: u32 = 1 << 3;
const RTL8139_ACCEPT_PHYSICAL_MATCH: u32 = 1 << 1;
const RTL8139_WRAP: u32 = 1 << 7;
const RTL8139_ISR_RX_OK: u16 = 0x0001;
const RTL8139_ISR_TX_OK: u16 = 0x0004;
const RTL8139_RX_EMPTY: u8 = 0x01;
const RTL8139_RX_BUFFER_LEN: usize = 8192 + 16 + 1500;
const RTL8139_RX_RING_LEN: usize = 8192;
const RTL8139_TX_BUFFER_LEN: usize = 1536;
const RTL8139_TX_SLOTS: usize = 4;
const ETHERNET_HEADER_LEN: usize = 14;
const ETHERTYPE_ARP: u16 = 0x0806;
const ETHERTYPE_IPV4: u16 = 0x0800;
const ETHERTYPE_TEDDY_TEST: u16 = 0x88B5;
const ARP_PACKET_LEN: usize = 28;
const IPV4_MIN_HEADER_LEN: usize = 20;
const UDP_HEADER_LEN: usize = 8;
const IP_PROTOCOL_UDP: u8 = 17;
const DHCP_CLIENT_PORT: u16 = 68;
const DHCP_SERVER_PORT: u16 = 67;
const DHCP_DISCOVER_OPTIONS_LEN: usize = 13;
const DHCP_DISCOVER_LEN: usize = 236 + DHCP_DISCOVER_OPTIONS_LEN;
const DHCP_REQUEST_OPTIONS_LEN: usize = 19;
const DHCP_REQUEST_LEN: usize = 236 + DHCP_REQUEST_OPTIONS_LEN;

#[derive(Clone, Copy)]
pub enum NicKind {
    Rtl8139,
    E1000,
    E1000e,
    Vmxnet3,
    Unknown,
}

#[derive(Clone, Copy)]
pub struct NetworkTextBuffer {
    bytes: [u8; MAX_TEXT],
    len: usize,
}

impl NetworkTextBuffer {
    const fn new() -> Self {
        Self {
            bytes: [0; MAX_TEXT],
            len: 0,
        }
    }

    fn push_str(&mut self, text: &str) {
        let bytes = text.as_bytes();
        let write_len = bytes.len().min(self.bytes.len().saturating_sub(self.len));
        self.bytes[self.len..self.len + write_len].copy_from_slice(&bytes[..write_len]);
        self.len += write_len;
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("?")
    }
}

#[derive(Clone, Copy)]
pub struct Ipv4Address {
    octets: [u8; 4],
}

impl Ipv4Address {
    pub const fn unspecified() -> Self {
        Self { octets: [0, 0, 0, 0] }
    }

    pub const fn from_octets(octets: [u8; 4]) -> Self {
        Self { octets }
    }

    pub const fn broadcast() -> Self {
        Self {
            octets: [255, 255, 255, 255],
        }
    }

    pub fn octets(&self) -> [u8; 4] {
        self.octets
    }
}

#[derive(Clone, Copy)]
pub struct MacAddress {
    bytes: [u8; 6],
}

impl MacAddress {
    pub const fn zero() -> Self {
        Self { bytes: [0; 6] }
    }

    pub fn bytes(&self) -> [u8; 6] {
        self.bytes
    }

    pub const fn broadcast() -> Self {
        Self { bytes: [0xFF; 6] }
    }
}

#[derive(Clone, Copy)]
pub struct NetworkInfo {
    pub detected: bool,
    pub prepared: bool,
    pub nic_kind: NicKind,
    pub bus: u8,
    pub slot: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub io_base: u32,
    pub mmio_base: u32,
    pub mac: MacAddress,
    pub ip: Ipv4Address,
    pub router: Ipv4Address,
    pub dns: Ipv4Address,
    pub name: NetworkTextBuffer,
    pub dhcp_ready: bool,
    pub dns_ready: bool,
    pub sockets_ready: bool,
    pub driver_ready: bool,
    pub driver_state: NetworkTextBuffer,
    pub irq_line: u8,
    pub command_register: u8,
    pub interrupt_status: u16,
    pub rx_config: u32,
    pub tx_config: u32,
    pub rx_buffer_addr: u32,
    pub tx_buffer_addr: [u32; RTL8139_TX_SLOTS],
    pub rx_packets: u64,
    pub tx_completions: u64,
    pub tx_attempts: u64,
    pub last_rx_status: u16,
    pub last_rx_length: u16,
    pub last_rx_ethertype: u16,
    pub last_rx_source: MacAddress,
    pub last_rx_destination: MacAddress,
    pub arp_packets: u64,
    pub last_arp_opcode: u16,
    pub last_arp_sender_mac: MacAddress,
    pub last_arp_sender_ip: Ipv4Address,
    pub last_arp_target_ip: Ipv4Address,
    pub ipv4_packets: u64,
    pub last_ipv4_protocol: u8,
    pub last_ipv4_source: Ipv4Address,
    pub last_ipv4_destination: Ipv4Address,
    pub udp_packets: u64,
    pub last_udp_source_port: u16,
    pub last_udp_destination_port: u16,
    pub last_udp_length: u16,
    pub dhcp_packets: u64,
    pub dhcp_discover_attempts: u64,
    pub dhcp_request_attempts: u64,
    pub last_dhcp_message_type: u8,
    pub last_dhcp_xid: u32,
    pub dhcp_offer_ip: Ipv4Address,
    pub dhcp_server: Ipv4Address,
    pub current_rx_offset: u16,
    pub current_rx_read: u16,
    pub last_tx_length: u16,
}

#[derive(Clone, Copy)]
struct NetworkState {
    info: NetworkInfo,
    tx_slot: usize,
    probed: bool,
}

#[repr(align(16))]
struct Rtl8139RxBuffer([u8; RTL8139_RX_BUFFER_LEN]);

#[repr(align(16))]
struct Rtl8139TxBuffer([u8; RTL8139_TX_BUFFER_LEN]);

static mut RTL8139_RX_BUFFER: Rtl8139RxBuffer = Rtl8139RxBuffer([0; RTL8139_RX_BUFFER_LEN]);
static mut RTL8139_TX_BUFFERS: [Rtl8139TxBuffer; RTL8139_TX_SLOTS] = [
    Rtl8139TxBuffer([0; RTL8139_TX_BUFFER_LEN]),
    Rtl8139TxBuffer([0; RTL8139_TX_BUFFER_LEN]),
    Rtl8139TxBuffer([0; RTL8139_TX_BUFFER_LEN]),
    Rtl8139TxBuffer([0; RTL8139_TX_BUFFER_LEN]),
];

static NETWORK: Mutex<NetworkState> = Mutex::new(NetworkState {
    info: NetworkInfo {
        detected: false,
        prepared: false,
        nic_kind: NicKind::Unknown,
        bus: 0,
        slot: 0,
        function: 0,
        vendor_id: 0,
        device_id: 0,
        io_base: 0,
        mmio_base: 0,
        mac: MacAddress::zero(),
        ip: Ipv4Address::unspecified(),
        router: Ipv4Address::unspecified(),
        dns: Ipv4Address::unspecified(),
        name: NetworkTextBuffer::new(),
        dhcp_ready: false,
        dns_ready: false,
        sockets_ready: false,
        driver_ready: false,
        driver_state: NetworkTextBuffer::new(),
        irq_line: 0,
        command_register: 0,
        interrupt_status: 0,
        rx_config: 0,
        tx_config: 0,
        rx_buffer_addr: 0,
        tx_buffer_addr: [0; RTL8139_TX_SLOTS],
        rx_packets: 0,
        tx_completions: 0,
        tx_attempts: 0,
        last_rx_status: 0,
        last_rx_length: 0,
        last_rx_ethertype: 0,
        last_rx_source: MacAddress::zero(),
        last_rx_destination: MacAddress::zero(),
        arp_packets: 0,
        last_arp_opcode: 0,
        last_arp_sender_mac: MacAddress::zero(),
        last_arp_sender_ip: Ipv4Address::unspecified(),
        last_arp_target_ip: Ipv4Address::unspecified(),
        ipv4_packets: 0,
        last_ipv4_protocol: 0,
        last_ipv4_source: Ipv4Address::unspecified(),
        last_ipv4_destination: Ipv4Address::unspecified(),
        udp_packets: 0,
        last_udp_source_port: 0,
        last_udp_destination_port: 0,
        last_udp_length: 0,
        dhcp_packets: 0,
        dhcp_discover_attempts: 0,
        dhcp_request_attempts: 0,
        last_dhcp_message_type: 0,
        last_dhcp_xid: 0,
        dhcp_offer_ip: Ipv4Address::unspecified(),
        dhcp_server: Ipv4Address::unspecified(),
        current_rx_offset: 0,
        current_rx_read: 0,
        last_tx_length: 0,
    },
    tx_slot: 0,
    probed: false,
});

pub fn init() -> NetworkInfo {
    let mut state = NETWORK.lock();
    state.info = deferred_network_info();
    state.probed = false;
    state.info
}

pub fn info() -> NetworkInfo {
    let mut state = NETWORK.lock();
    ensure_probed(&mut state);
    state.info
}

pub fn send_test_frame() -> Result<(), &'static str> {
    let mut state = NETWORK.lock();
    ensure_probed(&mut state);
    if !matches!(state.info.nic_kind, NicKind::Rtl8139) || state.info.io_base == 0 {
        return Err("network: rtl8139 not ready");
    }

    let slot = state.tx_slot;
    let payload = b"TEDDYOS-NET";
    let frame_len = ETHERNET_HEADER_LEN + payload.len();
    if frame_len > RTL8139_TX_BUFFER_LEN {
        return Err("network: frame too large");
    }

    unsafe {
        let buffer = &mut RTL8139_TX_BUFFERS[slot].0;
        buffer[..6].fill(0xFF);
        buffer[6..12].copy_from_slice(&state.info.mac.bytes());
        buffer[12..14].copy_from_slice(&ETHERTYPE_TEDDY_TEST.to_be_bytes());
        buffer[ETHERNET_HEADER_LEN..frame_len].copy_from_slice(payload);

        let io_base = state.info.io_base as u16;
        Port::<u32>::new(io_base + RTL8139_TX_STATUS0 + (slot as u16 * 4)).write(frame_len as u32);
    }

    state.info.tx_attempts = state.info.tx_attempts.saturating_add(1);
    state.info.last_tx_length = frame_len as u16;
    Ok(())
}

pub fn send_arp_request(target: Ipv4Address) -> Result<(), &'static str> {
    let mut state = NETWORK.lock();
    ensure_probed(&mut state);
    if !matches!(state.info.nic_kind, NicKind::Rtl8139) || state.info.io_base == 0 {
        return Err("network: rtl8139 not ready");
    }

    let slot = state.tx_slot;
    let frame_len = ETHERNET_HEADER_LEN + ARP_PACKET_LEN;
    unsafe {
        let buffer = &mut RTL8139_TX_BUFFERS[slot].0;
        buffer[..6].fill(0xFF);
        buffer[6..12].copy_from_slice(&state.info.mac.bytes());
        buffer[12..14].copy_from_slice(&ETHERTYPE_ARP.to_be_bytes());

        let arp = &mut buffer[ETHERNET_HEADER_LEN..ETHERNET_HEADER_LEN + ARP_PACKET_LEN];
        arp[0..2].copy_from_slice(&1u16.to_be_bytes());
        arp[2..4].copy_from_slice(&0x0800u16.to_be_bytes());
        arp[4] = 6;
        arp[5] = 4;
        arp[6..8].copy_from_slice(&1u16.to_be_bytes());
        arp[8..14].copy_from_slice(&state.info.mac.bytes());
        arp[14..18].copy_from_slice(&state.info.ip.octets());
        arp[18..24].fill(0);
        arp[24..28].copy_from_slice(&target.octets());

        let io_base = state.info.io_base as u16;
        Port::<u32>::new(io_base + RTL8139_TX_STATUS0 + (slot as u16 * 4)).write(frame_len as u32);
    }

    state.info.tx_attempts = state.info.tx_attempts.saturating_add(1);
    state.info.last_tx_length = frame_len as u16;
    state.info.last_arp_target_ip = target;
    Ok(())
}

pub fn send_dhcp_discover() -> Result<(), &'static str> {
    let mut state = NETWORK.lock();
    ensure_probed(&mut state);
    if !matches!(state.info.nic_kind, NicKind::Rtl8139) || state.info.io_base == 0 {
        return Err("network: rtl8139 not ready");
    }

    let slot = state.tx_slot;
    let xid = 0x5444_0000u32 | ((state.info.dhcp_discover_attempts as u32 + 1) & 0xFFFF);
    let frame_len = build_dhcp_frame(
        &mut state.info,
        slot,
        xid,
        1,
        None,
        None,
        DHCP_DISCOVER_LEN,
    )?;

    state.info.tx_attempts = state.info.tx_attempts.saturating_add(1);
    state.info.dhcp_discover_attempts = state.info.dhcp_discover_attempts.saturating_add(1);
    state.info.last_tx_length = frame_len as u16;
    state.info.last_dhcp_xid = xid;
    state.info.last_dhcp_message_type = 1;
    state.info.dhcp_ready = false;
    Ok(())
}

pub fn send_dhcp_request() -> Result<(), &'static str> {
    let mut state = NETWORK.lock();
    ensure_probed(&mut state);
    if !matches!(state.info.nic_kind, NicKind::Rtl8139) || state.info.io_base == 0 {
        return Err("network: rtl8139 not ready");
    }
    if state.info.dhcp_offer_ip.octets() == Ipv4Address::unspecified().octets() {
        return Err("network: no dhcp offer available");
    }
    if state.info.dhcp_server.octets() == Ipv4Address::unspecified().octets() {
        return Err("network: no dhcp server available");
    }

    let slot = state.tx_slot;
    let xid = if state.info.last_dhcp_xid != 0 {
        state.info.last_dhcp_xid
    } else {
        0x5444_8000u32 | ((state.info.dhcp_request_attempts as u32 + 1) & 0xFFFF)
    };
    let offered_ip = state.info.dhcp_offer_ip;
    let server = state.info.dhcp_server;
    let frame_len = build_dhcp_frame(
        &mut state.info,
        slot,
        xid,
        3,
        Some(offered_ip),
        Some(server),
        DHCP_REQUEST_LEN,
    )?;

    state.info.tx_attempts = state.info.tx_attempts.saturating_add(1);
    state.info.dhcp_request_attempts = state.info.dhcp_request_attempts.saturating_add(1);
    state.info.last_tx_length = frame_len as u16;
    state.info.last_dhcp_xid = xid;
    state.info.last_dhcp_message_type = 3;
    Ok(())
}

fn build_dhcp_frame(
    info: &mut NetworkInfo,
    slot: usize,
    xid: u32,
    message_type: u8,
    requested_ip: Option<Ipv4Address>,
    server_id: Option<Ipv4Address>,
    dhcp_len: usize,
) -> Result<usize, &'static str> {
    let udp_len = (UDP_HEADER_LEN + dhcp_len) as u16;
    let ipv4_len = (IPV4_MIN_HEADER_LEN as u16) + udp_len;
    let frame_len = ETHERNET_HEADER_LEN + ipv4_len as usize;
    if frame_len > RTL8139_TX_BUFFER_LEN {
        return Err("network: dhcp frame too large");
    }

    unsafe {
        let buffer = &mut RTL8139_TX_BUFFERS[slot].0;
        let destination = MacAddress::broadcast();
        buffer[..6].copy_from_slice(&destination.bytes());
        buffer[6..12].copy_from_slice(&info.mac.bytes());
        buffer[12..14].copy_from_slice(&ETHERTYPE_IPV4.to_be_bytes());

        let ipv4 = &mut buffer[ETHERNET_HEADER_LEN..ETHERNET_HEADER_LEN + IPV4_MIN_HEADER_LEN];
        ipv4.fill(0);
        ipv4[0] = 0x45;
        ipv4[2..4].copy_from_slice(&ipv4_len.to_be_bytes());
        ipv4[4..6].copy_from_slice(&(info.tx_attempts as u16 + 1).to_be_bytes());
        ipv4[8] = 64;
        ipv4[9] = IP_PROTOCOL_UDP;
        ipv4[12..16].copy_from_slice(&Ipv4Address::unspecified().octets());
        ipv4[16..20].copy_from_slice(&Ipv4Address::broadcast().octets());
        let checksum = ipv4_checksum(ipv4);
        ipv4[10..12].copy_from_slice(&checksum.to_be_bytes());

        let udp_start = ETHERNET_HEADER_LEN + IPV4_MIN_HEADER_LEN;
        let udp = &mut buffer[udp_start..udp_start + UDP_HEADER_LEN];
        udp[0..2].copy_from_slice(&DHCP_CLIENT_PORT.to_be_bytes());
        udp[2..4].copy_from_slice(&DHCP_SERVER_PORT.to_be_bytes());
        udp[4..6].copy_from_slice(&udp_len.to_be_bytes());
        udp[6..8].copy_from_slice(&0u16.to_be_bytes());

        let dhcp = &mut buffer[udp_start + UDP_HEADER_LEN..udp_start + UDP_HEADER_LEN + dhcp_len];
        dhcp.fill(0);
        dhcp[0] = 1;
        dhcp[1] = 1;
        dhcp[2] = 6;
        dhcp[3] = 0;
        dhcp[4..8].copy_from_slice(&xid.to_be_bytes());
        dhcp[10..12].copy_from_slice(&0x8000u16.to_be_bytes());
        dhcp[28..34].copy_from_slice(&info.mac.bytes());
        dhcp[236..240].copy_from_slice(&[99, 130, 83, 99]);
        dhcp[240..243].copy_from_slice(&[53, 1, message_type]);

        let mut options_index = 243usize;
        if let Some(ip) = requested_ip {
            let octets = ip.octets();
            dhcp[options_index..options_index + 6]
                .copy_from_slice(&[50, 4, octets[0], octets[1], octets[2], octets[3]]);
            options_index += 6;
        }
        if let Some(server) = server_id {
            let octets = server.octets();
            dhcp[options_index..options_index + 6]
                .copy_from_slice(&[54, 4, octets[0], octets[1], octets[2], octets[3]]);
            options_index += 6;
        }
        dhcp[options_index..options_index + 5].copy_from_slice(&[55, 3, 1, 3, 6]);
        options_index += 5;
        dhcp[options_index] = 255;

        let io_base = info.io_base as u16;
        Port::<u32>::new(io_base + RTL8139_TX_STATUS0 + (slot as u16 * 4)).write(frame_len as u32);
    }

    Ok(frame_len)
}

pub fn poll() {
    let mut state = NETWORK.lock();
    if !state.probed {
        return;
    }
    if !matches!(state.info.nic_kind, NicKind::Rtl8139) || state.info.io_base == 0 {
        return;
    }

    let io_base = state.info.io_base as u16;
    unsafe {
        let mut isr = Port::<u16>::new(io_base + RTL8139_ISR);
        let pending = isr.read();
        if pending == 0 {
            return;
        }

        state.info.interrupt_status = pending;
        if pending & RTL8139_ISR_RX_OK != 0 {
            while command_has_packet(io_base) {
                if !consume_rtl8139_packet(&mut state.info, io_base) {
                    break;
                }
                state.info.rx_packets = state.info.rx_packets.saturating_add(1);
            }
        }
        if pending & RTL8139_ISR_TX_OK != 0 {
            state.info.tx_completions = state.info.tx_completions.saturating_add(1);
            state.tx_slot = (state.tx_slot + 1) % RTL8139_TX_SLOTS;
        }

        isr.write(pending);
    }
}

fn ensure_probed(state: &mut NetworkState) {
    if state.probed {
        return;
    }

    state.info = detect_supported_nic().unwrap_or_else(|| {
        let mut info = deferred_network_info();
        info.driver_state = {
            let mut text = NetworkTextBuffer::new();
            text.push_str("no supported NIC");
            text
        };
        info.name = {
            let mut text = NetworkTextBuffer::new();
            text.push_str("no supported NIC");
            text
        };
        info
    });
    state.probed = true;
}

fn deferred_network_info() -> NetworkInfo {
    NetworkInfo {
        detected: false,
        prepared: false,
        nic_kind: NicKind::Unknown,
        bus: 0,
        slot: 0,
        function: 0,
        vendor_id: 0,
        device_id: 0,
        io_base: 0,
        mmio_base: 0,
        mac: MacAddress::zero(),
        ip: Ipv4Address::unspecified(),
        router: Ipv4Address::unspecified(),
        dns: Ipv4Address::unspecified(),
        name: {
            let mut text = NetworkTextBuffer::new();
            text.push_str("network probe deferred");
            text
        },
        dhcp_ready: false,
        dns_ready: false,
        sockets_ready: false,
        driver_ready: false,
        driver_state: {
            let mut text = NetworkTextBuffer::new();
            text.push_str("probe on demand");
            text
        },
        irq_line: 0,
        command_register: 0,
        interrupt_status: 0,
        rx_config: 0,
        tx_config: 0,
        rx_buffer_addr: 0,
        tx_buffer_addr: [0; RTL8139_TX_SLOTS],
        rx_packets: 0,
        tx_completions: 0,
        tx_attempts: 0,
        last_rx_status: 0,
        last_rx_length: 0,
        last_rx_ethertype: 0,
        last_rx_source: MacAddress::zero(),
        last_rx_destination: MacAddress::zero(),
        arp_packets: 0,
        last_arp_opcode: 0,
        last_arp_sender_mac: MacAddress::zero(),
        last_arp_sender_ip: Ipv4Address::unspecified(),
        last_arp_target_ip: Ipv4Address::unspecified(),
        ipv4_packets: 0,
        last_ipv4_protocol: 0,
        last_ipv4_source: Ipv4Address::unspecified(),
        last_ipv4_destination: Ipv4Address::unspecified(),
        udp_packets: 0,
        last_udp_source_port: 0,
        last_udp_destination_port: 0,
        last_udp_length: 0,
        dhcp_packets: 0,
        dhcp_discover_attempts: 0,
        dhcp_request_attempts: 0,
        last_dhcp_message_type: 0,
        last_dhcp_xid: 0,
        dhcp_offer_ip: Ipv4Address::unspecified(),
        dhcp_server: Ipv4Address::unspecified(),
        current_rx_offset: 0,
        current_rx_read: 0,
        last_tx_length: 0,
    }
}

fn detect_supported_nic() -> Option<NetworkInfo> {
    for bus in 0u8..=255 {
        for slot in 0u8..32 {
            for function in 0u8..8 {
                let vendor_device = pci_read_u32(bus, slot, function, 0x00);
                let vendor_id = (vendor_device & 0xFFFF) as u16;
                if vendor_id == 0xFFFF {
                    if function == 0 {
                        break;
                    }
                    continue;
                }

                let class_reg = pci_read_u32(bus, slot, function, 0x08);
                let class_code = ((class_reg >> 24) & 0xFF) as u8;
                if class_code != 0x02 {
                    continue;
                }

                let device_id = (vendor_device >> 16) as u16;
                let Some((nic_kind, name)) = supported_nic(vendor_id, device_id) else {
                    continue;
                };

                let mut info = NetworkInfo {
                    detected: true,
                    prepared: false,
                    nic_kind,
                    bus,
                    slot,
                    function,
                    vendor_id,
                    device_id,
                    io_base: 0,
                    mmio_base: 0,
                    mac: MacAddress::zero(),
                    ip: Ipv4Address::unspecified(),
                    router: Ipv4Address::unspecified(),
                    dns: Ipv4Address::unspecified(),
                    name: {
                        let mut text = NetworkTextBuffer::new();
                        text.push_str(name);
                        text
                    },
                    dhcp_ready: false,
                    dns_ready: false,
                    sockets_ready: false,
                    driver_ready: false,
                    driver_state: NetworkTextBuffer::new(),
                    irq_line: 0,
                    command_register: 0,
                    interrupt_status: 0,
                    rx_config: 0,
                    tx_config: 0,
                    rx_buffer_addr: 0,
                    tx_buffer_addr: [0; RTL8139_TX_SLOTS],
                    rx_packets: 0,
                    tx_completions: 0,
                    tx_attempts: 0,
                    last_rx_status: 0,
                    last_rx_length: 0,
                    last_rx_ethertype: 0,
                    last_rx_source: MacAddress::zero(),
                    last_rx_destination: MacAddress::zero(),
                    arp_packets: 0,
                    last_arp_opcode: 0,
                    last_arp_sender_mac: MacAddress::zero(),
                    last_arp_sender_ip: Ipv4Address::unspecified(),
                    last_arp_target_ip: Ipv4Address::unspecified(),
                    ipv4_packets: 0,
                    last_ipv4_protocol: 0,
                    last_ipv4_source: Ipv4Address::unspecified(),
                    last_ipv4_destination: Ipv4Address::unspecified(),
                    udp_packets: 0,
                    last_udp_source_port: 0,
                    last_udp_destination_port: 0,
                    last_udp_length: 0,
                    dhcp_packets: 0,
                    dhcp_discover_attempts: 0,
                    dhcp_request_attempts: 0,
                    last_dhcp_message_type: 0,
                    last_dhcp_xid: 0,
                    dhcp_offer_ip: Ipv4Address::unspecified(),
                    dhcp_server: Ipv4Address::unspecified(),
                    current_rx_offset: 0,
                    current_rx_read: 0,
                    last_tx_length: 0,
                };

                prepare_pci_device(bus, slot, function, nic_kind, &mut info);
                return Some(info);
            }
        }
    }

    None
}

fn supported_nic(vendor_id: u16, device_id: u16) -> Option<(NicKind, &'static str)> {
    match (vendor_id, device_id) {
        (0x10EC, 0x8139) => Some((NicKind::Rtl8139, "rtl8139")),
        (0x8086, 0x100E) => Some((NicKind::E1000, "e1000")),
        (0x8086, 0x10D3) => Some((NicKind::E1000e, "e1000e")),
        (0x15AD, 0x07B0) => Some((NicKind::Vmxnet3, "vmxnet3")),
        _ => None,
    }
}

fn prepare_pci_device(bus: u8, slot: u8, function: u8, nic_kind: NicKind, info: &mut NetworkInfo) {
    let command_reg = pci_read_u32(bus, slot, function, 0x04);
    let command = (command_reg & 0xFFFF) as u16 | 0x0004 | 0x0002 | 0x0001;
    pci_write_u32(
        bus,
        slot,
        function,
        0x04,
        (command_reg & 0xFFFF_0000) | command as u32,
    );

    let bar0 = pci_read_u32(bus, slot, function, 0x10);
    let bar1 = pci_read_u32(bus, slot, function, 0x14);
    if bar0 & 0x1 == 0x1 {
        info.io_base = bar0 & 0xFFFF_FFFC;
    } else {
        info.mmio_base = bar0 & 0xFFFF_FFF0;
    }
    if info.mmio_base == 0 && bar1 & 0x1 == 0 {
        info.mmio_base = bar1 & 0xFFFF_FFF0;
    }

    if matches!(nic_kind, NicKind::Rtl8139) && info.io_base != 0 {
        let io_base = info.io_base as u16;
        info.mac = read_rtl8139_mac(io_base);
        info.irq_line = (pci_read_u32(bus, slot, function, 0x3C) & 0xFF) as u8;
        initialize_rtl8139(io_base, info);
    }

    info.prepared = true;
}

fn read_rtl8139_mac(io_base: u16) -> MacAddress {
    let mut mac = [0u8; 6];
    unsafe {
        for (index, byte) in mac.iter_mut().enumerate() {
            *byte = Port::<u8>::new(io_base + RTL8139_IDR0 + index as u16).read();
        }
    }
    MacAddress { bytes: mac }
}

fn initialize_rtl8139(io_base: u16, info: &mut NetworkInfo) {
    unsafe {
        let mut config1 = Port::<u8>::new(io_base + RTL8139_CONFIG1);
        config1.write(0x00);

        let mut command = Port::<u8>::new(io_base + RTL8139_COMMAND);
        command.write(RTL8139_RESET);
        for _ in 0..100_000 {
            if command.read() & RTL8139_RESET == 0 {
                break;
            }
        }

        Port::<u16>::new(io_base + RTL8139_IMR).write(0x0000);
        Port::<u16>::new(io_base + RTL8139_ISR).write(0xFFFF);
        Port::<u16>::new(io_base + RTL8139_CAPR).write(0x0000);
        Port::<u16>::new(io_base + RTL8139_CBR).write(0x0000);

        info.rx_buffer_addr = core::ptr::addr_of!(RTL8139_RX_BUFFER.0) as u32;
        Port::<u32>::new(io_base + RTL8139_RBSTART).write(info.rx_buffer_addr);

        for index in 0..RTL8139_TX_SLOTS {
            let tx_addr = core::ptr::addr_of!(RTL8139_TX_BUFFERS[index].0) as u32;
            info.tx_buffer_addr[index] = tx_addr;
            Port::<u32>::new(io_base + RTL8139_TX_ADDR0 + (index as u16 * 4)).write(tx_addr);
            Port::<u32>::new(io_base + RTL8139_TX_STATUS0 + (index as u16 * 4)).write(0);
        }

        let rx_config = RTL8139_ACCEPT_BROADCAST | RTL8139_ACCEPT_PHYSICAL_MATCH | RTL8139_WRAP;
        Port::<u32>::new(io_base + RTL8139_RCR).write(rx_config);
        let tx_config = Port::<u32>::new(io_base + RTL8139_TCR).read();
        command.write(RTL8139_RX_ENABLE | RTL8139_TX_ENABLE);

        info.command_register = command.read();
        info.interrupt_status = Port::<u16>::new(io_base + RTL8139_ISR).read();
        info.rx_config = Port::<u32>::new(io_base + RTL8139_RCR).read();
        info.tx_config = tx_config;
        info.current_rx_read = Port::<u16>::new(io_base + RTL8139_CBR).read();

        let link_ok = Port::<u8>::new(io_base + RTL8139_MSR).read() & 0x04 != 0;
        info.driver_ready = info.command_register & (RTL8139_RX_ENABLE | RTL8139_TX_ENABLE)
            == (RTL8139_RX_ENABLE | RTL8139_TX_ENABLE);
        info.driver_state = {
            let mut text = NetworkTextBuffer::new();
            if info.driver_ready {
                if link_ok {
                    text.push_str("rtl8139 rx/tx ready link-up");
                } else {
                    text.push_str("rtl8139 rx/tx ready link-down");
                }
            } else {
                text.push_str("rtl8139 init failed");
            }
            text
        };
    }
}

fn command_has_packet(io_base: u16) -> bool {
    unsafe { Port::<u8>::new(io_base + RTL8139_COMMAND).read() & RTL8139_RX_EMPTY == 0 }
}

fn consume_rtl8139_packet(info: &mut NetworkInfo, io_base: u16) -> bool {
    let offset = (info.current_rx_offset as usize) % RTL8139_RX_RING_LEN;
    let packet = unsafe { &RTL8139_RX_BUFFER.0 };
    let status = u16::from_le_bytes([packet[offset], packet[(offset + 1) % RTL8139_RX_RING_LEN]]);
    let length = u16::from_le_bytes([packet[(offset + 2) % RTL8139_RX_RING_LEN], packet[(offset + 3) % RTL8139_RX_RING_LEN]]);

    info.last_rx_status = status;
    info.last_rx_length = length;

    if length as usize >= ETHERNET_HEADER_LEN + 4 {
        let header_offset = (offset + 4) % RTL8139_RX_RING_LEN;
        info.last_rx_destination = read_mac_from_ring(packet, header_offset);
        info.last_rx_source = read_mac_from_ring(packet, (header_offset + 6) % RTL8139_RX_RING_LEN);
        let type_hi = packet[(header_offset + 12) % RTL8139_RX_RING_LEN];
        let type_lo = packet[(header_offset + 13) % RTL8139_RX_RING_LEN];
        info.last_rx_ethertype = u16::from_be_bytes([type_hi, type_lo]);
        if info.last_rx_ethertype == ETHERTYPE_ARP && length as usize >= ETHERNET_HEADER_LEN + ARP_PACKET_LEN + 4 {
            parse_arp_packet(info, packet, (header_offset + ETHERNET_HEADER_LEN) % RTL8139_RX_RING_LEN);
        } else if info.last_rx_ethertype == ETHERTYPE_IPV4 && length as usize >= ETHERNET_HEADER_LEN + IPV4_MIN_HEADER_LEN + 4 {
            parse_ipv4_packet(info, packet, (header_offset + ETHERNET_HEADER_LEN) % RTL8139_RX_RING_LEN);
        }
    }

    let advance = ((length as usize + 4 + 3) & !3) % RTL8139_RX_RING_LEN;
    let new_offset = (offset + advance) % RTL8139_RX_RING_LEN;
    info.current_rx_offset = new_offset as u16;
    unsafe {
        Port::<u16>::new(io_base + RTL8139_CAPR).write(info.current_rx_offset.wrapping_sub(16));
        info.current_rx_read = Port::<u16>::new(io_base + RTL8139_CBR).read();
    }
    true
}

fn read_mac_from_ring(packet: &[u8; RTL8139_RX_BUFFER_LEN], offset: usize) -> MacAddress {
    let mut mac = [0u8; 6];
    for (index, byte) in mac.iter_mut().enumerate() {
        *byte = packet[(offset + index) % RTL8139_RX_RING_LEN];
    }
    MacAddress { bytes: mac }
}

fn parse_arp_packet(info: &mut NetworkInfo, packet: &[u8; RTL8139_RX_BUFFER_LEN], offset: usize) {
    info.arp_packets = info.arp_packets.saturating_add(1);
    let opcode_hi = packet[(offset + 6) % RTL8139_RX_RING_LEN];
    let opcode_lo = packet[(offset + 7) % RTL8139_RX_RING_LEN];
    info.last_arp_opcode = u16::from_be_bytes([opcode_hi, opcode_lo]);
    info.last_arp_sender_mac = read_mac_from_ring(packet, (offset + 8) % RTL8139_RX_RING_LEN);
    info.last_arp_sender_ip = read_ipv4_from_ring(packet, (offset + 14) % RTL8139_RX_RING_LEN);
    info.last_arp_target_ip = read_ipv4_from_ring(packet, (offset + 24) % RTL8139_RX_RING_LEN);
}

fn parse_ipv4_packet(info: &mut NetworkInfo, packet: &[u8; RTL8139_RX_BUFFER_LEN], offset: usize) {
    info.ipv4_packets = info.ipv4_packets.saturating_add(1);
    let version_ihl = packet[offset % RTL8139_RX_RING_LEN];
    let ihl = usize::from(version_ihl & 0x0F) * 4;
    if ihl < IPV4_MIN_HEADER_LEN {
        return;
    }

    info.last_ipv4_protocol = packet[(offset + 9) % RTL8139_RX_RING_LEN];
    info.last_ipv4_source = read_ipv4_from_ring(packet, (offset + 12) % RTL8139_RX_RING_LEN);
    info.last_ipv4_destination = read_ipv4_from_ring(packet, (offset + 16) % RTL8139_RX_RING_LEN);

    if info.last_ipv4_protocol == IP_PROTOCOL_UDP {
        parse_udp_packet(info, packet, (offset + ihl) % RTL8139_RX_RING_LEN);
    }
}

fn parse_udp_packet(info: &mut NetworkInfo, packet: &[u8; RTL8139_RX_BUFFER_LEN], offset: usize) {
    info.udp_packets = info.udp_packets.saturating_add(1);
    info.last_udp_source_port = u16::from_be_bytes([
        packet[offset % RTL8139_RX_RING_LEN],
        packet[(offset + 1) % RTL8139_RX_RING_LEN],
    ]);
    info.last_udp_destination_port = u16::from_be_bytes([
        packet[(offset + 2) % RTL8139_RX_RING_LEN],
        packet[(offset + 3) % RTL8139_RX_RING_LEN],
    ]);
    info.last_udp_length = u16::from_be_bytes([
        packet[(offset + 4) % RTL8139_RX_RING_LEN],
        packet[(offset + 5) % RTL8139_RX_RING_LEN],
    ]);

    if info.last_udp_source_port == DHCP_SERVER_PORT || info.last_udp_destination_port == DHCP_CLIENT_PORT {
        info.dhcp_packets = info.dhcp_packets.saturating_add(1);
        let dhcp_offset = (offset + UDP_HEADER_LEN) % RTL8139_RX_RING_LEN;
        info.last_dhcp_xid = u32::from_be_bytes([
            packet[(dhcp_offset + 4) % RTL8139_RX_RING_LEN],
            packet[(dhcp_offset + 5) % RTL8139_RX_RING_LEN],
            packet[(dhcp_offset + 6) % RTL8139_RX_RING_LEN],
            packet[(dhcp_offset + 7) % RTL8139_RX_RING_LEN],
        ]);
        info.last_dhcp_message_type = parse_dhcp_message_type(packet, dhcp_offset);
        info.dhcp_offer_ip = read_ipv4_from_ring(packet, (dhcp_offset + 16) % RTL8139_RX_RING_LEN);

        if let Some(server) = parse_dhcp_option_ipv4(packet, dhcp_offset, 54) {
            info.dhcp_server = server;
        }
        if let Some(router) = parse_dhcp_option_ipv4(packet, dhcp_offset, 3) {
            info.router = router;
        }
        if let Some(dns) = parse_dhcp_option_ipv4(packet, dhcp_offset, 6) {
            info.dns = dns;
            info.dns_ready = dns.octets() != Ipv4Address::unspecified().octets();
        }

        match info.last_dhcp_message_type {
            2 => {
                info.dhcp_ready = false;
                info.driver_state = {
                    let mut text = NetworkTextBuffer::new();
                    text.push_str("rtl8139 dhcp offer received");
                    text
                };
            }
            5 => {
                info.ip = info.dhcp_offer_ip;
                info.dhcp_ready = true;
                info.driver_state = {
                    let mut text = NetworkTextBuffer::new();
                    text.push_str("rtl8139 dhcp lease active");
                    text
                };
            }
            6 => {
                info.dhcp_ready = false;
                info.ip = Ipv4Address::unspecified();
                info.driver_state = {
                    let mut text = NetworkTextBuffer::new();
                    text.push_str("rtl8139 dhcp nack");
                    text
                };
            }
            _ => {}
        }
    }
}

fn parse_dhcp_message_type(packet: &[u8; RTL8139_RX_BUFFER_LEN], dhcp_offset: usize) -> u8 {
    let options_offset = (dhcp_offset + 240) % RTL8139_RX_RING_LEN;
    let mut index = options_offset;
    for _ in 0..64 {
        let option = packet[index % RTL8139_RX_RING_LEN];
        if option == 0 {
            index = (index + 1) % RTL8139_RX_RING_LEN;
            continue;
        }
        if option == 255 {
            break;
        }
        let length = packet[(index + 1) % RTL8139_RX_RING_LEN] as usize;
        if option == 53 && length >= 1 {
            return packet[(index + 2) % RTL8139_RX_RING_LEN];
        }
        index = (index + 2 + length) % RTL8139_RX_RING_LEN;
    }
    0
}

fn parse_dhcp_option_ipv4(
    packet: &[u8; RTL8139_RX_BUFFER_LEN],
    dhcp_offset: usize,
    target_option: u8,
) -> Option<Ipv4Address> {
    let options_offset = (dhcp_offset + 240) % RTL8139_RX_RING_LEN;
    let mut index = options_offset;
    for _ in 0..64 {
        let option = packet[index % RTL8139_RX_RING_LEN];
        if option == 0 {
            index = (index + 1) % RTL8139_RX_RING_LEN;
            continue;
        }
        if option == 255 {
            break;
        }

        let length = packet[(index + 1) % RTL8139_RX_RING_LEN] as usize;
        if option == target_option && length >= 4 {
            return Some(Ipv4Address::from_octets([
                packet[(index + 2) % RTL8139_RX_RING_LEN],
                packet[(index + 3) % RTL8139_RX_RING_LEN],
                packet[(index + 4) % RTL8139_RX_RING_LEN],
                packet[(index + 5) % RTL8139_RX_RING_LEN],
            ]));
        }
        index = (index + 2 + length) % RTL8139_RX_RING_LEN;
    }
    None
}

fn read_ipv4_from_ring(packet: &[u8; RTL8139_RX_BUFFER_LEN], offset: usize) -> Ipv4Address {
    Ipv4Address::from_octets([
        packet[offset % RTL8139_RX_RING_LEN],
        packet[(offset + 1) % RTL8139_RX_RING_LEN],
        packet[(offset + 2) % RTL8139_RX_RING_LEN],
        packet[(offset + 3) % RTL8139_RX_RING_LEN],
    ])
}

fn ipv4_checksum(header: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut index = 0usize;
    while index + 1 < header.len() {
        sum = sum.wrapping_add(u16::from_be_bytes([header[index], header[index + 1]]) as u32);
        index += 2;
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

fn pci_read_u32(bus: u8, slot: u8, function: u8, offset: u8) -> u32 {
    unsafe {
        let address = pci_address(bus, slot, function, offset);
        Port::<u32>::new(PCI_CONFIG_ADDRESS).write(address);
        Port::<u32>::new(PCI_CONFIG_DATA).read()
    }
}

fn pci_write_u32(bus: u8, slot: u8, function: u8, offset: u8, value: u32) {
    unsafe {
        let address = pci_address(bus, slot, function, offset);
        Port::<u32>::new(PCI_CONFIG_ADDRESS).write(address);
        Port::<u32>::new(PCI_CONFIG_DATA).write(value);
    }
}

fn pci_address(bus: u8, slot: u8, function: u8, offset: u8) -> u32 {
    0x8000_0000
        | ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((function as u32) << 8)
        | (u32::from(offset) & 0xFC)
}
