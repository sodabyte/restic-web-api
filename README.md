# Restic Web API


A RESTful API built in Rust using Actix Web for managing and interacting with your Restic backup repository. This API lets you check out Restic stats, view snapshots, and delete snapshots through simple HTTP endpoints.


Note: I am new to Rust programming, and this project is part of my learning process. As such, the security and reliability of the program are not fully guaranteed. Please use it with caution, and feel free to contribute or suggest improvements.


## Features


- View Restic repository stats.
- List and manage Restic snapshots.
- Delete specific snapshots from the repository.




## Installation


### Prerequisites


- Rust (stable version recommended)
- Cargo (Rust's package manager)
- Restic installed and configured




### Steps


1. Clone the repository:


```bash
git clone https://github.com/sodabyte/restic-web-api.git
cd restic-web-api
```


2. Build the project:


```bash
cargo build --release
```


3. Create a configuration file (config.toml) in ~/.config/resticapi/config.toml:


```toml
[repository]
path = "/path/to/your/restic/repository"
password = "your-repository-password"


[server]
ip = "127.0.0.1"
port = 8080
```


4. Run the API server:


```bash
./target/release/restic-web-api
```


5. The server will be accessible at http://127.0.0.1:8080.


## API Endpoints


GET /stats: Retrieve stats from the Restic repository.


GET /snapshots: List all snapshots in the repository.


DELETE /snapshots/{id}: Delete a snapshot by its ID.




## Configuration


You can modify the config.toml file to set your repository path, password, and the server's IP/port.


Example config.toml:


```bash
[repository]
path = "/path/to/your/restic/repository"
password = "your-repository-password"


[server]
ip = "127.0.0.1"
port = 8080
```
