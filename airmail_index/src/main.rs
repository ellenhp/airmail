#[macro_use]
extern crate lazy_static;

pub mod openaddresses;
pub mod openstreetmap;
pub mod query_pip;
pub mod substitutions;

use airmail::poi::AirmailPoi;
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
use crossbeam::channel::{Receiver, Sender};
use futures_util::TryStreamExt;
use geojson::GeoJson;
use s2::{cellid::CellID, latlng::LatLng};
use std::{collections::HashMap, error::Error, str::FromStr, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    spawn,
    sync::Mutex,
    task::spawn_blocking,
};

use crate::openaddresses::parse_oa_geojson;

pub async fn populate_admin_areas(poi: &mut AirmailPoi, port: usize) -> Result<(), Box<dyn Error>> {
    let cell = CellID(poi.s2cell);
    let latlng = LatLng::from(cell);
    let pip_response = query_pip::query_pip(latlng.lat.deg(), latlng.lng.deg(), port).await?;
    let mut locality: Vec<String> = pip_response
        .locality
        .unwrap_or_default()
        .iter()
        .map(|a| a.name.to_lowercase())
        .collect();
    if let Some(neighbourhood) = pip_response.neighbourhood {
        locality.extend(neighbourhood.iter().map(|a| a.name.to_lowercase()));
    }
    let region = pip_response
        .region
        .unwrap_or_default()
        .iter()
        .map(|a| a.name.to_lowercase())
        .collect();
    let country = pip_response
        .country
        .unwrap_or_default()
        .iter()
        .map(|a| a.name.to_lowercase())
        .collect();

    poi.locality = locality;
    poi.region = region;
    poi.country = country;

    Ok(())
}

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
    /// Path to an OpenAddresses data file.
    #[clap(long, short)]
    openaddresses: Option<String>,
    /// Path to an OpenStreetMap file in "turbosm" form. Use turbosm to convert if need be.
    #[clap(long, short)]
    turbosm: Option<String>,
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

