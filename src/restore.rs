use crate::AppState;
use actix_web::{post, web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

// request structure for the restore endpoint
#[derive(Deserialize)]
struct RestoreRequest {
    snapshot_id: String,
    target_dir: String,
}

// function to restore a snapshot using restic
async fn restore_restic_snapshot(
    repo_path: &str,
    repo_password: &str,
    snapshot_id: &str,
    target_dir: &str,
) -> Result<(), String> {
    let mut password_file = NamedTempFile::new()
        .map_err(|e| format!("Failed to create temp file for password: {}", e))?;
    password_file
        .write_all(repo_password.as_bytes())
        .map_err(|e| format!("Failed to write password to temp file: {}", e))?;

    let output = Command::new("restic")
        .arg("-r")
        .arg(repo_path)
        .arg("--password-file")
        .arg(password_file.path())
        .arg("restore")
        .arg(snapshot_id)
        .arg("--target")
        .arg(target_dir)
        .output()
        .map_err(|e| format!("Failed to execute restic: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Restic error: {}", stderr));
    }

    Ok(())
}

// endpoint for restoring a snapshot
#[post("/restore")]
async fn restore_snapshot(
    data: web::Data<AppState>,
    req: web::Json<RestoreRequest>,
) -> impl Responder {
    let config = data.config.lock().await;

    if req.target_dir.trim().is_empty() {
        return HttpResponse::BadRequest().json(json!({ "error": "Target directory is required" }));
    }

    match restore_restic_snapshot(
        &config.repository.path,
        &config.repository.password,
        &req.snapshot_id,
        &req.target_dir,
    )
    .await
    {
        Ok(_) => HttpResponse::Ok().json(json!({ "message": "Snapshot restored successfully" })),
        Err(err) => HttpResponse::InternalServerError().json(json!({ "error": err })),
    }
}
