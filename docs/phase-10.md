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
- ARP support:
  - parse received ARP frame metadata
  - transmit broadcast ARP requests from the terminal
- minimal IPv4/UDP support:
  - parse received IPv4 metadata
  - parse received UDP metadata
  - parse DHCP-related UDP metadata
  - transmit a DHCP Discover broadcast
  - parse DHCP Offer/Ack/Nak details
  - extract offered IPv4, router, DNS, and DHCP server metadata
  - apply the configured IPv4 address when a DHCP Ack is observed
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
  - last observed ARP metadata
  - last observed IPv4/UDP/DHCP metadata
- terminal diagnostic commands:
  - `netinfo`
  - `netdiag`
  - `netsend`
  - `arp <ipv4>`
  - `dhcp`
  - `dns`
  - `fetch`

## Scope

This Phase 10 implementation now includes both PCI NIC detection and a concrete
device-specific initialization path for RTL8139, including RX/TX DMA buffer
programming, poll-based runtime status tracking, basic received-frame metadata
parsing, ARP frame handling, initial IPv4/UDP parsing, and controlled transmit
paths including DHCP Discover. It now also keeps track of offered/configured
addressing details from DHCP replies so lease progress is visible from inside
the terminal.

It does not yet complete a full DHCP request/renew state machine, DNS resolution,
TCP/UDP data transfer, or HTTP/HTTPS fetches. Those remain the next networking
steps on top of this device-discovery and hardware-init foundation.

## Terminal Commands

- `netinfo` prints the detected NIC, PCI location, BAR information, MAC, and current IPv4/router/DNS state
- `netdiag` prints IRQ, selected NIC register state, DMA buffer addresses, packet counters, last-frame metadata, and DHCP lease details
- `netsend` queues a small broadcast test Ethernet frame on RTL8139
- `arp <ipv4>` queues a broadcast ARP request for the target IPv4 address
- `dhcp` queues a DHCP Discover broadcast
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
8. Run `arp 192.168.1.1` and confirm the transmit-attempt counter changes again.
9. If other guests or the host network generate ARP traffic, confirm `netdiag` shows ARP counters and last ARP metadata.
10. Run `dhcp` and confirm the DHCP transmit counter changes.
11. If a DHCP reply is seen, confirm `netdiag` shows IPv4/UDP/DHCP counters, the offered IPv4, DHCP server, and any router/DNS values.
12. If a DHCP Ack is seen, confirm `netinfo` reports a non-zero configured IPv4 address and `netdiag` reports `ready yes`.
13. Run `dns example.com` and `fetch https://example.com` to confirm the higher-level networking surface is still reachable from inside the OS.

## Known Limitations

- DHCP Discover transmission and Offer/Ack parsing are present, but the full lease-request/renew state machine is not complete yet
- DNS, TCP, UDP sockets, and HTTP/HTTPS are not finished yet
- the current work is a networking foundation and diagnostics pass, not a full updater-ready internet stack
- compile and VMware verification were not possible in this shell because the Rust toolchain is not available on `PATH`
