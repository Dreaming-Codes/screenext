use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
pub struct Args {
    /// IP address of the device
    #[arg(short = 'H', long, value_name = "HOST")]
    pub host: Option<String>,
    #[arg(short, long, value_name = "PAIRING_FILE")]
    pub pairing_file: Option<String>,
    #[arg(short, long, value_name = "UDID")]
    pub udid: Option<String>,
    #[arg(short, long, value_name = "PORT", default_value_t = 12345)]
    pub app_port: u16,
}
