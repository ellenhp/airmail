pub mod openstreetmap;
pub mod query_pip;

use airmail::poi::{AirmailPoi, ToIndexPoi};
use bollard::{
    container::{
        CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
        StartContainerOptions, StopContainerOptions,
    },
    service::{HostConfig, MountTypeEnum},
    Docker, API_DEFAULT_VERSION,
};
use clap::Parser;
use crossbeam::channel::{Receiver, Sender};
use std::{collections::HashMap, error::Error};
use tokio::spawn;

pub async fn populate_admin_areas(poi: &mut AirmailPoi, port: usize) -> Result<(), Box<dyn Error>> {
    let pip_response = query_pip::query_pip(poi.s2cell, port).await?;
    for admin in pip_response.admins {
        poi.admins.push(admin);
    }

    Ok(())
}

#[derive(Debug, Parser)]
struct Args {
    /// Path to the Docker socket.
    #[clap(long, short)]
    docker_socket: Option<String>,
    /// Path to the Who's On First Spatialite database.
    #[clap(long, short)]
    wof_db: String,
    /// Whether to forcefully recreate the container. Default false.
    #[clap(long, short, default_value = "false")]
    recreate: bool,
    /// Path to an OpenAddresses data file.
    #[clap(long, short)]
    openaddresses: Option<String>,
    /// Path to an osmflat file. Refer to `osmflatc` documentation for instructions on how to generate this file.
    #[clap(long, short)]
    osmflat: Option<String>,
    /// Path to flat nodes file for turbosm.
    #[clap(long, short)]
    turbosm_nodes: Option<String>,
    /// Path to the Airmail index.
    #[clap(long, short)]
    index: String,
}

#[derive(Debug, Clone, PartialEq)]
enum ContainerStatus {
    Running,
    Stopped,
    DoesNotExist,
}

const PIP_SERVICE_IMAGE: &str = "spatial_custom";
// const PIP_SERVICE_IMAGE: &str = "docker.io/pelias/spatial:latest";

async fn docker_connect() -> Result<Docker, Box<dyn std::error::Error>> {
    let docker = if let Some(docker_socket) = &Args::parse().docker_socket {
        Docker::connect_with_socket(docker_socket, 20, API_DEFAULT_VERSION)?
    } else {
        Docker::connect_with_local_defaults()?
    };
    Ok(docker)
}

