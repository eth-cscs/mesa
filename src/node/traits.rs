use std::error::Error;

use serde_json::Value;

pub trait Node {
    /// Shuts down a node
    fn power_off() -> Result<(), Box<dyn Error>>;
    // Start a node
    fn power_on() -> Result<(), Box<dyn Error>>;
    /// Restart a node
    fn reset() -> Result<(), Box<dyn Error>>;
    /// Get node's power status
    fn get_power_status() -> Result<String, Box<dyn Error>>;
    /// Connect to node's console
    fn connect_to_console() -> Result<(), Box<dyn Error>>;
    /// Get CFS configuration name related to the image used to boot the node
    fn get_boot_config() -> Option<String>;
    /// Get CFS configuration assigned to configure the node
    fn get_desired_config() -> Option<String>;
    /// Get node status (OFF, BOOTING, CONFIGURING, STANDBY)
    fn get_status() -> Result<String, Box<dyn Error>>;
    /// Get node's details like:
    /// CFS configuration used to create boot image
    /// CFS configuration to configure the node
    /// Power status
    /// If node is configured
    /// Current CFS session and current layer running (if any)
    fn get_details() -> Result<Value, Box<dyn Error>>;
    /// Download boot image
    fn download_boot_image(); // TODO
}

pub trait Cluster {
    /// Shuts down all nodes of a cluster
    fn power_off() -> Result<(), Box<dyn Error>>;
    /// Start all nodes of a cluster
    fn power_on() -> Result<(), Box<dyn Error>>;
    /// Restarts all nodes of a cluster
    fn reset() -> Result<(), Box<dyn Error>>;
    /// Get power state for all nodes in a cluster
    fn get_power_state() -> Result<(), Box<dyn Error>>;
    /// Get all CFS configuration related to each node of a cluster
    fn get_boot_config() -> Option<Vec<(String, String)>>;
    /// Get CFS configurations related to each node of a cluster
    fn get_desired_config() -> Option<Vec<(String, String)>>;
    /// Get overall cluster status (OFF, BOOTING, CONFIGURING, STANDBY)
    fn get_status() -> Result<String, Box<dyn Error>>;
    /// Get cluster details
    fn get_details() -> Result<Value, Box<dyn Error>>;
    /// Migrate cluster
    fn migrate() -> Result<(), Box<dyn Error>>;
}
