use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::error::Error;
use dotenv::dotenv;
use std::env;
use log::{info, error};
use log4rs;

fn gen_https_client() -> Result<Client, Box<dyn Error>> {
	Client::builder()
		.https_only(true)
		.build()
		.map_err(|e| format!("Failed to create HTTPS client: {}", e).into())
}

fn get_wan_address(client: &Client) -> Result<String, Box<dyn Error>> {
	info!("Fetching WAN address...");
	let cf_trace_url = "https://1.1.1.1/cdn-cgi/trace";
	let response = client
		.get(cf_trace_url)
		.send()
		.map_err(|e| format!("Failed to send request to Cloudflare: {}", e))?
		.text()
		.map_err(|e| format!("Failed to read response text: {}", e))?;
	let wan_address = response
		.lines()
		.find_map(|line| {
			line.split_once('=')
				.filter(|&(key, _)| key == "ip")
				.map(|(_, value)| value.to_string())
		})
		.ok_or_else::<Box<dyn Error>, _>(|| "IP address not found in Cloudflare trace response".into())?;
	info!("WAN address found: {}", wan_address);
	Ok(wan_address)
}

fn get_dns_address(client: &Client, domain: &str) -> Result<String, Box<dyn Error>> {
	info!("Fetching DNS address for {}", domain);
	let doh_url = "https://1.1.1.1/dns-query";
	let response = client
		.get(doh_url)
		.query(&[("name", domain), ("type", "A")])
		.header("accept", "application/dns-json")
		.send()
		.map_err(|e| format!("Failed to send DoH request: {}", e))?
		.text()
		.map_err(|e| format!("Failed to read DoH response text: {}", e))?;
	let parsed: Value = serde_json::from_str(&response)
		.map_err(|e| format!("Failed to parse DoH response JSON: {}", e))?;
	let dns_address = parsed["Answer"]
		.as_array()
		.and_then(|answers| answers.get(0))
		.and_then(|answer| answer["data"].as_str())
		.map(|data| data.trim_matches('"').to_string())
		.ok_or_else::<Box<dyn Error>, _>(|| format!("No IP address found for the {}", domain).into())?;
	info!("DNS address found: {}", dns_address);
	Ok(dns_address)
}

struct CfApiClient {
	domain: String,
	zone_id: String,
	dns_record_id: String,
	token: String,
}

impl CfApiClient {
	fn update_dns_record(&self, client: &Client, ip: &str) -> Result<(), Box<dyn Error>> {
		// https://developers.cloudflare.com/api/operations/dns-records-for-a-zone-update-dns-record
		info!("Updating DNS record for {} to {}", self.domain, ip);
		let update_dns_record_url = format!(
			"https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
			self.zone_id, self.dns_record_id
		);
		let body = json!({
			"content": ip,
			"name": &self.domain,
			"type": "A",
		});
		let response = client
			.patch(&update_dns_record_url)
			.bearer_auth(&self.token)
			.json(&body)
			.send()
			.map_err(|e| format!("Failed to send DNS record update request: {}", e))?
			.text()
			.map_err(|e| format!("Failed to read DNS update response text: {}", e))?;
		let parsed: Value = serde_json::from_str(&response)
			.map_err(|e| format!("Failed to parse DNS record update response JSON: {}", e))?;
		if parsed["success"].as_bool().unwrap_or(false) {
			info!("DNS record updated successfully");
			Ok(())
		} else {
			error!("DNS record update failed: {:?}", parsed);
			Err("DNS record update was not successful".into())
		}
	}
}

fn build_cf_api_client(client: &Client, domain: &str, zone_id: &str, token: &str) -> Result<CfApiClient, Box<dyn Error>> {
	// https://developers.cloudflare.com/api/operations/dns-records-for-a-zone-list-dns-records
	info!("Building Cloudflare API client...");
	let get_dns_record_url = format!(
		"https://api.cloudflare.com/client/v4/zones/{}/dns_records",
		zone_id
	);
	let response = client
		.get(&get_dns_record_url)
		.bearer_auth(token)
		.send()
		.map_err(|e| format!("Failed to send DNS record request: {}", e))?
		.text()
		.map_err(|e| format!("Failed to read DNS response text: {}", e))?;
	let parsed: Value = serde_json::from_str(&response)
		.map_err(|e| format!("Failed to parse DNS response JSON: {}", e))?;
	let dns_record_id = parsed["result"]
		.as_array()
		.and_then(|results| results.get(0))
		.and_then(|result| result["id"].as_str())
		.ok_or_else::<Box<dyn Error>, _>(|| format!("DNS record ID not found for {}", domain).into())?;
	info!("DNS record ID found: {}", dns_record_id);
	Ok(CfApiClient {
		domain: domain.to_string(),
		zone_id: zone_id.to_string(),
		dns_record_id: dns_record_id.to_string(),
		token: token.to_string(),
	})
}

fn main() -> Result<(), Box<dyn Error>> {
	dotenv().ok();
	//env_logger::init();
	log4rs::init_file("log4rs.yaml", Default::default())?;
	
	info!("Starting application...");
	
	let domain = env::var("DOMAIN").expect("DOMAIN environment variable is required");
	let zone_id = env::var("ZONE_ID").expect("ZONE_ID environment variable is required");
	let token = env::var("TOKEN").expect("TOKEN environment variable is required");
	
	let client = gen_https_client()?;
	
	let wan_address = get_wan_address(&client)?;
	let dns_address = get_dns_address(&client, &domain)?;
	
	if wan_address != dns_address {
		let cf_api_client = build_cf_api_client(&client, &domain, &zone_id, &token)?;
		cf_api_client.update_dns_record(&client, &wan_address)?;
	}
	
	info!("Application finished successfully");
	Ok(())
}
