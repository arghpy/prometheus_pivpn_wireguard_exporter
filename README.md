# Prometheus PiVPN Wireguard Exporter

[![Super-Linter](https://github.com/arghpy/prometheus_pivpn_wireguard_exporter/actions/workflows/linter.yml/badge.svg)](https://github.com/marketplace/actions/super-linter)

A prometheus exporter for a default installation of wireguard via [PiVPN](https://github.com/pivpn/pivpn).

## Motivation

I searched for a minimalist, plug-and-play solution to monitor clients connected to my wireguard
server. There were some solutions, but I wanted one that would show custom names, tailored on
the configuration format that PiVPN provides.

I couldn't find such solution, so I created one.

## Installation

### Prerequisites

The following prerequisites are necessary in order to run:

- a running wireguard server on linux
- a folder containing the public keys of clients, structured like:

```text
/etc/wireguard/keys
├── john_laptop_pub
├── john_phone_pub
├── jane_laptop_pub
├── jane_phone_pub
└── homelab_pub
```

These will we automatically created every time a client is added/removed using pivpn.

### Download from Releases

Soon to come.

### Building

In order to build the program, you will need:

- [rust](https://www.rust-lang.org/tools/install)
- [Git](https://git-scm.com/downloads)

1. Clone the repository:

```bash
git clone https://github.com/arghpy/prometheus_pivpn_wireguard_exporter.git
```

2. Build for release and copy the resulting binary in `~/.cargo/bin/`:

```bash
cargo install --path .
```

3. Run as root the program:

```bash
sudo /home/<user>/.cargo/bin/prometheus_pivpn_wireguard_exporter
```

4. For additional information and options to pass, run:

```bash
/home/<user>/.cargo/bin/prometheus_pivpn_wireguard_exporter --help
```

Optionally run it as a systemd service:

1. Create the file **/etc/systemd/system/wireguard_exporter.service** with contents:

```ini
[Unit]
Description=Prometheus WireGuard Exporter
Wants=network-online.target
After=network-online.target

[Service]
Type=simple
ExecStart=/home/<user>/.cargo/bin/prometheus_pivpn_wireguard_exporter

[Install]
WantedBy=multi-user.target
```

2. Start and enable the service:

```bash
sudo systemctl enable --now wireguard_exporter.service
```

## Basic operation

The program will parse the public keys from the configuration directory **/etc/wireguard/keys**, creating an object
of the form:

```json
{
  "key": "client_name"
}
```

where:

- **key** will be the contents of the file
- **client_name** will be the name of the file, stripped of `_pub`

For example, the file `/etc/wireguard/john_laptop_pub` with content `XXXXXXXXX` will be in the hashmap:

```json
{
  "XXXXXXXXX": "john_laptop"
}
```

Next, it will do a dump on the wireguard interface specified (default `wg0`):

```bash
wg show <interface> dump
```

It will collect all the information and store it in an object like:

```json
{
    [
        "last_handshake": {
            "description": "Time of the last handshake since epoch",
                "data": data,
                "client": client,
                "interface": interface,
        },
        "since_last_handshake": {
            "description": "Seconds passed since the last handshake",
            "data": now - data,
            "client": client,
            "interface": interface,
        },
        "received_bytes_total": {
            "description": "Total bytes received from peer",
            "data": data,
            "client": client,
            "interface": interface,
        },
        "sent_bytes_total": {
            "description": "Total bytes sent to peer",
            "data": data,
            "client": client,
            "interface": interface,
        }
    ],
}
```

This information will be processed in a prometheus format and will be served as
a response on the specified port (default `9200`).

The response will be like:

```text
# HELP pivpn_sent_bytes_total Bytes sent to peer
# TYPE pivpn_sent_bytes_total counter
pivpn_sent_bytes_total{interface="wg0",client="john_laptop"} 2628807520
pivpn_sent_bytes_total{interface="wg0",client="jane_phone"} 7583928584
pivpn_sent_bytes_total{interface="wg0",client="jane_laptop"} 0
pivpn_sent_bytes_total{interface="wg0",client="john_phone"} 69200412
# HELP pivpn_received_bytes_total Bytes received from peer
# TYPE pivpn_received_bytes_total counter
pivpn_received_bytes_total{interface="wg0",client="john_laptop"} 209554952
pivpn_received_bytes_total{interface="wg0",client="jane_phone"} 418109920
pivpn_received_bytes_total{interface="wg0",client="jane_laptop"} 0
pivpn_received_bytes_total{interface="wg0",client="john_phone"} 44178876
# HELP pivpn_since_last_handshake Seconds passed since the last handshake
# TYPE pivpn_since_last_handshake gauge
pivpn_since_last_handshake_seconds{interface="wg0",client="john_laptop"} 93
pivpn_since_last_handshake_seconds{interface="wg0",client="jane_phone"} 77
pivpn_since_last_handshake_seconds{interface="wg0",client="jane_laptop"} 1745327154
pivpn_since_last_handshake_seconds{interface="wg0",client="john_phone"} 604
# HELP pivpn_last_handshake Seconds registered last handshake
# TYPE pivpn_last_handshake gauge
pivpn_last_handshake_seconds{interface="wg0",client="john_laptop"} 1745327061
pivpn_last_handshake_seconds{interface="wg0",client="jane_phone"} 1745327077
pivpn_last_handshake_seconds{interface="wg0",client="jane_laptop"} 0
pivpn_last_handshake_seconds{interface="wg0",client="john_phone"} 1745326550
```

## Acknowledgements

This exporter was inspired by the work done on [prometheus_wireguard_exporter](https://github.com/MindFlavor/prometheus_wireguard_exporter).
