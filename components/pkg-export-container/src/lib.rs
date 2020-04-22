#[macro_use]
extern crate clap;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;

pub use crate::{build::BuildSpec,
                docker::{ContainerImage,
                         DockerBuildRoot},
                engine::Engine,
                error::{Error,
                        Result}};
use habitat_common::ui::{UIWriter,
                         UI};
use habitat_core::url::default_bldr_url;
use rusoto_core::{request::HttpClient,
                  Region};
use rusoto_credential::StaticProvider;
use rusoto_ecr::{Ecr,
                 EcrClient,
                 GetAuthorizationTokenRequest};
use std::{env,
          fmt,
          result,
          str::FromStr};

mod accounts;
mod build;
pub mod cli;
mod docker;
mod engine;
mod error;
mod graph;
#[cfg(unix)]
mod rootfs;
mod util;

/// The version of this library and program when built.
pub const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));

/// The Habitat Package Identifier string for a Busybox package.
const BUSYBOX_IDENT: &str = "core/busybox-static";
/// The Habitat Package Identifier string for SSL certificate authorities (CA) certificates package.
const CACERTS_IDENT: &str = "core/cacerts";

/// An image naming policy.
///
/// This is a value struct which captures the naming and tagging intentions for an image.
#[derive(Debug)]
pub struct Naming<'a> {
    /// An optional custom image name which would override a computed default value.
    pub custom_image_name:   Option<&'a str>,
    /// Whether or not to tag the image with a latest value.
    pub latest_tag:          bool,
    /// Whether or not to tag the image with a value containing a version from a Package
    /// Identifier.
    pub version_tag:         bool,
    /// Whether or not to tag the image with a value containing a version and release from a
    /// Package Identifier.
    pub version_release_tag: bool,
    /// An optional custom tag value for the image.
    pub custom_tag:          Option<&'a str>,
    /// A URL to a custom Docker registry to publish to. This will be used as part of every tag
    /// before pushing.
    pub registry_url:        Option<&'a str>,
    /// The type of registry we're publishing to. Ex: Amazon, Docker, Google, Azure.
    pub registry_type:       RegistryType,
}

impl<'a> Naming<'a> {
    /// Creates a `Naming` from cli arguments.
    pub fn new_from_cli_matches(m: &'a clap::ArgMatches<'_>) -> Self {
        let registry_type =
            value_t!(m.value_of("REGISTRY_TYPE"), RegistryType).unwrap_or(RegistryType::Docker);
        let registry_url = m.value_of("REGISTRY_URL");

        Naming { custom_image_name: m.value_of("IMAGE_NAME"),
                 latest_tag: !m.is_present("NO_TAG_LATEST"),
                 version_tag: !m.is_present("NO_TAG_VERSION"),
                 version_release_tag: !m.is_present("NO_TAG_VERSION_RELEASE"),
                 custom_tag: m.value_of("TAG_CUSTOM"),
                 registry_url,
                 registry_type }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum RegistryType {
    Amazon,
    Azure,
    Docker,
}

impl RegistryType {
    fn variants() -> &'static [&'static str] { &["amazon", "azure", "docker"] }
}

impl FromStr for RegistryType {
    type Err = Error;

    fn from_str(value: &str) -> result::Result<Self, Self::Err> {
        match value {
            "amazon" => Ok(RegistryType::Amazon),
            "azure" => Ok(RegistryType::Azure),
            "docker" => Ok(RegistryType::Docker),
            _ => Err(Error::InvalidRegistryType(String::from(value))),
        }
    }
}

impl fmt::Display for RegistryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let disp = match *self {
            RegistryType::Amazon => "amazon",
            RegistryType::Azure => "azure",
            RegistryType::Docker => "docker",
        };
        write!(f, "{}", disp)
    }
}

/// A credentials username and password pair.
///
/// This is a value struct which references username and password values.
#[derive(Debug)]
pub struct Credentials {
    pub token: String,
}

impl Credentials {
    pub async fn new(registry_type: RegistryType, username: &str, password: &str) -> Result<Self> {
        match registry_type {
            RegistryType::Amazon => {
                // The username and password should be valid IAM credentials
                let provider =
                    StaticProvider::new_minimal(username.to_string(), password.to_string());
                // TODO TED: Make the region configurable
                let client = EcrClient::new_with(HttpClient::new()?, provider, Region::UsWest2);
                let auth_token_req = GetAuthorizationTokenRequest { registry_ids: None };
                let token = client.get_authorization_token(auth_token_req)
                                  .await
                                  .map_err(Error::TokenFetchFailed)
                                  .and_then(|resp| {
                                      resp.authorization_data
                                          .ok_or(Error::NoECRTokensReturned)
                                          .and_then(|auth_data| {
                                              auth_data[0].clone()
                                                          .authorization_token
                                                          .ok_or(Error::NoECRTokensReturned)
                                          })
                                  })?;

                Ok(Credentials { token })
            }
            RegistryType::Docker | RegistryType::Azure => {
                Ok(Credentials { token: base64::encode(&format!("{}:{}",
                                                                username.to_string(),
                                                                password.to_string())), })
            }
        }
    }
}

