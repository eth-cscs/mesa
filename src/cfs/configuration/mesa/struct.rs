// use comfy_table::Table;
use std::fmt;

pub struct Configuration {
    pub name: String,
    pub last_updated: String,
    pub config_layers: Vec<Layer>,
}

impl Configuration {
    pub fn new(name: &str, last_updated: &str, config_layers: Vec<Layer>) -> Self {
        Self {
            name: String::from(name),
            last_updated: String::from(last_updated),
            config_layers,
        }
    }
}

impl fmt::Display for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\nConfig Details:\n - name: {}\n - last updated: {}\nLayers:",
            self.name, self.last_updated
        )?;

        for (i, config_layer) in self.config_layers.iter().enumerate() {
            write!(f, "\n Layer {}:{}", i, config_layer)?;
        }

        Ok(())
    }
}

pub struct Layer {
    pub name: String,
    pub repo_name: String,
    pub commit_id: String,
    pub author: String,
    pub commit_date: String,
}

impl Layer {
    pub fn new(
        name: &str,
        repo_name: &str,
        commit_id: &str,
        author: &str,
        commit_date: &str,
    ) -> Self {
        Self {
            name: String::from(name),
            repo_name: String::from(repo_name),
            commit_id: String::from(commit_id),
            author: String::from(author),
            commit_date: String::from(commit_date),
        }
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\n - name: {}\n - repo name: {}\n - commit id: {}\n - commit date: {}\n - author: {}",
            self.name, self.repo_name, self.commit_id, self.commit_date, self.author
        )
    }
}
