// Copyright 2016 The Grin Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Main for building the binary of a Grin peer-to-peer node.

extern crate blake2_rfc as blake2;
extern crate clap;
extern crate daemonize;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate slog;

extern crate grin_api as api;
extern crate grin_config as config;
extern crate grin_core as core;
extern crate grin_grin as grin;
extern crate grin_keychain as keychain;
extern crate grin_util as util;
extern crate grin_wallet as wallet;

use std::thread;
use std::io::Read;
use std::fs::File;
use std::time::Duration;
use std::env::current_dir;

use clap::{App, Arg, ArgMatches, SubCommand};
use daemonize::Daemonize;

use config::GlobalConfig;
use wallet::WalletConfig;
use core::global;
use util::{init_logger, LoggingConfig, LOGGER};

fn start_from_config_file(mut global_config: GlobalConfig) {
	info!(
		LOGGER,
		"Starting the Grin server from configuration file at {}",
		global_config.config_file_path.unwrap().to_str().unwrap()
	);

	global::set_mining_mode(
		global_config
			.members
			.as_mut()
			.unwrap()
			.server
			.clone()
			.chain_type,
	);

	grin::Server::start(global_config.members.as_mut().unwrap().server.clone()).unwrap();
	loop {
		thread::sleep(Duration::from_secs(60));
	}
}

fn main() {
	// First, load a global config object,
 // then modify that object with any switches
 // found so that the switches override the
 // global config file

	// This will return a global config object,
 // which will either contain defaults for all // of the config structures or a
 // configuration
 // read from a config file

	let mut global_config = GlobalConfig::new(None).unwrap_or_else(|e| {
		panic!("Error parsing config file: {}", e);
	});

	if global_config.using_config_file {
		// initialise the logger
		init_logger(global_config.members.as_mut().unwrap().logging.clone());
		info!(
			LOGGER,
			"Using configuration file at: {}",
			global_config
				.config_file_path
				.clone()
				.unwrap()
				.to_str()
				.unwrap()
		);
		global::set_mining_mode(
			global_config
				.members
				.as_mut()
				.unwrap()
				.server
				.clone()
				.chain_type,
		);
	} else {
		init_logger(Some(LoggingConfig::default()));
	}

	let args = App::new("Grin")
		.version("0.1")
		.author("The Grin Team")
		.about("Lightweight implementation of the MimbleWimble protocol.")

    // specification of all the server commands and options
    .subcommand(SubCommand::with_name("server")
                .about("Control the Grin server")
                .arg(Arg::with_name("port")
                     .short("p")
                     .long("port")
                     .help("Port to start the P2P server on")
                     .takes_value(true))
                .arg(Arg::with_name("api_port")
                     .short("a")
                     .long("api_port")
                     .help("Port on which to start the api server (e.g. transaction pool api)")
                     .takes_value(true))
                .arg(Arg::with_name("seed")
                     .short("s")
                     .long("seed")
                     .help("Override seed node(s) to connect to")
                     .takes_value(true))
                .arg(Arg::with_name("mine")
                     .short("m")
                     .long("mine")
                     .help("Starts the debugging mining loop"))
                .arg(Arg::with_name("wallet_url")
                     .short("w")
                     .long("wallet_url")
                     .help("A listening wallet receiver to which mining rewards will be sent")
                .takes_value(true))
                .subcommand(SubCommand::with_name("start")
                            .about("Start the Grin server as a daemon"))
                .subcommand(SubCommand::with_name("stop")
                            .about("Stop the Grin server daemon"))
                .subcommand(SubCommand::with_name("run")
                            .about("Run the Grin server in this console")))

    // specification of all the client commands and options
    .subcommand(SubCommand::with_name("client")
                .about("Communicates with the Grin server")
                .subcommand(SubCommand::with_name("status")
                            .about("current status of the Grin chain")))

	// specification of the wallet commands and options
	.subcommand(SubCommand::with_name("wallet")
		.about("Wallet software for Grin")
		.arg(Arg::with_name("pass")
			.short("p")
			.long("pass")
			.help("Wallet passphrase used to generate the private key seed")
			.takes_value(true)
			.default_value("mimblewimble"))
		.arg(Arg::with_name("data_dir")
			.short("dd")
			.long("data_dir")
			.help("Directory in which to store wallet files (defaults to current \
			directory)")
			.takes_value(true))
		.arg(Arg::with_name("port")
			.short("r")
			.long("port")
			.help("Port on which to run the wallet receiver when in receiver mode")
			.takes_value(true))
		.arg(Arg::with_name("external")
			.short("e")
			.long("external")
			.help("Listen on 0.0.0.0 interface to allow external connections (default is 127.0.0.1)")
			.takes_value(false))
		.arg(Arg::with_name("show_spent")
			.short("s")
			.long("show_spent")
			.help("Show spent outputs on wallet output command")
			.takes_value(false))
		.arg(Arg::with_name("api_server_address")
			.short("a")
			.long("api_server_address")
			.help("Api address of running node on which to check inputs and post transactions")
			.takes_value(true))

		.subcommand(SubCommand::with_name("receive")
			.about("Run the wallet in receiving mode. If an input file is \
				provided, will process it, otherwise runs in server mode waiting \
				for send requests.")
			.arg(Arg::with_name("input")
				.help("Partial transaction to receive, expects as a JSON file.")
				.short("i")
				.long("input")
				.takes_value(true)))

		.subcommand(SubCommand::with_name("send")
			.about("Builds a transaction to send someone some coins. By default, \
				the transaction will just be printed to stdout. If a destination is \
				provided, the command will attempt to contact the receiver at that \
				address and send the transaction directly.")
			.arg(Arg::with_name("amount")
				.help("Number of coins to send with optional fraction, e.g. 12.423")
				.index(1))
			.arg(Arg::with_name("minimum_confirmations")
				.help("Minimum number of confirmations required for an output to be spendable.")
				.short("c")
				.long("min_conf")
				.default_value("1")
				.takes_value(true))
			.arg(Arg::with_name("selection_strategy")
				.help("Coin/Output selection strategy.")
				.short("s")
				.long("selection")
				.possible_values(&["all", "smallest"])
				.default_value("all")
				.takes_value(true))
			.arg(Arg::with_name("dest")
				.help("Send the transaction to the provided server")
				.short("d")
				.long("dest")
				.takes_value(true)))

		.subcommand(SubCommand::with_name("burn")
			.about("** TESTING ONLY ** Burns the provided amount to a known \
				key. Similar to send but burns an output to allow single-party \
				transactions.")
			.arg(Arg::with_name("amount")
				.help("Number of coins to burn")
				.index(1))
			.arg(Arg::with_name("minimum_confirmations")
				.help("Minimum number of confirmations required for an output to be spendable.")
				.short("c")
				.long("min_conf")
				.default_value("1")
				.takes_value(true)))

		.subcommand(SubCommand::with_name("outputs")
			.about("raw wallet info (list of outputs)"))

		.subcommand(SubCommand::with_name("info")
			.about("basic wallet contents summary"))

		.subcommand(SubCommand::with_name("init")
			.about("Initialize a new wallet seed file.")))

	.get_matches();

	match args.subcommand() {
		// server commands and options
		("server", Some(server_args)) => {
			server_command(server_args, global_config);
		}

		// client commands and options
		("client", Some(client_args)) => match client_args.subcommand() {
			("status", _) => {
				println!("status info...");
			}
			_ => panic!("Unknown client command, use 'grin help client' for details"),
		},

		// client commands and options
		("wallet", Some(wallet_args)) => {
			wallet_command(wallet_args);
		}

		// If nothing is specified, try to just use the config file instead
		// this could possibly become the way to configure most things
		// with most command line options being phased out
		_ => {
			if global_config.using_config_file {
				start_from_config_file(global_config);
			} else {
				// won't attempt to just start with defaults,
	// and will reject
				println!("Unknown command, and no configuration file was found.");
				println!("Use 'grin help' for a list of all commands.");
			}
		}
	}
}

