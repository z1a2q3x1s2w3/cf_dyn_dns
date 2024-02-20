
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

fn get_wan_address() -> String {
	String::from("1.2.3.4")
}

fn main() {
	println!("{}", get_wan_address());
}