/// Exports a Docker image to a Docker engine from a build specification and naming policy.
///
/// # Errors
///
/// * If a generic and temporary build root directory cannot be created containing a root
/// file system
/// * If additional Docker-related files cannot be created in the root file system
/// * If building the Docker image fails
/// * If destroying the temporary build root directory fails
pub async fn export<'a>(ui: &'a mut UI,
                        build_spec: BuildSpec<'a>,
                        naming: &'a Naming<'a>,
                        memory: Option<&'a str>,
                        engine: Engine)
                        -> Result<ContainerImage> {
    ui.begin(format!("Building a runnable Docker image with: {}",
                     build_spec.idents_or_archives.join(", ")))?;
    let build_root = DockerBuildRoot::from_build_root(build_spec.create(ui).await?, ui)?;
    let image = build_root.export(ui, naming, memory, engine)?;
    build_root.destroy(ui)?;
    ui.end(format!("Docker image '{}' created with tags: {}",
                   image.name(),
                   image.tags().join(", ")))?;

    Ok(image)
}

/// Creates a build specification and naming policy from Cli arguments, and then exports a Docker
/// image to a Docker engine from them.
///
/// # Errors
///
/// * The actual import fails.
/// * Current directory does not exist.
/// * There are insufficient permissions to access the current directory.
/// * Pushing the image to remote registry fails.
/// * Parsing of credentials fails.
/// * The image (tags) cannot be removed.
pub async fn export_for_cli_matches(ui: &mut UI,
                                    matches: &clap::ArgMatches<'_>)
                                    -> Result<Option<ContainerImage>> {
    #[cfg(windows)]
    DockerOS::fail_if_not_in_windows_mode()?;

    let default_url = default_bldr_url();
    let spec = BuildSpec::new_from_cli_matches(&matches, &default_url)?;
    let naming = Naming::new_from_cli_matches(&matches);

    let engine = Engine::new_from_cli_matches(&matches);

    let docker_image = export(ui, spec, &naming, matches.value_of("MEMORY_LIMIT"), engine).await?;
    docker_image.create_report(ui, env::current_dir()?.join("results"))?;

    if matches.is_present("PUSH_IMAGE") {
        let credentials = Credentials::new(naming.registry_type,
                                           matches.value_of("REGISTRY_USERNAME")
                                                  .expect("Username not specified"),
                                           matches.value_of("REGISTRY_PASSWORD")
                                                  .expect("Password not specified")).await?;
        docker_image.push(ui, &credentials, naming.registry_url)?;
    }
    if matches.is_present("RM_IMAGE") {
        docker_image.rm(ui)?;

        Ok(None)
    } else {
        Ok(Some(docker_image))
    }
}

/// Currently when exporting containers on Windows, the Docker daemon
/// *must* be in Windows mode (i.e., only Windows containers can be
/// exported on Windows machines).
///
/// If the daemon is in Linux mode, we return an error and should stop
/// the export process.
#[derive(Clone, Copy, Debug)]
pub enum DockerOS {
    Linux,
    Windows,
}

impl DockerOS {
    /// Returns the OS for which the locally-running Docker daemon is
    /// managing containers.
    ///
    /// Daemons running on Linux will report "Linux", while a Windows
    /// daemon may report "Windows" or "Linux", depending on what mode
    /// it is currently running in.
    #[allow(dead_code)]
    fn current() -> DockerOS {
        let mut cmd = Engine::Docker.command();
        cmd.arg("version").arg("--format='{{.Server.Os}}'");
        debug!("Running command: {:?}", cmd);
        let result = cmd.output().expect("Docker command failed to spawn");
        let result = String::from_utf8_lossy(&result.stdout);
        if result.contains("windows") {
            DockerOS::Windows
        } else {
            DockerOS::Linux
        }
    }

    #[cfg(windows)]
    pub fn fail_if_not_in_windows_mode() -> Result<()> {
        match DockerOS::current() {
            DockerOS::Windows => Ok(()),
            other => Err(Error::DockerNotInWindowsMode(other).into()),
        }
    }
}