const PIP_SERVICE_IMAGE: &str = "docker.io/pelias/pip-service:latest";

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
    idx: usize,
    recreate: bool,
    docker: &Docker,
) -> Result<(), Box<dyn std::error::Error>> {
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
                3102.to_string(),
                Some(vec![bollard::models::PortBinding {
                    host_ip: None,
                    host_port: Some(format!("{}", 3102 + idx)),
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
    let index_path = args.index.clone();
    let docker = docker_connect().await?;
    let max_pip = 8;
    for i in 0..max_pip {
        let _ = subprocess::Exec::cmd("chcon")
            .arg("-t")
            .arg("container_file_t")
            .arg(&args.wof_db)
            .join();
        maybe_start_pip_container(&args.wof_db, i, args.recreate, &docker).await?;
    }

    if let Some(turbosm_path) = args.turbosm {
        let mut nonblocking_join_handles = Vec::new();
        let (no_admin_sender, no_admin_receiver): (Sender<AirmailPoi>, Receiver<AirmailPoi>) =
            crossbeam::channel::bounded(1024);
        let (to_index_sender, to_index_receiver): (Sender<AirmailPoi>, Receiver<AirmailPoi>) =
            crossbeam::channel::bounded(1024);
        for _ in 0..128 {
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
                    for _attempt in 0..5 {
                        let port = (rand::random::<usize>() % max_pip) + 3102;
                        if let Err(err) = populate_admin_areas(&mut poi, port).await {
                            println!("Failed to populate admin areas. {}", err);
                        } else {
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
        let count = Arc::new(Mutex::new(0));
        let start = std::time::Instant::now();
        let count = count.clone();

        let indexing_join_handle = spawn(async move {
            if !std::path::Path::new(&index_path).exists() {
                std::fs::create_dir(&index_path).unwrap();
            }
            let mut index = airmail::index::AirmailIndex::create(&index_path).unwrap();
            let mut writer = index.writer().unwrap();
            loop {
                {
                    let mut count = count.lock().await;
                    *count += 1;
                    if *count % 1000 == 0 {
                        println!(
                            "{} POIs parsed in {} seconds, {} per second.",
                            *count,
                            start.elapsed().as_secs(),
                            *count as f64 / start.elapsed().as_secs_f64(),
                        );
                    }
                }

                if let Ok(poi) = to_index_receiver.recv() {
                    if let Err(err) = writer.add_poi(poi) {
                        println!("Failed to add POI to index. {}", err);
                    }
                } else {
                    break;
                }
            }
            writer.commit().unwrap();
        });

        openstreetmap::parse_osm(&turbosm_path, &mut |poi| {
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

    if let Some(openaddresses_path) = args.openaddresses {
        let openaddresses_file = tokio::fs::File::open(openaddresses_path).await?;
        let reader = BufReader::new(openaddresses_file);
        let mut lines = reader.lines();
        let count = Arc::new(Mutex::new(0));
        let start = std::time::Instant::now();
        let (raw_sender, raw_receiver): (Sender<String>, Receiver<String>) =
            crossbeam::channel::bounded(1024);
        let (no_admin_sender, no_admin_receiver): (Sender<AirmailPoi>, Receiver<AirmailPoi>) =
            crossbeam::channel::bounded(1024);
        let (to_index_sender, to_index_receiver): (Sender<AirmailPoi>, Receiver<AirmailPoi>) =
            crossbeam::channel::bounded(1024);
        let mut blocking_join_handles = Vec::new();
        let mut nonblocking_join_handles = Vec::new();
        for _ in 0..16 {
            let receiver = raw_receiver.clone();
            let no_admin_sender = no_admin_sender.clone();
            let count = count.clone();
            blocking_join_handles.push(spawn_blocking(move || loop {
                {
                    let mut count = count.blocking_lock();
                    *count += 1;
                    if *count % 1000 == 0 {
                        println!(
                            "{} POIs parsed in {} seconds, {} per second.",
                            *count,
                            start.elapsed().as_secs(),
                            *count as f64 / start.elapsed().as_secs_f64(),
                        );
                    }
                }

                let line = if let Ok(line) = receiver.recv() {
                    line
                } else {
                    break;
                };
                let geojson = if let Ok(geojson) = GeoJson::from_str(&line) {
                    geojson
                } else {
                    println!("Failed to parse line: {}", line);
                    continue;
                };
                match parse_oa_geojson(&geojson) {
                    Ok(poi) => {
                        no_admin_sender.send(poi).unwrap();
                    }
                    Err(err) => {
                        println!("Failed to parse line. {}", err);
                    }
                }
            }));
        }
        for _ in 0..16 {
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
                    for _attempt in 0..5 {
                        let port = (rand::random::<usize>() % max_pip) + 3102;
                        if let Err(err) = populate_admin_areas(&mut poi, port).await {
                            println!("Failed to populate admin areas. {}", err);
                        } else {
                            to_index_sender.send(poi).unwrap();
                            sent = true;
                            break;
                        }
                    }
                    if !sent {
                        println!("Failed to populate admin areas after 5 attempts.");
                    }
                }
            }));
        }
        let indexing_join_handle = spawn(async move {
            let mut index = airmail::index::AirmailIndex::create(&index_path).unwrap();
            let mut writer = index.writer().unwrap();
            loop {
                if let Ok(poi) = to_index_receiver.recv() {
                    if let Err(err) = writer.add_poi(poi) {
                        println!("Failed to add POI to index. {}", err);
                    }
                } else {
                    break;
                }
            }
            writer.commit().unwrap();
        });
        loop {
            if let Some(line) = lines.next_line().await? {
                raw_sender.send(line)?;
            } else {
                break;
            }
        }
        drop(raw_sender);
        println!("Waiting for threads to finish.");
        for handle in blocking_join_handles {
            handle.await.unwrap();
        }
        drop(no_admin_sender);
        println!("Waiting for tasks to finish.");
        for handle in nonblocking_join_handles {
            handle.await.unwrap();
        }
        drop(to_index_sender);
        indexing_join_handle.await.unwrap();
    }

    println!("Done. Merging segments.");
    let index_path = args.index.clone();
    let mut index = airmail::index::AirmailIndex::new(&index_path)?;
    index.merge().await?;

    Ok(())
}