async fn get_container_status(
    idx: usize,
    docker: &Docker,
) -> Result<ContainerStatus, Box<dyn std::error::Error>> {
    let containers = &docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await?;

    for container in containers {
        if let Some(names) = &container.names {
            if names.contains(&format!("/airmail-pip-service-{}", idx)) {
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

async fn maybe_start_pip_container(
    wof_db_path: &str,
    recreate: bool,
    docker: &Docker,
) -> Result<(), Box<dyn std::error::Error>> {
    // Holdover from when we had multiple containers.
    let idx = 0;
    let container_state = get_container_status(idx, docker).await?;
    if container_state == ContainerStatus::Running && !recreate {
        println!(
            "Container `airmail-pip-service-{}` is already running.",
            idx
        );
        return Ok(());
    }

    let docker = docker_connect().await?;

    let pip_config = bollard::container::Config {
        image: Some(PIP_SERVICE_IMAGE),
        env: Some(vec![]),
        host_config: Some(HostConfig {
            port_bindings: Some(HashMap::from([(
                3000.to_string(),
                Some(vec![bollard::models::PortBinding {
                    host_ip: None,
                    host_port: Some(format!("{}", 3102 + idx)),
                }]),
            )])),
            mounts: Some(vec![bollard::models::Mount {
                source: Some(wof_db_path.to_string()),
                target: Some("/mnt/whosonfirst/whosonfirst-spatialite.db".to_string()),
                typ: Some(MountTypeEnum::BIND),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        cmd: Some(vec![
            "server",
            "--db",
            "/mnt/whosonfirst/whosonfirst-spatialite.db",
        ]),
        exposed_ports: Some(HashMap::from([("3000/tcp", HashMap::new())])),
        ..Default::default()
    };

    // println!("Pulling image: {}", PIP_SERVICE_IMAGE);
    // let _ = &docker
    //     .create_image(
    //         Some(CreateImageOptions {
    //             from_image: PIP_SERVICE_IMAGE,
    //             ..Default::default()
    //         }),
    //         None,
    //         None,
    //     )
    //     .try_collect::<Vec<_>>()
    //     .await?;

    if recreate {
        println!("Stopping container `airmail-pip-service-{}`", idx);
        let _ = &docker
            .stop_container(
                &format!("airmail-pip-service-{}", idx),
                None::<StopContainerOptions>,
            )
            .await;
        let _ = &docker
            .remove_container(
                &format!("airmail-pip-service-{}", idx),
                None::<RemoveContainerOptions>,
            )
            .await;
    }

    if container_state == ContainerStatus::DoesNotExist || recreate {
        println!("Creating container `airmail-pip-service-{}`", idx);
        let _ = &docker
            .create_container(
                Some(CreateContainerOptions {
                    name: &format!("airmail-pip-service-{}", idx),
                    platform: None,
                }),
                pip_config,
            )
            .await?;
    }

    if get_container_status(idx, &docker).await? != ContainerStatus::Running {
        println!("Starting container `airmail-pip-service-{}`", idx);
        let _ = &docker
            .start_container(
                &format!("airmail-pip-service-{}", idx),
                None::<StartContainerOptions<String>>,
            )
            .await?;
        println!("Waiting for container to start.");
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    if get_container_status(idx, &docker).await? == ContainerStatus::Running {
        println!("Container `airmail-pip-service-{}` is running.", idx);
    } else {
        println!("Container `airmail-pip-service-{}` failed to start.", idx);
        return Err(format!("Container `airmail-pip-service-{}` failed to start.", idx).into());
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();
    let docker = docker_connect().await?;
    let _ = subprocess::Exec::cmd("chcon")
        .arg("-t")
        .arg("container_file_t")
        .arg(&args.wof_db)
        .join();
    maybe_start_pip_container(&args.wof_db, args.recreate, &docker).await?;

    if let Some(osmflat_path) = args.osmflat {
        let mut nonblocking_join_handles = Vec::new();
        let (no_admin_sender, no_admin_receiver): (Sender<AirmailPoi>, Receiver<AirmailPoi>) =
            crossbeam::channel::bounded(1024 * 64);
        let (to_index_sender, to_index_receiver): (Sender<ToIndexPoi>, Receiver<ToIndexPoi>) =
            crossbeam::channel::bounded(1024 * 64);

        for _ in 0..1.max(num_cpus::get() / 2) {
            let no_admin_receiver = no_admin_receiver.clone();
            let to_index_sender = to_index_sender.clone();
            nonblocking_join_handles.push(spawn(async move {
                loop {
                    let mut poi = if let Ok(poi) = no_admin_receiver.recv() {
                        poi
                    } else {
                        break;
                    };
                    let mut sent = false;
                    for attempt in 0..5 {
                        if attempt > 0 {
                            println!("Retrying to populate admin areas.");
                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        }
                        let port = 3102;
                        if let Err(err) = populate_admin_areas(&mut poi, port).await {
                            println!("Failed to populate admin areas. {}", err);
                        } else {
                            let poi = ToIndexPoi::from(poi);
                            to_index_sender.send(poi).unwrap();
                            sent = true;
                            break;
                        }
                    }
                    if !sent {
                        println!("Failed to populate admin areas after 5 attempts. Skipping POI.");
                    }
                }
            }));
        }
        let index_path = args.index.clone();
        let start = std::time::Instant::now();

        let indexing_join_handle = spawn(async move {
            if !std::path::Path::new(&index_path).exists() {
                std::fs::create_dir(&index_path).unwrap();
            }
            let mut index = airmail::index::AirmailIndex::create(&index_path).unwrap();
            let mut writer = index.writer().unwrap();
            let mut count = 0;
            loop {
                {
                    count += 1;
                    if count % 1000 == 0 {
                        println!(
                            "{} POIs parsed in {} seconds, {} per second.",
                            count,
                            start.elapsed().as_secs(),
                            count as f64 / start.elapsed().as_secs_f64(),
                        );
                    }
                }

                if let Ok(poi) = to_index_receiver.recv() {
                    if let Err(err) = writer.add_poi(poi).await {
                        println!("Failed to add POI to index. {}", err);
                    }
                } else {
                    break;
                }
            }
            writer.commit().unwrap();
        });

        openstreetmap::parse_osm(&osmflat_path, &mut |poi| {
            no_admin_sender.send(poi).unwrap();
            Ok(())
        })
        .unwrap();
        drop(no_admin_sender);
        println!("Waiting for tasks to finish.");
        for handle in nonblocking_join_handles {
            handle.await.unwrap();
        }
        drop(to_index_sender);
        indexing_join_handle.await.unwrap();
    }

    Ok(())
}
