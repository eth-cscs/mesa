use comfy_table::Table;
use serde_json::Value;

use crate::shasta::cfs::configuration::r#struct::get_put_payload::CfsConfigurationResponse;

pub fn print_table(cfs_configurations: Vec<Value>) {
    let mut table = Table::new();

    table.set_header(vec!["Name", "Last updated", "Layers"]);

    for cfs_configuration in cfs_configurations {
        let mut layers: String = String::new();

        if cfs_configuration["layers"].as_array().is_some() {
            let layers_json = cfs_configuration["layers"].as_array().unwrap();

            layers = format!(
                "COMMIT: {} NAME: {}",
                layers_json[0]["commit"], layers_json[0]["name"]
            );

            for layer in layers_json.iter().skip(1) {
                layers = format!(
                    "{}\nCOMMIT: {} NAME: {}",
                    layers, layer["commit"], layer["name"]
                );
            }
        }

        table.add_row(vec![
            cfs_configuration["name"].as_str().unwrap(),
            cfs_configuration["lastUpdated"].as_str().unwrap(),
            &layers,
        ]);
    }

    println!("{table}");
}

pub fn print_table_struct(cfs_configurations: Vec<CfsConfigurationResponse>) {
    let mut table = Table::new();

    table.set_header(vec!["Name", "Last updated", "Layers"]);

    for cfs_configuration in cfs_configurations {
        let mut layers: String = String::new();

        if !cfs_configuration.layers.is_empty() {
            let layers_json = cfs_configuration.layers;

            layers = format!(
                "COMMIT: {} NAME: {}",
                layers_json[0].commit.as_ref().unwrap(),
                layers_json[0].name
            );

            for layer in layers_json.iter().skip(1) {
                layers = format!(
                    "{}\nCOMMIT: {} NAME: {}",
                    layers,
                    layer.commit.as_ref().unwrap(),
                    layer.name
                );
            }
        }

        table.add_row(vec![
            cfs_configuration.name,
            cfs_configuration.last_updated,
            layers,
        ]);
    }

    println!("{table}");
}
