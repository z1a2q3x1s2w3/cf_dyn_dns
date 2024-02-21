use error_chain::error_chain;
use clap::{arg, command};
use std::net::ToSocketAddrs;
// Features
// Use checkip.amazonaws.com to get WAN IP address
// Use CloudFlare API to update DNS record
// Include logging functionality
// Include command line argument parsing
	// Switches for domain name and API token

// Outline:
// wan_address = get_wan_address()
// dns_address = get_dns_address()
// if wan_address != dns_address
	// update_dns_record

error_chain! {
	foreign_links {
		Io(std::io::Error);
		HttpRequest(reqwest::Error);
	}
}

fn get_wan_address() -> Result<String> {
	let ip_reflect_url = "https://checkip.amazonaws.com";
	let body = reqwest::blocking::get(ip_reflect_url)?
		.text()?;
	Ok(body
		.trim()
		.to_string())
}

fn get_dns_address(domain: &str) -> Result<String> {
	let socket_address = String::from(domain) + ":443";
	let mut ip_addresses = socket_address.to_socket_addrs()?;
	if let Some(addr) = ip_addresses.next() {
		Ok(addr.ip().to_string())
	} else {
		Err("No IP address found for the domain".into())
	}
}

fn main() -> Result<()> {
	let matches = command!()
//		.author("zaqxsw")
//		.about("CloudFlare dynamic DNS")
		.arg(arg!(-d --domain <URL> "The domain to be updated").required(true))
		.get_matches();
	
	let wan_address = get_wan_address()?;
	println!("WAN IP = {:?}", wan_address);
	
	let dns_address = get_dns_address(matches.get_one::<String>("domain").expect("required"))?;
	println!("DNS IP = {:?}", dns_address);
	
	Ok(())
}
