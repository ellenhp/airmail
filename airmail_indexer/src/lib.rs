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
use lingua::{IsoCode639_3, Language};
use redb::{Database, ReadTransaction, TableDefinition};
use reqwest::Url;
use std::{collections::HashMap, error::Error, str::FromStr, sync::Arc};
use tokio::spawn;

pub(crate) const TABLE_AREAS: TableDefinition<u64, &[u8]> = TableDefinition::new("admin_areas");
pub(crate) const TABLE_NAMES: TableDefinition<u64, &str> = TableDefinition::new("admin_names");
pub(crate) const TABLE_LANGS: TableDefinition<u64, &str> = TableDefinition::new("admin_langs");

const COUNTRIES: [u64; 214] = [
    85632343, 85632573, 85632229, 85632529, 85632405, 85632773, 85632281, 85632715, 85632505,
    85632785, 85632793, 85632295, 85632717, 85632609, 85632491, 85632475, 85632997, 85632213,
    85633001, 85632167, 85632285, 85632247, 85632749, 85632623, 85633009, 85632339, 85632171,
    85632235, 85632395, 85632571, 85633041, 85632643, 85632391, 85632541, 85633051, 85632449,
    85633057, 85632245, 85632695, 85632519, 85632487, 85632675, 85632721, 85632441, 85632437,
    85633105, 85633111, 85632319, 85633121, 85632503, 85632713, 85632451, 85632261, 85633135,
    85632581, 85632217, 85632781, 85633129, 85632257, 85633143, 85632755, 85632431, 85633147,
    85632407, 85633159, 85632335, 85633163, 85632547, 85632189, 85633217, 85632603, 85632691,
    85632287, 85633171, 85632385, 85632757, 85632397, 85632483, 85632323, 85633229, 85632433,
    85633237, 85632203, 85633241, 85632315, 85632461, 85632469, 85632191, 85632361, 85633249,
    85633253, 85632593, 85632215, 85632425, 85632429, 85632329, 85632761, 85632359, 85632709,
    85632259, 85632551, 85632639, 85632231, 85632401, 85632307, 85632241, 85632533, 85632369,
    85633267, 85632313, 85632249, 85632173, 85633269, 85633275, 85633279, 85632627, 85632693,
    85633285, 85633287, 85632667, 85632223, 85632663, 85632373, 85632553, 85632181, 85632439,
    85632161, 85632679, 85633331, 85632357, 85632305, 85632383, 85633293, 85632739, 85632729,
    85632535, 85632269, 85632735, 85632599, 85633337, 85633341, 85632465, 85632747, 85633345,
    85632207, 85632179, 85632521, 85632347, 85632509, 85632659, 85633723, 85633739, 85633735,
    85632331, 85632355, 85632299, 85633745, 85633755, 85632685, 85632303, 85632253, 85632591,
    85632661, 85632751, 85633789, 85632605, 85633779, 85633769, 85632467, 85633763, 85632365,
    85632379, 85632443, 85632657, 85632765, 85632545, 85632185, 85632413, 85632635, 85632325,
    85632647, 85632293, 85632513, 85632583, 85632671, 85632703, 85632455, 85632393, 85632271,
    85632607, 85632403, 85632227, 85633805, 85632625, 102312305, 85633793, 85632511, 85632645,
    85632187, 85632569, 85632317, 85632763, 85632263, 85632681, 85633259, 1, 85632733, 1729945891,
    1729945893, 1729989201, 85632499, 85633813, 85632559, 85632243,
];

pub(crate) enum WofCacheItem {
    Names(u64, Vec<String>),
    Langs(u64, Vec<String>),
    Admins(u64, Vec<u64>),
}

pub(crate) async fn populate_admin_areas<'a>(
    read: &'_ ReadTransaction<'_>,
    to_cache_sender: Sender<WofCacheItem>,
    poi: &mut ToIndexPoi,
    spatial_url: &Url,
) -> Result<(), Box<dyn Error>> {
    let pip_response = query_pip::query_pip(read, to_cache_sender, poi.s2cell, spatial_url).await?;
    for admin in pip_response.admin_names {
        poi.admins.push(admin);
    }
    for lang in pip_response.admin_langs {
        let _ = IsoCode639_3::from_str(&lang)
            .map(|iso| poi.languages.push(Language::from_iso_code_639_3(&iso)));
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
                if container.state == Some("running".to_string()) {
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
            privileged: Some(true),
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

    if get_container_status(idx, docker).await? != ContainerStatus::Running {
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

    if get_container_status(idx, docker).await? == ContainerStatus::Running {
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
    spatial_url: Url,
}

impl ImporterBuilder {
    pub fn new(whosonfirst_spatialite_path: &str, spatial_url: &Url) -> Self {
        let tmp_dir = std::env::temp_dir();
        let admin_cache = tmp_dir.join("admin_cache.db").to_string_lossy().to_string();
        Self {
            admin_cache,
            wof_db: whosonfirst_spatialite_path.to_string(),
            recreate: false,
            docker_socket: None,
            spatial_url: spatial_url.clone(),
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
                txn.open_table(TABLE_AREAS).unwrap();
                txn.open_table(TABLE_NAMES).unwrap();
            }
            txn.commit().unwrap();
        }

        // Conditionally start the spatial server container.
        if self.docker_socket.is_some() {
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
        }
        Importer {
            admin_cache: Arc::new(db),
            spatial_url: self.spatial_url,
        }
    }
}

pub struct Importer {
    admin_cache: Arc<Database>,
    spatial_url: Url,
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
                            let mut table = write.open_table(TABLE_NAMES).unwrap();
                            let packed = names.join("\0");
                            table.insert(admin, packed.as_str()).unwrap();
                        }
                        Ok(WofCacheItem::Langs(admin, langs)) => {
                            let mut table = write.open_table(TABLE_LANGS).unwrap();
                            let packed = langs.join("\0");
                            table.insert(admin, packed.as_str()).unwrap();
                        }
                        Ok(WofCacheItem::Admins(s2cell, admins)) => {
                            let mut table = write.open_table(TABLE_AREAS).unwrap();
                            let packed = admins
                                .iter()
                                .flat_map(|id| id.to_le_bytes())
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
            let spatial_url = self.spatial_url.clone();
            nonblocking_join_handles.push(spawn(async move {
                let mut read = admin_cache.begin_read().unwrap();
                let mut counter = 0;
                while let Ok(mut poi) = no_admin_receiver.recv() {
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

                        if let Err(err) = populate_admin_areas(
                            &read,
                            to_cache_sender.clone(),
                            &mut poi,
                            &spatial_url,
                        )
                        .await
                        {
                            println!(
                                "Failed to populate admin areas, {}, attempt: {}",
                                err, attempt
                            );
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
                if let Err(err) = writer.add_poi(poi, source).await {
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
