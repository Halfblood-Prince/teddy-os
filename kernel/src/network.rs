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
const RTL8139_RX_BUFFER_LEN: usize = 8192 + 16 + 1500;
const RTL8139_TX_BUFFER_LEN: usize = 1536;
const RTL8139_TX_SLOTS: usize = 4;

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
    pub last_rx_status: u16,
    pub current_rx_offset: u16,
    pub current_rx_read: u16,
}

#[derive(Clone, Copy)]
struct NetworkState {
    info: NetworkInfo,
    tx_slot: usize,
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
        last_rx_status: 0,
        current_rx_offset: 0,
        current_rx_read: 0,
    },
    tx_slot: 0,
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
        driver_ready: false,
        driver_state: {
            let mut text = NetworkTextBuffer::new();
            text.push_str("not initialized");
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
        last_rx_status: 0,
        current_rx_offset: 0,
        current_rx_read: 0,
    });
    state.info
}

pub fn info() -> NetworkInfo {
    NETWORK.lock().info
}

pub fn poll() {
    let mut state = NETWORK.lock();
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
            state.info.rx_packets = state.info.rx_packets.saturating_add(1);
            state.info.last_rx_status = read_rx_status(io_base, state.info.current_rx_offset);
            state.info.current_rx_read = Port::<u16>::new(io_base + RTL8139_CBR).read();
        }
        if pending & RTL8139_ISR_TX_OK != 0 {
            state.info.tx_completions = state.info.tx_completions.saturating_add(1);
            state.tx_slot = (state.tx_slot + 1) % RTL8139_TX_SLOTS;
        }

        isr.write(pending);
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
                    last_rx_status: 0,
                    current_rx_offset: 0,
                    current_rx_read: 0,
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

unsafe fn read_rx_status(io_base: u16, _offset: u16) -> u16 {
    Port::<u16>::new(io_base + RTL8139_CAPR).read()
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
