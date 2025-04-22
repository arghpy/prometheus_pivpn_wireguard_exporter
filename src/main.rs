use argh::FromArgs;
use serde_json::json;
use std::{
    collections::HashMap, error::Error, fs, io, path::Path, process::Command, time::SystemTime,
};
use tiny_http::{Header, Response, Server};

const WIREGUARD_KEYS: &str = "/etc/wireguard/keys";
const DEFAULT_INTERFACE: &str = "wg0";
const DEFAULT_PORT: i32 = 9200;

fn default_interface() -> String {
    println!("Default interface will be used: {}", DEFAULT_INTERFACE);
    DEFAULT_INTERFACE.to_string()
}

fn default_port() -> i32 {
    println!("Default port will be used: {}", DEFAULT_PORT);
    DEFAULT_PORT
}

#[derive(FromArgs)]
/// Export wireguard metrics from PiVPN's default implementation
#[argh(help_triggers("-h", "--help"))]
struct Args {
    /// wireguard interface to monitor
    #[argh(option, short = 'i', default = "default_interface()")]
    interface: String,
    /// port to listen on
    #[argh(option, short = 'p', default = "default_port()")]
    port: i32,
}

fn main() {
    let args: Args = argh::from_env();

    let server = Server::http(format!("[::]:{}", &args.port)).unwrap();
    println!("Listening on http://[::]:{}", &args.port);

    for request in server.incoming_requests() {
        if request.url() == "/metrics" {
            match handle_metrics(&args.interface) {
                Ok(body) => {
                    let response = Response::from_string(body)
                        .with_header("Content-Type: text/plain".parse::<Header>().unwrap());
                    request.respond(response).unwrap();
                }
                Err(e) => {
                    let response =
                        Response::from_string(format!("Error: {}", e)).with_status_code(500);
                    request.respond(response).unwrap();
                }
            }
        } else {
            let response = Response::from_string("Not Found").with_status_code(404);
            request.respond(response).unwrap();
        }
    }
}

fn handle_metrics(interface: &String) -> Result<String, Box<dyn Error>> {
    let clients = gather_clients()?;
    let wg_data = process_wg_dump(&interface, clients)?;

    let mut text: Vec<String> = Vec::new();

    text.push("# HELP pivpn_sent_bytes_total Bytes sent to peer".to_string());
    text.push("# TYPE pivpn_sent_bytes_total counter".to_string());
    for peer in &wg_data {
        let metric = format!(
            "pivpn_sent_bytes_total{{interface={},client={}}} {:?}",
            peer["sent_bytes_total"]["interface"],
            peer["sent_bytes_total"]["client"],
            serde_json::from_value::<i64>(peer["sent_bytes_total"]["data"].clone())?
        );
        text.push(metric);
    }

    text.push("# HELP pivpn_received_bytes_total Bytes received from peer".to_string());
    text.push("# TYPE pivpn_received_bytes_total counter".to_string());
    for peer in &wg_data {
        let metric = format!(
            "pivpn_received_bytes_total{{interface={},client={}}} {:?}",
            peer["received_bytes_total"]["interface"],
            peer["received_bytes_total"]["client"],
            serde_json::from_value::<i64>(peer["received_bytes_total"]["data"].clone())?
        );
        text.push(metric);
    }

    text.push(
        "# HELP pivpn_since_last_handshake Seconds passed since the last handshake".to_string(),
    );
    text.push("# TYPE pivpn_since_last_handshake gauge".to_string());
    for peer in &wg_data {
        let metric = format!(
            "pivpn_since_last_handshake_seconds{{interface={},client={}}} {:?}",
            peer["since_last_handshake"]["interface"],
            peer["since_last_handshake"]["client"],
            serde_json::from_value::<i64>(peer["since_last_handshake"]["data"].clone())?,
        );
        text.push(metric);
    }

    text.push("# HELP pivpn_last_handshake Seconds registered last handshake".to_string());
    text.push("# TYPE pivpn_last_handshake gauge".to_string());
    for peer in &wg_data {
        let metric = format!(
            "pivpn_last_handshake_seconds{{interface={},client={}}} {:?}",
            peer["last_handshake"]["interface"],
            peer["last_handshake"]["client"],
            serde_json::from_value::<i64>(peer["last_handshake"]["data"].clone())?,
        );
        text.push(metric);
    }
    Ok(text.join("\n"))
}

fn gather_clients() -> Result<HashMap<String, String>, io::Error> {
    let entries = fs::read_dir(Path::new(WIREGUARD_KEYS))?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;

    let mut clients: HashMap<String, String> = HashMap::new();
    for entry in entries {
        if entry.to_str().unwrap().ends_with("_pub") {
            let pub_key: String = fs::read_to_string(&entry).unwrap().trim().to_string();
            let client = entry
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .strip_suffix("_pub")
                .unwrap_or("unknown");
            clients.insert(pub_key.to_string(), client.to_string());
        }
    }
    Ok(clients)
}

fn process_wg_dump(
    interface: &String,
    clients: HashMap<String, String>,
) -> Result<Vec<serde_json::Value>, Box<dyn Error>> {
    let wg_show = Command::new("wg")
        .arg("show")
        .arg(format!("{}", interface).as_str())
        .arg("dump")
        .output()?;

    let mut binding = String::from_utf8(wg_show.stdout).unwrap();
    for (key, client) in clients {
        binding = binding.replace(&key.to_string(), &client.to_string());
    }
    let mut wg_lines: Vec<&str> = binding.lines().collect();
    wg_lines.remove(0); // remove self

    let mut prometheus_data: Vec<serde_json::Value> = Vec::new();
    for line in wg_lines {
        let data = line.split("\t").collect::<Vec<&str>>();

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i32;
        let last_handshake = data[4].parse::<i32>().unwrap();

        prometheus_data.push(json!({
            "last_handshake": {
                "description": "Time of the last handshake since epoch",
                "data": last_handshake,
                "client": data[0],
                "interface": interface,
            },
            "since_last_handshake": {
                "description": "Seconds passed since the last handshake",
                "data": now - last_handshake,
                "client": data[0],
                "interface": interface,
            },
            "received_bytes_total": {
                "description": "Total bytes received from peer",
                "data": data[5].parse::<i64>().unwrap(),
                "client": data[0],
                "interface": interface,
            },
            "sent_bytes_total": {
                "description": "Total bytes sent to peer",
                "data": data[6].parse::<i64>().unwrap(),
                "client": data[0],
                "interface": interface,
            }
        }));
    }

    Ok(prometheus_data)
}
