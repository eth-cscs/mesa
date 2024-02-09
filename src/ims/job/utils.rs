use std::io::{self, Write};

use serde_json::Value;

use crate::ims::{self};

/// Wait an IMS job to finish
pub async fn wait_ims_job_to_finish(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    ims_job_id: &str,
) {
    let mut i = 0;
    let max = 1800; // Max ammount of attempts to check if CFS session has ended
    loop {
        let ims_job: Value = ims::job::http_client::get(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            Some(ims_job_id),
        )
        .await
        .unwrap();

        log::debug!(
            "IMS job details:\n{}",
            serde_json::to_string_pretty(&ims_job).unwrap()
        );

        let ims_job_status = ims_job["status"].as_str().unwrap();

        if (ims_job_status != "error" && ims_job_status != "success") && i < max {
            print!("\x1B[2K"); // Clear current line
            io::stdout().flush().unwrap();
            print!(
                "\rWaiting IMS job '{}' with job status '{}'. Checking again in 2 secs. Attempt {} of {}",
                ims_job_id, ims_job_status, i, max
            );
            io::stdout().flush().unwrap();

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            i += 1;
        } else {
            println!(
                "\nIMS job '{}' finished with job status '{}'.\nExit",
                ims_job_id, ims_job_status
            );
            break;
        }
    }
}
