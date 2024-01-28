use std::collections::HashMap;

use bollard::{
    container::{
        CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
        StartContainerOptions, StopContainerOptions,
    },
    image::CreateImageOptions,
    service::{HostConfig, MountTypeEnum},
    Docker, API_DEFAULT_VERSION,
};
use clap::Parser;
use futures_util::TryStreamExt;

#[derive(Debug, Parser)]
struct Args {
    /// Path to the Docker socket.
    #[clap(long, short)]
    docker_socket: Option<String>,
    /// Path to the Who's On First SQLite database.
    #[clap(long, short)]
    wof_db: String,
    /// Whether to forcefully recreate the container. Default false.
    #[clap(long, short, default_value = "false")]
    recreate: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum ContainerStatus {
    Running,
    Stopped,
    DoesNotExist,
}

const PIP_SERVICE_IMAGE: &str = "docker.io/pelias/pip-service:latest";

async fn docker_connect() -> Result<Docker, Box<dyn std::error::Error>> {
    let docker = if let Some(docker_socket) = &Args::parse().docker_socket {
        Docker::connect_with_socket(docker_socket, 20, API_DEFAULT_VERSION)?
    } else {
        Docker::connect_with_local_defaults()?
    };
    Ok(docker)
}

async fn get_container_status() -> Result<ContainerStatus, Box<dyn std::error::Error>> {
    let docker = docker_connect().await?;

    let containers = &docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await?;

    for container in containers {
        if let Some(names) = &container.names {
            if names.contains(&"/airmail-pip-service".to_string()) {
                if &container.state == &Some("running".to_string()) {
                    return Ok(ContainerStatus::Running);
                } else {
                    return Ok(ContainerStatus::Stopped);
                }
            }
        }
    }
    Ok(ContainerStatus::DoesNotExist)
}

async fn maybe_start_container(
    wof_db_path: &str,
    recreate: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let container_state = get_container_status().await?;
    if container_state == ContainerStatus::Running && !recreate {
        println!("Container `airmail-pip-service` is already running.");
        return Ok(());
    }

    let docker = docker_connect().await?;

    let pip_config = bollard::container::Config {
        image: Some(PIP_SERVICE_IMAGE),
        env: Some(vec![]),
        host_config: Some(HostConfig {
            port_bindings: Some(HashMap::from([(
                "3102/tcp".to_string(),
                Some(vec![bollard::models::PortBinding {
                    host_ip: None,
                    host_port: Some("3102".to_string()),
                }]),
            )])),
            mounts: Some(vec![bollard::models::Mount {
                source: Some(wof_db_path.to_string()),
                target: Some(
                    "/mnt/pelias/whosonfirst/sqlite/whosonfirst-data-mapped.db".to_string(),
                ),
                typ: Some(MountTypeEnum::BIND),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        exposed_ports: Some(HashMap::from([("3102/tcp", HashMap::new())])),
        ..Default::default()
    };

    println!("Pulling image: {}", PIP_SERVICE_IMAGE);
    let _ = &docker
        .create_image(
            Some(CreateImageOptions {
                from_image: PIP_SERVICE_IMAGE,
                ..Default::default()
            }),
            None,
            None,
        )
        .try_collect::<Vec<_>>()
        .await?;

    if recreate {
        println!("Stopping container `airmail-pip-service`");
        let _ = &docker
            .stop_container("airmail-pip-service", None::<StopContainerOptions>)
            .await?;
        let _ = &docker
            .remove_container("airmail-pip-service", None::<RemoveContainerOptions>)
            .await?;
    }

    if container_state == ContainerStatus::DoesNotExist || recreate {
        println!("Creating container `airmail-pip-service`");
        let _ = &docker
            .create_container(
                Some(CreateContainerOptions {
                    name: "airmail-pip-service",
                    platform: None,
                }),
                pip_config,
            )
            .await?;
    }

    println!("Starting container `airmail-pip-service`");
    let _ = &docker
        .start_container("airmail-pip-service", None::<StartContainerOptions<String>>)
        .await?;

    println!("Waiting for container to start.");
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    if get_container_status().await? == ContainerStatus::Running {
        println!("Container `airmail-pip-service` is running.");
    } else {
        println!("Container `airmail-pip-service` failed to start.");
        return Err("Container `airmail-pip-service` failed to start.".into());
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    maybe_start_container(&args.wof_db, args.recreate).await?;

    Ok(())
}
