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

pub struct VCluster {

    pub name: String,
    pub description: String,
}
