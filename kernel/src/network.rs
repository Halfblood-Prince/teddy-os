use spin::Mutex;
use x86_64::instructions::port::Port;

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;
const MAX_TEXT: usize = 48;

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
}

#[derive(Clone, Copy)]
struct NetworkState {
    info: NetworkInfo,
}

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
    },
});

pub fn init() -> NetworkInfo {
    let mut state = NETWORK.lock();
    state.info = detect_supported_nic().unwrap_or(NetworkInfo {
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
            text.push_str("no supported NIC");
            text
        },
        dhcp_ready: false,
        dns_ready: false,
        sockets_ready: false,
    });
    state.info
}

pub fn info() -> NetworkInfo {
    NETWORK.lock().info
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
        info.mac = read_rtl8139_mac(info.io_base as u16);
    }

    info.prepared = true;
}

fn read_rtl8139_mac(io_base: u16) -> MacAddress {
    let mut mac = [0u8; 6];
    unsafe {
        for (index, byte) in mac.iter_mut().enumerate() {
            *byte = Port::<u8>::new(io_base + index as u16).read();
        }
    }
    MacAddress { bytes: mac }
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
