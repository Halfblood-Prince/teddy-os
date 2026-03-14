# Phase 10

## Implemented in This Phase

- a new modular kernel networking module at `kernel/src/network.rs`
- PCI-based scanning for supported VMware-friendly NICs:
  - `rtl8139`
  - `e1000`
  - `e1000e`
  - `vmxnet3`
- PCI command-register preparation for detected network devices
- RTL8139-specific device initialization path:
  - power/config register enable
  - software reset
  - interrupt mask clear
  - RX buffer programming
  - TX buffer slot programming
  - RX/TX command enable
  - receive filter configuration
- poll-based NIC status tracking from the runtime loop
- receive-ring metadata parsing for RTL8139:
  - packet status
  - frame length
  - source MAC
  - destination MAC
  - EtherType
- test transmit path for a broadcast Ethernet frame
- basic NIC metadata reporting:
  - vendor/device IDs
  - bus/slot/function
  - BAR-derived I/O or MMIO base
  - RTL8139 MAC address readout when available
  - IRQ line and selected RTL8139 register snapshots
  - RX/TX DMA buffer addresses
  - RX/TX completion counters
  - last received frame metadata
- terminal diagnostic commands:
  - `netinfo`
  - `netdiag`
  - `netsend`
  - `dhcp`
  - `dns`
  - `fetch`

## Scope

This Phase 10 implementation now includes both PCI NIC detection and a concrete
device-specific initialization path for RTL8139, including RX/TX DMA buffer
programming, poll-based runtime status tracking, basic received-frame metadata
parsing, and a controlled test transmit path.

It does not yet complete a packet path, DHCP lease acquisition, DNS resolution,
TCP/UDP data transfer, or HTTP/HTTPS fetches. Those remain the next networking
steps on top of this device-discovery and hardware-init foundation.

## Terminal Commands

- `netinfo` prints the detected NIC, PCI location, BAR information, and MAC when available
- `netdiag` prints IRQ, selected NIC register state, DMA buffer addresses, packet counters, and last-frame metadata
- `netsend` queues a small broadcast test Ethernet frame on RTL8139
- `dhcp` reports the current DHCP-client scaffolding status
- `dns <host>` reports the current DNS-resolver scaffolding status
- `fetch <url>` reports the current transport/fetch scaffolding status

## VMware Test Instructions

1. Configure the VMware VM with a supported virtual NIC.
2. Boot Teddy-OS.
3. Open the terminal.
4. Run `netinfo`.
5. Confirm that a supported NIC is detected and that PCI/BAR information is shown.
6. Run `netdiag` and confirm IRQ/register state, DMA addresses, packet counters, and frame metadata are shown for RTL8139 when that device is selected.
7. Run `netsend` and confirm the transmit-attempt counter changes.
8. Run `dhcp`, `dns example.com`, and `fetch https://example.com` to confirm the networking surface is reachable from inside the OS.

## Known Limitations

- there is no completed protocol stack above raw Ethernet frame handling yet
- DHCP, DNS, TCP, UDP, and HTTP/HTTPS are not finished yet
- the current work is a networking foundation and diagnostics pass, not a full updater-ready internet stack
- compile and VMware verification were not possible in this shell because the Rust toolchain is not available on `PATH`