/// Handles the server part of the command line, mostly running, starting and
/// stopping the Grin blockchain server. Processes all the command line
/// arguments
/// to build a proper configuration and runs Grin with that configuration.
fn server_command(server_args: &ArgMatches, global_config: GlobalConfig) {
	info!(LOGGER, "Starting the Grin server...");

	// just get defaults from the global config
	let mut server_config = global_config.members.unwrap().server;

	if let Some(port) = server_args.value_of("port") {
		server_config.p2p_config.as_mut().unwrap().port = port.parse().unwrap();
	}

	if let Some(api_port) = server_args.value_of("api_port") {
		let default_ip = "0.0.0.0";
		server_config.api_http_addr = format!("{}:{}", default_ip, api_port);
	}

	if server_args.is_present("mine") {
		server_config.mining_config.as_mut().unwrap().enable_mining = true;
	}

	if let Some(wallet_url) = server_args.value_of("wallet_url") {
		server_config
			.mining_config
			.as_mut()
			.unwrap()
			.wallet_receiver_url = wallet_url.to_string();
	}

	if let Some(seeds) = server_args.values_of("seed") {
		server_config.seeding_type = grin::Seeding::List;
		server_config.seeds = Some(seeds.map(|s| s.to_string()).collect());
	}

	// start the server in the different run modes (interactive or daemon)
	match server_args.subcommand() {
		("run", _) => {
			grin::Server::start(server_config).unwrap();
			loop {
				thread::sleep(Duration::from_secs(60));
			}
		}
		("start", _) => {
			let daemonize = Daemonize::new()
				.pid_file("/tmp/grin.pid")
				.chown_pid_file(true)
				.working_directory(current_dir().unwrap())
				.privileged_action(move || {
					grin::Server::start(server_config.clone()).unwrap();
					loop {
						thread::sleep(Duration::from_secs(60));
					}
				});
			match daemonize.start() {
				Ok(_) => info!(LOGGER, "Grin server succesfully started."),
				Err(e) => error!(LOGGER, "Error starting: {}", e),
			}
		}
		("stop", _) => println!("TODO, just 'kill $pid' for now."),
		(cmd, _) => {
      println!(":: {:?}", server_args);
      panic!("Unknown server command '{}', use 'grin help server' for details", cmd);
    }
	}
}

