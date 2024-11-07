use actix_cors::Cors;
use actix_web::{delete, get, web, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::process::Command;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::sync::Mutex;

// configuration structure based on the expected structure of config.toml
#[derive(Deserialize)]
struct Config {
    repository: RepositoryConfig,
    server: ServerConfig,
}

// repository configuration details, including the path to the restic repository and password
#[derive(Deserialize)]
struct RepositoryConfig {
    path: String,
    password: String,
}

// server configuration for ip address and port
#[derive(Deserialize)]
struct ServerConfig {
    ip: String,
    port: u16,
}

// application state containing the configuration, wrapped in an Arc<Mutex> for thread-safe access
struct AppState {
    config: Arc<Mutex<Config>>,
}

// error response structure for json api responses
#[derive(serde::Serialize)]
struct ErrorResponse {
    error: String,
}

// retrieves the config file path from the user's home directory
fn get_config_path() -> Result<PathBuf, String> {
    let home_dir = env::var("HOME").map_err(|_| "HOME directory not found".to_string())?;
    let config_path = PathBuf::from(format!("{}/.config/resticapi/config.toml", home_dir));

    if config_path.exists() {
        Ok(config_path)
    } else {
        Err("Configuration file not found at ~/.config/resticapi/config.toml".to_string())
    }
}

// loads configuration data from the toml file and deserializes it into config struct
fn load_config() -> Result<Config, Box<dyn Error>> {
    let config_path = match get_config_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1); // exit if the configuration file is not found
        }
    };

    let config_contents = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_contents)?;
    Ok(config)
}

// function to get stats from restic repository using the restic cli
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

// executes the restic command to retrieve a list of snapshots in json format
async fn get_restic_snapshots(repo_path: &str, repo_password: &str) -> Result<Value, String> {
    // creates a temporary file for the password to securely pass it to the cli
    let mut password_file = NamedTempFile::new()
        .map_err(|e| format!("Failed to create temp file for password: {}", e))?;

    // write the password to the temporary file
    password_file
        .write_all(repo_password.as_bytes())
        .map_err(|e| format!("Failed to write password to temp file: {}", e))?;

    // run the Restic command
    let output = Command::new("restic")
        .arg("-r")
        .arg(repo_path)
        .arg("--password-file")
        .arg(password_file.path())
        .arg("snapshots")
        .arg("--json")
        .output()
        .map_err(|e| format!("Failed to execute restic: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Restic error: {}", stderr));
    }

    let stdout =
        String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8 sequence: {}", e))?;
    let json: Value =
        serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse JSON: {}", e))?;
    Ok(json)
}

// deletes a specific snapshot from the restic repository by snapshot id
async fn delete_restic_snapshot(
    repo_path: &str,
    repo_password: &str,
    snapshot_id: &str,
) -> Result<(), String> {
    // creates a temporary file for the password to securely pass it to the cli
    let mut password_file = NamedTempFile::new()
        .map_err(|e| format!("Failed to create temp file for password: {}", e))?;

    // write the password to the temporary file
    password_file
        .write_all(repo_password.as_bytes())
        .map_err(|e| format!("Failed to write password to temp file: {}", e))?;

    // executes the Restic command to delete the snapshot and prune the repository
    let output = Command::new("restic")
        .arg("-r")
        .arg(repo_path)
        .arg("--password-file")
        .arg(password_file.path())
        .arg("forget")
        .arg(snapshot_id)
        .arg("--prune")
        .output()
        .map_err(|e| format!("Failed to execute restic: {}", e))?;

    // checks if the command executed successfully
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Restic error: {}", stderr));
    }

    Ok(())
}

// endpoint to retrieve restic stats (/stats)
#[get("stats")]
async fn stats(data: web::Data<AppState>) -> impl Responder {
    let config = data.config.lock().await;

    match get_restic_stats(&config.repository.path, &config.repository.password).await {
        Ok(json) => HttpResponse::Ok().json(json),
        Err(err) => HttpResponse::InternalServerError().json(ErrorResponse { error: err }),
    }
}

// endpoint to retrieve a list of snapshots (/snapshots)
#[get("/snapshots")]
async fn snapshots(data: web::Data<AppState>) -> impl Responder {
    let config = data.config.lock().await;

    match get_restic_snapshots(&config.repository.path, &config.repository.password).await {
        Ok(json) => HttpResponse::Ok().json(json),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e })),
    }
}

// endpoint to delete a snapshot by its id (/snapshots/{id})
#[delete("/snapshots/{id}")]
async fn delete_snapshot(id: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let config = data.config.lock().await;
    let snapshot_id = id.into_inner();

    match delete_restic_snapshot(
        &config.repository.path,
        &config.repository.password,
        &snapshot_id,
    )
    .await
    {
        Ok(_) => HttpResponse::Ok().json(json!({ "message": "Snapshot deleted successfully" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e })),
    }
}

// main function to start the actix web server
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // load configuration from the config file
    let config = load_config().expect("Failed to load configuration");
    let config = Arc::new(Mutex::new(config));

    // clones ip and port to avoid moving config later
    let server_ip;
    let server_port;
    {
        let config_guard = config.lock().await;
        server_ip = config_guard.server.ip.clone();
        server_port = config_guard.server.port;
    }

    // starts the http server
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(AppState {
                config: Arc::clone(&config),
            }))
            .service(stats)
            .service(snapshots)
            .service(delete_snapshot)
    })
    .bind((server_ip, server_port))?
    .run()
    .await
}
