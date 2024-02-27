mod query_pip;

use airmail::{
    index::AirmailIndex,
    poi::{SchemafiedPoi, ToIndexPoi},
};
use bollard::{
    container::{
        CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
        StartContainerOptions, StopContainerOptions,
    },
    service::{HostConfig, MountTypeEnum},
    Docker, API_DEFAULT_VERSION,
};
use crossbeam::channel::{Receiver, Sender};
use redb::{Database, ReadTransaction, TableDefinition};
use std::{collections::HashMap, error::Error, sync::Arc};
use tokio::spawn;

pub(crate) const ADMIN_AREAS: TableDefinition<u64, &[u8]> = TableDefinition::new("admin_areas");
pub(crate) const ADMIN_NAMES: TableDefinition<u64, &str> = TableDefinition::new("admin_names");

pub(crate) enum WofCacheItem {
    Names(u64, Vec<String>),
    Admins(u64, Vec<u64>),
}

pub(crate) async fn populate_admin_areas<'a>(
    read: &'_ ReadTransaction<'_>,
    to_cache_sender: Sender<WofCacheItem>,
    poi: &mut ToIndexPoi,
    port: usize,
) -> Result<(), Box<dyn Error>> {
    let pip_response = query_pip::query_pip(read, to_cache_sender, poi.s2cell, port).await?;
    for admin in pip_response.admins {
        poi.admins.push(admin);
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
enum ContainerStatus {
    Running,
    Stopped,
    DoesNotExist,
}

const PIP_SERVICE_IMAGE: &str = "spatial_custom";
// const PIP_SERVICE_IMAGE: &str = "docker.io/pelias/spatial:latest";

async fn docker_connect(socket: &Option<String>) -> Result<Docker, Box<dyn std::error::Error>> {
    let docker = if let Some(docker_socket) = socket {
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

pub struct ImporterBuilder {
    admin_cache: String,
    wof_db: String,
    recreate: bool,
    docker_socket: Option<String>,
}

impl ImporterBuilder {
    pub fn new(whosonfirst_spatialite_path: &str) -> Self {
        let tmp_dir = std::env::temp_dir();
        let admin_cache = tmp_dir.join("admin_cache.db").to_string_lossy().to_string();
        Self {
            admin_cache,
            wof_db: whosonfirst_spatialite_path.to_string(),
            recreate: false,
            docker_socket: None,
        }
    }

    pub fn admin_cache(mut self, admin_cache: &str) -> Self {
        self.admin_cache = admin_cache.to_string();
        self
    }

    pub fn recreate_containers(mut self, recreate: bool) -> Self {
        self.recreate = recreate;
        self
    }

    pub fn docker_socket(mut self, docker_socket: &str) -> Self {
        self.docker_socket = Some(docker_socket.to_string());
        self
    }

    pub async fn build(self) -> Importer {
        let db = Database::create(&self.admin_cache)
            .expect("Failed to open or create administrative area cache database.");
        {
            let txn = db.begin_write().unwrap();
            {
                txn.open_table(ADMIN_AREAS).unwrap();
                txn.open_table(ADMIN_NAMES).unwrap();
            }
            txn.commit().unwrap();
        }
        let docker_socket = docker_connect(&self.docker_socket)
                .await
                .expect("Failed to connect to the Docker daemon at socket path. Try specifiying the correct path or verifying it exists, and verifying permissions.");
        {
            let _ = subprocess::Exec::cmd("chcon")
                .arg("-t")
                .arg("container_file_t")
                .arg(&self.wof_db)
                .join();
            maybe_start_pip_container(&self.wof_db, self.recreate, &docker_socket)
                .await
                .expect("Failed to start spatial server container.");
        }
        Importer {
            admin_cache: Arc::new(db),
        }
    }
}

pub struct Importer {
    admin_cache: Arc<Database>,
}

impl Importer {
    pub async fn run_import(
        &self,
        index: &mut AirmailIndex,
        source: &str,
        receiver: Receiver<ToIndexPoi>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut nonblocking_join_handles = Vec::new();
        let (to_cache_sender, to_cache_receiver): (Sender<WofCacheItem>, Receiver<WofCacheItem>) =
            crossbeam::channel::bounded(1024 * 64);
        let (to_index_sender, to_index_receiver): (Sender<SchemafiedPoi>, Receiver<SchemafiedPoi>) =
            crossbeam::channel::bounded(1024 * 64);
        {
            let admin_cache = self.admin_cache.clone();
            std::thread::spawn(move || {
                let mut write = admin_cache.begin_write().unwrap();
                let mut count = 0;
                loop {
                    count += 1;
                    if count % 5000 == 0 {
                        write.commit().unwrap();
                        write = admin_cache.begin_write().unwrap();
                    }
                    match to_cache_receiver.recv() {
                        Ok(WofCacheItem::Names(admin, names)) => {
                            let mut table = write.open_table(ADMIN_NAMES).unwrap();
                            let packed = names.join("\0");
                            table.insert(admin, packed.as_str()).unwrap();
                        }
                        Ok(WofCacheItem::Admins(s2cell, admins)) => {
                            let mut table = write.open_table(ADMIN_AREAS).unwrap();
                            let packed = admins
                                .iter()
                                .map(|id| id.to_le_bytes())
                                .flatten()
                                .collect::<Vec<_>>();
                            table.insert(s2cell, packed.as_slice()).unwrap();
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        for _ in 1..num_cpus::get_physical() {
            println!("Spawning worker");
            let no_admin_receiver = receiver.clone();
            let to_index_sender = to_index_sender.clone();
            let to_cache_sender = to_cache_sender.clone();
            let admin_cache = self.admin_cache.clone();
            nonblocking_join_handles.push(spawn(async move {
                let mut read = admin_cache.begin_read().unwrap();
                let mut counter = 0;
                loop {
                    let mut poi = if let Ok(poi) = no_admin_receiver.recv() {
                        poi
                    } else {
                        break;
                    };
                    counter += 1;
                    if counter % 1000 == 0 {
                        read = admin_cache.begin_read().unwrap();
                    }
                    let mut sent = false;
                    for attempt in 0..5 {
                        if attempt > 0 {
                            println!("Retrying to populate admin areas.");
                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        }
                        let port = 3102;
                        if let Err(err) =
                            populate_admin_areas(&read, to_cache_sender.clone(), &mut poi, port)
                                .await
                        {
                            println!("Failed to populate admin areas. {}", err);
                        } else {
                            let poi = SchemafiedPoi::from(poi);
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
        drop(to_index_sender);
        let start = std::time::Instant::now();

        let mut writer = index.writer().unwrap();
        let mut count = 0;
        loop {
            {
                count += 1;
                if count % 10000 == 0 {
                    println!(
                        "{} POIs parsed in {} seconds, {} per second.",
                        count,
                        start.elapsed().as_secs(),
                        count as f64 / start.elapsed().as_secs_f64(),
                    );
                }
            }

            if let Ok(poi) = to_index_receiver.recv() {
                if let Err(err) = writer.add_poi(poi, &source).await {
                    println!("Failed to add POI to index. {}", err);
                }
            } else {
                break;
            }
        }
        writer.commit().unwrap();

        println!("Waiting for tasks to finish.");
        for handle in nonblocking_join_handles {
            handle.await.unwrap();
        }

        Ok(())
    }
}
