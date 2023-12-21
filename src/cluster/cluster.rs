pub trait Node {
    /// Shuts down a node
    pub fn power_off() -> Result<(), dyn Err>;
    // Start a node
    pub fn power_on() -> Result<(), dyn Err>;
    /// Restart a node
    pub fn reset() -> Result<(), dyn Err>;
    /// Get node's power status
    pub fn get_power_status() -> Result<String, dyn Err>;
    /// Connect to node's console
    pub fn connect_to_console() -> Result<(), dyn Err>;
    /// Get CFS configuration name related to the image used to boot the node
    pub fn get_boot_config() -> Option<String>;
    /// Get CFS configuration assigned to configure the node
    pub fn get_desired_config() -> Option<String>;
    /// Get node status (OFF, BOOTING, CONFIGURING, STANDBY)
    pub fn get_status() -> Result<String, dyn Err>;
    /// Get node's details like:
    /// CFS configuration used to create boot image
    /// CFS configuration to configure the node
    /// Power status
    /// If node is configured
    /// Current CFS session and current layer running (if any)
    pub fn get_details() -> Result<Value, dyn Err>;
    /// Download boot image
    pub fn download_boot_image(); // TODO
}

pub trait Cluster {
    /// Shuts down all nodes of a cluster
    pub fn power_off() -> Result<(), dyn Err>;
    /// Start all nodes of a cluster
    pub fn power_on() -> Result<(), dyn Err>;
    /// Restarts all nodes of a cluster
    pub fn reset() -> Result<(), dyn Err>;
    /// Get power state for all nodes in a cluster
    pub fn get_power_state() -> Resutl<(), dyn Err>;
    /// Get all CFS configuration related to each node of a cluster
    pub fn get_boot_config() -> Option<Vec<(String, String)>>;
    /// Get CFS configurations related to each node of a cluster
    pub fn get_desired_config() -> Option<Vec<(String, String)>>;
    /// Get overall cluster status (OFF, BOOTING, CONFIGURING, STANDBY)
    pub fn get_status() -> Result<String, dyn Err>;
    /// Get cluster details
    pub fn get_details() -> Result<Value, dyn Err>;
    /// Migrate cluster
    pub fn migrate() -> Result<(), dyn Err>;
}
