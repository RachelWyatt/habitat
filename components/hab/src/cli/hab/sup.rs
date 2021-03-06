use super::{svc::{ConfigOptSharedLoad,
                  SharedLoad},
            util::{CacheKeyPath,
                   ConfigOptCacheKeyPath,
                   ConfigOptRemoteSup,
                   RemoteSup}};
use crate::VERSION;
use configopt::{self,
                configopt_fields,
                ConfigOpt};
use habitat_common::{cli::{RING_ENVVAR,
                           RING_KEY_ENVVAR},
                     types::{AutomateAuthToken,
                             EventStreamConnectMethod,
                             EventStreamMetadata,
                             EventStreamServerCertificate,
                             GossipListenAddr,
                             HttpListenAddr,
                             ListenCtlAddr}};
use habitat_core::{env::Config,
                   package::PackageIdent,
                   util::serde_string};
use rants::{error::Error as RantsError,
            Address as NatsAddress};
use std::{fmt,
          net::{Ipv4Addr,
                SocketAddr},
          path::PathBuf,
          str::FromStr};
use structopt::{clap::AppSettings,
                StructOpt};

#[derive(ConfigOpt, StructOpt)]
#[structopt(name = "hab",
            version = VERSION,
            about = "The Habitat Supervisor",
            author = "\nThe Habitat Maintainers <humans@habitat.sh>\n",
            usage = "hab sup <SUBCOMMAND>",
            settings = &[AppSettings::VersionlessSubcommands],
        )]
#[allow(clippy::large_enum_variant)]
pub enum Sup {
    /// Start an interactive Bash-like shell
    #[structopt(usage = "hab sup bash", no_version)]
    Bash,
    /// Depart a Supervisor from the gossip ring; kicking and banning the target from joining again
    /// with the same member-id
    #[structopt(no_version)]
    Depart {
        /// The member-id of the Supervisor to depart
        #[structopt(name = "MEMBER_ID")]
        member_id:  String,
        #[structopt(flatten)]
        remote_sup: RemoteSup,
    },
    /// Run the Habitat Supervisor
    #[structopt(no_version)]
    Run(SupRun),
    #[structopt(no_version)]
    Secret(Secret),
    /// Start an interactive Bourne-like shell
    #[structopt(usage = "hab sup sh", no_version)]
    Sh,
    /// Query the status of Habitat services
    #[structopt(no_version)]
    Status {
        /// A package identifier (ex: core/redis, core/busybox-static/1.42.2)
        #[structopt(name = "PKG_IDENT")]
        pkg_ident:  Option<PackageIdent>,
        #[structopt(flatten)]
        remote_sup: RemoteSup,
    },
    /// Gracefully terminate the Habitat Supervisor and all of its running services
    #[structopt(usage = "hab sup term [OPTIONS]", no_version)]
    Term,
}

// TODO (DM): This is unnecessarily difficult due to the orphan rule and the lack of specialization.
// The `configopt` library could be improved to make this easier.
#[derive(Deserialize, Serialize, Debug)]
struct EventStreamAddress(#[serde(with = "serde_string")] NatsAddress);

impl fmt::Display for EventStreamAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}

impl FromStr for EventStreamAddress {
    type Err = RantsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> { Ok(EventStreamAddress(s.parse()?)) }
}

#[configopt_fields]
#[derive(ConfigOpt, StructOpt, Deserialize)]
#[configopt(attrs(serde))]
#[serde(deny_unknown_fields)]
#[structopt(name = "run",
            no_version,
            about = "Run the Habitat Supervisor",
            // set custom usage string, otherwise the binary
            // is displayed confusingly as `hab-sup`
            // see: https://github.com/kbknapp/clap-rs/blob/2724ec5399c500b12a1a24d356f4090f4816f5e2/src/app/mod.rs#L373-L394
            usage = "hab sup run [FLAGS] [OPTIONS] [--] [PKG_IDENT_OR_ARTIFACT]"
        )]