fn wallet_command(wallet_args: &ArgMatches) {
	let mut wallet_config = WalletConfig::default();

	if let Some(port) = wallet_args.value_of("port") {
		wallet_config.api_listen_port = port.to_string();
	}

	if wallet_args.is_present("external") {
		wallet_config.api_listen_interface = "0.0.0.0".to_string();
	}

	if let Some(dir) = wallet_args.value_of("dir") {
		wallet_config.data_file_dir = dir.to_string().clone();
	}

	if let Some(sa) = wallet_args.value_of("api_server_address") {
		wallet_config.check_node_api_http_addr = sa.to_string().clone();
	}

	let mut show_spent=false;
	if wallet_args.is_present("show_spent") {
		show_spent=true;
	}

	// Derive the keychain based on seed from seed file and specified passphrase.
	// Generate the initial wallet seed if we are running "wallet init".
	if let ("init", Some(_)) = wallet_args.subcommand() {
		wallet::WalletSeed::init_file(&wallet_config).expect("Failed to init wallet seed file.");

		// we are done here with creating the wallet, so just return
		return;
	}

	let wallet_seed =
		wallet::WalletSeed::from_file(&wallet_config).expect("Failed to read wallet seed file.");
	let passphrase = wallet_args
		.value_of("pass")
		.expect("Failed to read passphrase.");
	let keychain = wallet_seed
		.derive_keychain(&passphrase)
		.expect("Failed to derive keychain from seed file and passphrase.");

	match wallet_args.subcommand() {
		("receive", Some(receive_args)) => if let Some(f) = receive_args.value_of("input") {
			let mut file = File::open(f).expect("Unable to open transaction file.");
			let mut contents = String::new();
			file.read_to_string(&mut contents)
				.expect("Unable to read transaction file.");
			wallet::receive_json_tx_str(&wallet_config, &keychain, contents.as_str()).unwrap();
		} else {
			wallet::server::start_rest_apis(wallet_config, keychain);
		},
		("send", Some(send_args)) => {
			let amount = send_args
				.value_of("amount")
				.expect("Amount to send required");
			let amount = core::core::amount_from_hr_string(amount)
				.expect("Could not parse amount as a number with optional decimal point.");
			let minimum_confirmations: u64 = send_args
				.value_of("minimum_confirmations")
				.unwrap()
				.parse()
				.expect("Could not parse minimum_confirmations as a whole number.");
			let selection_strategy = send_args
				.value_of("selection_strategy")
				.expect("Selection strategy required");
			let mut dest = "stdout";
			if let Some(d) = send_args.value_of("dest") {
				dest = d;
			}
			let max_outputs = 500;
			let result=wallet::issue_send_tx(
				&wallet_config,
				&keychain,
				amount,
				minimum_confirmations,
				dest.to_string(),
				max_outputs,
				(selection_strategy == "all"),
			);
			match result {
				Ok(_) => {}, //success messaged logged internally
				Err(wallet::Error::NotEnoughFunds(_)) => {},
				Err(e) => panic!(e),
			};
		}
		("burn", Some(send_args)) => {
			let amount = send_args
				.value_of("amount")
				.expect("Amount to burn required");
			let amount = core::core::amount_from_hr_string(amount)
				.expect("Could not parse amount as number with optional decimal point.");
			let minimum_confirmations: u64 = send_args
				.value_of("minimum_confirmations")
				.unwrap()
				.parse()
				.expect("Could not parse minimum_confirmations as a whole number.");
			let max_outputs = 500;
			wallet::issue_burn_tx(
				&wallet_config,
				&keychain,
				amount,
				minimum_confirmations,
				max_outputs,
			).unwrap();
		}
		("info", Some(_)) => {
			wallet::show_info(&wallet_config, &keychain);
		}
		("outputs", Some(_)) => {
			wallet::show_outputs(&wallet_config, &keychain, show_spent);
		}
		_ => panic!("Unknown wallet command, use 'grin help wallet' for details"),
	}
}
