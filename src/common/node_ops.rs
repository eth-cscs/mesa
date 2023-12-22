use serde_json::Value;

pub fn nodes_to_string_format_one_line(nodes: Option<&Vec<Value>>) -> String {
    if let Some(nodes_content) = nodes {
        nodes_to_string_format_discrete_columns(nodes, nodes_content.len() + 1)
    } else {
        "".to_string()
    }
}

pub fn nodes_to_string_format_discrete_columns(
    nodes: Option<&Vec<Value>>,
    num_columns: usize,
) -> String {
    let mut members: String;

    match nodes {
        Some(nodes) if !nodes.is_empty() => {
            members = nodes[0].as_str().unwrap().to_string(); // take first element

            for (i, _) in nodes.iter().enumerate().skip(1) {
                // iterate for the rest of the list
                if i % num_columns == 0 {
                    // breaking the cell content into multiple lines (only 2 xnames per line)

                    members.push_str(",\n");
                } else {
                    members.push(',');
                }

                members.push_str(nodes[i].as_str().unwrap());
            }
        }
        _ => members = "".to_string(),
    }

    members
}

pub fn string_vec_to_multi_line_string(nodes: Option<&Vec<String>>, num_columns: usize) -> String {
    let mut members: String;

    match nodes {
        Some(nodes) if !nodes.is_empty() => {
            members = nodes.first().unwrap().to_string(); // take first element

            for (i, _) in nodes.iter().enumerate().skip(1) {
                // iterate for the rest of the list
                if i % num_columns == 0 {
                    // breaking the cell content into multiple lines (only 2 xnames per line)

                    members.push_str(",\n");
                } else {
                    members.push(',');
                }

                members.push_str(&nodes[i]);
            }
        }
        _ => members = "".to_string(),
    }

    members
}
