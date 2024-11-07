use crate::AppState;
use actix_web::{get, web, HttpResponse, Responder};
use serde_json::{json, Value};
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

// function to retrieve stats from restic repository using the restic cli
async fn get_restic_stats(repo_path: &str, repo_password: &str) -> Result<Value, String> {
    // creates a temporary file to store the repository password securely
    let mut password_file = NamedTempFile::new()
        .map_err(|e| format!("Failed to create temp file for password: {}", e))?;

    // write the password to the temporary file
    password_file
        .write_all(repo_password.as_bytes())
        .map_err(|e| format!("Failed to write password to temp file: {}", e))?;

    // executes the restic cli command to fetch stats in json format
    let output = Command::new("restic")
        .arg("-r")
        .arg(repo_path)
        .arg("--password-file")
        .arg(password_file.path())
        .arg("stats")
        .arg("--json")
        .output()
        .map_err(|e| format!("Failed to execute restic: {}", e))?;

    // checks if the command executed successfully
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Restic error: {}", stderr));
    }

    // parses the json output from the restic command
    let stdout =
        String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8 sequence: {}", e))?;
    let json: Value =
        serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse JSON: {}", e))?;
    Ok(json)
}

// endpoint to retrieve restic stats (/stats)
#[get("/stats")]
async fn stats(data: web::Data<AppState>) -> impl Responder {
    let config = data.config.lock().await;

    match get_restic_stats(&config.repository.path, &config.repository.password).await {
        Ok(json) => HttpResponse::Ok().json(json),
        Err(err) => HttpResponse::InternalServerError().json(json!({ "error": err })),
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(stats);
}
