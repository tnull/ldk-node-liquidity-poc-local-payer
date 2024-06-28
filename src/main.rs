use std::io;
use std::io::prelude::*;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use ldk_node::bitcoin::secp256k1::PublicKey;
use ldk_node::lightning_invoice::Bolt11Invoice;
use ldk_node::{Builder, Config, Event, LogLevel};

use client::ServerHackClient;
use ldk_node::bitcoin::Network;
use protos::GetNodeIdRequest;

#[tokio::main]
async fn main() {
	let args: Vec<String> = std::env::args().collect();
	let mut config = Config::default();
	let client = ServerHackClient::new("127.0.0.1:3000".to_string());
	let node_id_req = GetNodeIdRequest {};
	let node_id_res = client.get_node_id(node_id_req).await.unwrap();
	let lsp_node_id = PublicKey::from_str(&node_id_res.node_id).unwrap();
	let lsp_address = "127.0.0.1:9735".parse().unwrap();
	config.trusted_peers_0conf = vec![lsp_node_id.clone()];
	config.network = Network::Signet;

	let mut builder = Builder::from_config(config);
	builder.set_storage_dir_path("/tmp/ldk_node_poc_sender/".to_string());
	builder.set_log_level(LogLevel::Trace);
	builder.set_esplora_server("https://mutinynet.com/api/".to_string());

	let node = Arc::new(builder.build().unwrap());
	node.start().unwrap();

	let event_node = Arc::clone(&node);
	std::thread::spawn(move || loop {
		let event = event_node.wait_next_event();
		match event {
			Event::ChannelReady { .. } => {
				let invoice = Bolt11Invoice::from_str(&args[1]).unwrap();
				event_node.bolt11_payment().send(&invoice).unwrap();
			},
			_ => println!("GOT NEW EVENT: {:?}", event),
		}
		event_node.event_handled();
	});

	println!("FUNDING_ADDRESS: {}", node.onchain_payment().new_address().unwrap());

	let needs_channel = node.list_channels().is_empty();
	if needs_channel {
		while node.list_balances().spendable_onchain_balance_sats < 1000000 {
			std::thread::sleep(Duration::from_secs(5));
		}
		node.connect_open_channel(lsp_node_id, lsp_address, 1000000, None, None, false).unwrap();
	}

	pause();

	std::thread::spawn(move || {
		node.stop().unwrap();
	});
}

fn pause() {
	let mut stdin = io::stdin();
	let mut stdout = io::stdout();

	// We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
	write!(stdout, "Press any key to continue...").unwrap();
	stdout.flush().unwrap();

	// Read a single byte and discard
	let _ = stdin.read(&mut [0u8]).unwrap();
}
