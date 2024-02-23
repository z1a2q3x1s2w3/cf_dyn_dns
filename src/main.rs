use error_chain::error_chain;
use clap::{arg, command};
use std::net::ToSocketAddrs;
use serde_json::json;
use serde_json::Value;
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
		Json(serde_json::Error);
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

fn get_dns_address(domain: &String) -> Result<String> {
	let socket_address = String::from(domain) + ":443";
	let mut ip_addresses = socket_address.to_socket_addrs()?;
	if let Some(addr) = ip_addresses.next() {
		Ok(addr.ip().to_string())
	} else {
		Err("No IP address found for the domain".into())
	}
}

struct CfApiClient {
	domain: String,
	zone_id: String,
	dns_record_id: String,
	token: String,
}

impl CfApiClient {
	fn update_dns_record(&self, ip: &String) -> Result<()> {
		// https://developers.cloudflare.com/api/operations/dns-records-for-a-zone-update-dns-record
		let update_dns_record_url = String::from("https://api.cloudflare.com/client/v4/zones/") + &self.zone_id + "/dns_records/" + &self.dns_record_id;
		let http_client = reqwest::blocking::Client::builder()
			.https_only(true)
			.build()?;
		let body = json!({
			"content": ip,
			"name": &self.domain,
			"type": "A",
		});
		let response = http_client
			.patch(update_dns_record_url)
			.bearer_auth(&self.token)
			.body(body.to_string())
			.send()?
			.text()?;
		let parsed: Value = serde_json::from_str(&response)?;
		if parsed["success"].as_bool().unwrap() {
			Ok(())
		} else {
			Err("DNS record update was not successful".into())
		}
	}
}

fn build_cf_api_client(domain: &String, zone_id: &String, token: &String) -> Result<CfApiClient> {
	// https://developers.cloudflare.com/api/operations/dns-records-for-a-zone-list-dns-records
	let get_dns_record_url = String::from("https://api.cloudflare.com/client/v4/zones/") + zone_id + "/dns_records";
	let http_client = reqwest::blocking::Client::builder()
		.https_only(true)
		.build()?;
	let response = http_client
		.get(get_dns_record_url)
		.bearer_auth(token)
		.send()?
		.text()?;
	let parsed: Value = serde_json::from_str(&response)?;
	let mut dns_record_id = String::new();
	for i in 0..parsed["result"].as_array().unwrap().len() {
		if parsed["result"][i]["name"].as_str().unwrap() == domain {
			dns_record_id = parsed["result"][i]["id"].as_str().unwrap().to_string();
		}
	}
	Ok(CfApiClient {
		domain: domain.to_string(),
		zone_id: zone_id.to_string(),
		dns_record_id: dns_record_id,
		token: token.to_string(),
	})
}

fn main() -> Result<()> {
	let matches = command!()
		.arg(arg!(-d --domain <URL> "The domain to be updated").required(true))
		.arg(arg!(-z --zone_id <ID> "CloudFlare Zone ID").required(true))
		.arg(arg!(-t --token <TOKEN> "CloudFlare API token").required(true))
		.get_matches();
	
	let wan_address = get_wan_address()?;
	println!("WAN IP = {:?}", wan_address);
	
	let dns_address = get_dns_address(matches.get_one::<String>("domain").expect("domain is required"))?;
	println!("DNS IP = {:?}", dns_address);
	
	if wan_address != dns_address {
		let cf_api_client = build_cf_api_client(matches.get_one::<String>("domain").expect("domain is required"), matches.get_one::<String>("zone_id").expect("zone_id is required"), matches.get_one::<String>("token").expect("token is required"))?;
		cf_api_client.update_dns_record(&wan_address)?;
	}
	
	Ok(())
}