#[allow(dead_code)]
pub struct SupRun {
    /// The listen address for the Gossip System Gateway
    #[structopt(name = "LISTEN_GOSSIP",
                long = "listen-gossip",
                env = GossipListenAddr::ENVVAR,
                default_value = GossipListenAddr::default_as_str())]
    listen_gossip: SocketAddr,
    /// Start the supervisor in local mode
    #[structopt(name = "LOCAL_GOSSIP_MODE",
                long = "local-gossip-mode",
                conflicts_with_all = &["LISTEN_GOSSIP", "PEER", "PEER_WATCH_FILE"])]
    local_gossip_mode: bool,
    /// The listen address for the HTTP Gateway
    #[structopt(name = "LISTEN_HTTP",
                long = "listen-http",
                env = HttpListenAddr::ENVVAR,
                default_value = HttpListenAddr::default_as_str())]
    listen_http: SocketAddr,
    /// Disable the HTTP Gateway completely
    #[structopt(name = "HTTP_DISABLE", long = "http-disable", short = "D")]
    http_disable: bool,
    /// The listen address for the Control Gateway. If not specified, the value will be taken from
    /// the HAB_LISTEN_CTL environment variable if defined
    #[structopt(name = "LISTEN_CTL",
                long = "listen-ctl",
                env = ListenCtlAddr::ENVVAR,
                default_value = ListenCtlAddr::default_as_str())]
    listen_ctl: SocketAddr,
    /// The organization that the Supervisor and its subsequent services are part of
    #[structopt(name = "ORGANIZATION", long = "org")]
    organization: Option<String>,
    /// The listen address of one or more initial peers (IP[:PORT])
    #[structopt(name = "PEER", long = "peer")]
    #[serde(default)]
    // TODO (DM): This could probably be a different type for better validation (Vec<SockAddr>?)
    peer: Vec<String>,
    /// If this Supervisor is a permanent peer
    #[structopt(name = "PERMANENT_PEER", long = "permanent-peer", short = "I")]
    permanent_peer: bool,
    /// Watch this file for connecting to the ring
    #[structopt(name = "PEER_WATCH_FILE",
                long = "peer-watch-file",
                conflicts_with = "PEER")]
    peer_watch_file: Option<PathBuf>,
    #[structopt(flatten)]
    #[serde(flatten)]
    cache_key_path: CacheKeyPath,
    /// The name of the ring used by the Supervisor when running with wire encryption. (ex: hab sup
    /// run --ring myring)
    #[structopt(name = "RING",
                long = "ring",
                short = "r",
                env = RING_ENVVAR,
                conflicts_with = "RING_KEY")]
    ring: Option<String>,
    /// The contents of the ring key when running with wire encryption. (Note: This option is
    /// explicitly undocumented and for testing purposes only. Do not use it in a production
    /// system. Use the corresponding environment variable instead.) (ex: hab sup run --ring-key
    /// 'SYM-SEC-1 foo-20181113185935 GCrBOW6CCN75LMl0j2V5QqQ6nNzWm6and9hkKBSUFPI=')
    #[structopt(name = "RING_KEY",
                long = "ring-key",
                env = RING_KEY_ENVVAR,
                hidden = true,
                conflicts_with = "RING")]
    ring_key: Option<String>,
    /// Use package config from this path, rather than the package itself
    #[structopt(name = "CONFIG_DIR", long = "config-from")]
    config_dir: Option<PathBuf>,
    /// Enable automatic updates for the Supervisor itself
    #[structopt(name = "AUTO_UPDATE", long = "auto-update", short = "A")]
    auto_update: bool,
    /// Used for enabling TLS for the HTTP gateway. Read private key from KEY_FILE. This should be
    /// a RSA private key or PKCS8-encoded private key, in PEM format
    #[structopt(name = "KEY_FILE", long = "key", requires = "CERT_FILE")]
    key_file: Option<PathBuf>,
    /// Used for enabling TLS for the HTTP gateway. Read server certificates from CERT_FILE. This
    /// should contain PEM-format certificates in the right order (the first certificate should
    /// certify KEY_FILE, the last should be a root CA)
    #[structopt(name = "CERT_FILE", long = "certs", requires = "KEY_FILE")]
    cert_file: Option<PathBuf>,
    /// Used for enabling client-authentication with TLS for the HTTP gateway. Read CA certificate
    /// from CA_CERT_FILE. This should contain PEM-format certificate that can be used to validate
    /// client requests
    #[structopt(name = "CA_CERT_FILE",
                long = "ca-certs",
                requires_all = &["CERT_FILE", "KEY_FILE"])]
    ca_cert_file: Option<PathBuf>,
    /// Load the given Habitat package as part of the Supervisor startup specified by a package
    /// identifier (ex: core/redis) or filepath to a Habitat Artifact (ex:
    /// /home/core-redis-3.0.7-21120102031201-x86_64-linux.hart)
    // TODO (DM): We could probably do better validation here
    #[structopt(name = "PKG_IDENT_OR_ARTIFACT")]
    pkg_ident_or_artifact: Option<String>,
    /// Verbose output; shows file and line/column numbers
    #[structopt(name = "VERBOSE", short = "v")]
    verbose: bool,
    /// Turn ANSI color off
    #[structopt(name = "NO_COLOR", long = "no-color")]
    no_color: bool,
    /// Use structured JSON logging for the Supervisor. Implies NO_COLOR
    #[structopt(name = "JSON", long = "json-logging")]
    json_logging: bool,
    /// The IPv4 address to use as the `sys.ip` template variable. If this argument is not set, the
    /// supervisor tries to dynamically determine an IP address. If that fails, the supervisor
    /// defaults to using `127.0.0.1`
    #[structopt(name = "SYS_IP_ADDRESS", long = "sys-ip-address")]
    sys_ip_address: Option<Ipv4Addr>,
    /// The name of the application for event stream purposes. This will be attached to all events
    /// generated by this Supervisor
    #[structopt(name = "EVENT_STREAM_APPLICATION", long = "event-stream-application")]
    event_stream_application: Option<String>,
    /// The name of the environment for event stream purposes. This will be attached to all events
    /// generated by this Supervisor
    #[structopt(name = "EVENT_STREAM_ENVIRONMENT", long = "event-stream-environment")]
    event_stream_environment: Option<String>,
    /// How long in seconds to wait for an event stream connection before exiting the Supervisor.
    /// Set to '0' to immediately start the Supervisor and continue running regardless of the
    /// initial connection status
    #[structopt(name = "EVENT_STREAM_CONNECT_TIMEOUT",
                long = "event-stream-connect-timeout",
                default_value = "0",
                env = EventStreamConnectMethod::ENVVAR)]
    event_stream_connect_timeout: u64,
    /// The event stream connection string (host:port) used by this Supervisor to send events to
    /// Chef Automate. This enables the event stream and requires --event-stream-application,
    /// --event-stream-environment, and --event-stream-token also be set
    #[structopt(name = "EVENT_STREAM_URL",
                long = "event-stream-url",
                requires_all = &["EVENT_STREAM_APPLICATION", 
                                 "EVENT_STREAM_ENVIRONMENT",
                                 AutomateAuthToken::ARG_NAME])]
    event_stream_url: Option<EventStreamAddress>,
    /// The name of the site where this Supervisor is running for event stream purposes
    #[structopt(name = "EVENT_STREAM_SITE", long = "event-stream-site")]
    event_stream_site: Option<String>,
    /// The authentication token for connecting the event stream to Chef Automate
    #[structopt(name = "EVENT_STREAM_TOKEN",
                long = "event-stream-token",
                env = AutomateAuthToken::ENVVAR,
                validator = AutomateAuthToken::validate)]
    automate_auth_token: Option<String>,
    /// An arbitrary key-value pair to add to each event generated by this Supervisor
    // TODO: This should be a different types
    #[structopt(name = "EVENT_STREAM_METADATA",
                long = "event-meta",
                validator = EventStreamMetadata::validate)]
    #[serde(default)]
    event_meta: Vec<String>,
    /// The path to Chef Automate's event stream certificate in PEM format used to establish a TLS
    /// connection
    #[structopt(name = "EVENT_STREAM_SERVER_CERTIFICATE",
                long = "event-stream-server-certificate",
                validator = EventStreamServerCertificate::validate)]
    event_stream_server_certificate: Option<String>,
    /// Automatically cleanup old packages.
    ///
    /// The Supervisor will automatically cleanup old packages only keeping the
    /// `NUM_LATEST_PACKAGES_TO_KEEP` latest packages. If this argument is not specified, no
    /// automatic package cleanup is performed.
    #[structopt(name = "NUM_LATEST_PACKAGES_TO_KEEP",
                long = "keep-latest-packages",
                env = "HAB_KEEP_LATEST_PACKAGES")]
    keep_latest_packages: Option<usize>,
    #[structopt(flatten)]
    #[serde(flatten)]
    shared_load: SharedLoad,
}

#[derive(ConfigOpt, StructOpt)]
#[structopt(no_version)]
/// Commands relating to a Habitat Supervisor's Control Gateway secret
pub enum Secret {
    /// Generate a secret key to use as a Supervisor's Control Gateway secret
    Generate,
}
