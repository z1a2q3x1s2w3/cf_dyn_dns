use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::error::Error;
use dotenv::dotenv;
use std::env;
use log::{debug, info, error};
use log4rs;

fn gen_https_client() -> Result<Client, Box<dyn Error>> {
	Client::builder()
		.https_only(true)
		.build()
		.map_err(|e| format!("Failed to create HTTPS client: {}", e).into())
}

fn get_wan_address(client: &Client) -> Result<String, Box<dyn Error>> {
	let cf_trace_url = "https://1.1.1.1/cdn-cgi/trace";
	debug!("Sending request to Cloudflare trace endpoint");
	let response = client
		.get(cf_trace_url)
		.send()
		.map_err(|e| format!("Failed to send request to Cloudflare: {}", e))?
		.text()
		.map_err(|e| format!("Failed to read response text: {}", e))?;
	debug!("Extracting WAN address from response");
	let wan_address = response
		.lines()
		.find_map(|line| {
			line.split_once('=')
				.filter(|&(key, _)| key == "ip")
				.map(|(_, value)| value.to_string())
		})
		.ok_or_else::<Box<dyn Error>, _>(|| "IP address not found in Cloudflare trace response".into())?;
	debug!("WAN address found: {}", wan_address);
	Ok(wan_address)
}

fn get_dns_address(client: &Client, domain: &str) -> Result<String, Box<dyn Error>> {
	let doh_url = "https://1.1.1.1/dns-query";
	debug!("Sending request to Cloudflare DoH service");
	let response = client
		.get(doh_url)
		.query(&[("name", domain), ("type", "A")])
		.header("accept", "application/dns-json")
		.send()
		.map_err(|e| format!("Failed to send DoH request: {}", e))?
		.text()
		.map_err(|e| format!("Failed to read DoH response text: {}", e))?;
	debug!("Parsing response as JSON");
	let parsed: Value = serde_json::from_str(&response)
		.map_err(|e| format!("Failed to parse DoH response JSON: {}", e))?;
	debug!("Extracting DNS address from parsed response");
	let dns_address = parsed["Answer"]
		.as_array()
		.and_then(|answers| answers.get(0))
		.and_then(|answer| answer["data"].as_str())
		.map(|data| data.trim_matches('"').to_string())
		.ok_or_else::<Box<dyn Error>, _>(|| format!("No IP address found for the {}", domain).into())?;
	debug!("DNS address found: {}", dns_address);
	Ok(dns_address)
}

struct CfApiClient {
	domain: String,
	zone_id: String,
	dns_record_id: String,
	token: String,
}

impl CfApiClient {
	fn update_dns_record(&self, client: &Client, ip: &str) -> Result<bool, Box<dyn Error>> {
		// https://developers.cloudflare.com/api/operations/dns-records-for-a-zone-update-dns-record
		debug!("Constructing URL for DNS record update request API");
		let update_dns_record_url = format!(
			"https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
			self.zone_id, self.dns_record_id
		);
		debug!("Constructing JSON body for patch request");
		let body = json!({
			"content": ip,
			"name": &self.domain,
			"type": "A",
		});
		debug!("Sending DNS record update request to API");
		let response = client
			.patch(&update_dns_record_url)
			.bearer_auth(&self.token)
			.json(&body)
			.send()
			.map_err(|e| format!("Failed to send DNS record update request: {}", e))?
			.text()
			.map_err(|e| format!("Failed to read DNS update response text: {}", e))?;
		debug!("Parsing response as JSON");
		let parsed: Value = serde_json::from_str(&response)
			.map_err(|e| format!("Failed to parse DNS record update response JSON: {}", e))?;
		debug!("Checking whether update was successful");
		if parsed["success"].as_bool().unwrap_or(false) {
			debug!("DNS record updated successfully");
			Ok(true)
		} else {
			error!("DNS record update failed: {:?}", parsed);
			Err("DNS record update was not successful".into())
		}
	}
}

fn build_cf_api_client(client: &Client, domain: &str, zone_id: &str, token: &str) -> Result<CfApiClient, Box<dyn Error>> {
	// https://developers.cloudflare.com/api/operations/dns-records-for-a-zone-list-dns-records
	debug!("Constructing URL for DNS record API");
	let get_dns_record_url = format!(
		"https://api.cloudflare.com/client/v4/zones/{}/dns_records",
		zone_id
	);
	debug!("Sending DNS record ID request to API");
	let response = client
		.get(&get_dns_record_url)
		.bearer_auth(token)
		.send()
		.map_err(|e| format!("Failed to send DNS record request: {}", e))?
		.text()
		.map_err(|e| format!("Failed to read DNS response text: {}", e))?;
	debug!("Parsing response as JSON");
	let parsed: Value = serde_json::from_str(&response)
		.map_err(|e| format!("Failed to parse DNS response JSON: {}", e))?;
	debug!("Extracting DNS record ID from parsed response");
	let dns_record_id = parsed["result"]
		.as_array()
		.and_then(|results| results.get(0))
		.and_then(|result| result["id"].as_str())
		.ok_or_else::<Box<dyn Error>, _>(|| format!("DNS record ID not found for {}", domain).into())?;
	debug!("DNS record ID found: {}", dns_record_id);
	Ok(CfApiClient {
		domain: domain.to_string(),
		zone_id: zone_id.to_string(),
		dns_record_id: dns_record_id.to_string(),
		token: token.to_string(),
	})
}

fn main() -> Result<(), Box<dyn Error>> {
	debug!("Application starting");
	debug!("Reading environment variables");
	dotenv().ok();
	debug!("Reading log config file");
	log4rs::init_file("log-config.yaml", Default::default())?;
	
	debug!("Setting up environment variables");
	let domain = env::var("DOMAIN").expect("DOMAIN environment variable is required");
	let zone_id = env::var("ZONE_ID").expect("ZONE_ID environment variable is required");
	let token = env::var("TOKEN").expect("TOKEN environment variable is required");
	
	debug!("Setting up HTTPS client");
	let client = gen_https_client()?;
	
	debug!("Getting WAN address");
	let wan_address = get_wan_address(&client)?;
	debug!("Getting DNS address");
	let dns_address = get_dns_address(&client, &domain)?;
	
	debug!("Compare WAN and DNS addresses");
	if wan_address != dns_address {
		info!("WAN address ({}) does not match DNS address ({})", wan_address, dns_address);
		debug!("Setting up Cloudflare API client");
		let cf_api_client = build_cf_api_client(&client, &domain, &zone_id, &token)?;
		if cf_api_client.update_dns_record(&client, &wan_address)? {
			info!("Address successfully updated to: {}", wan_address);
		}
	}
	
	debug!("Application finished");
	Ok(())
}
