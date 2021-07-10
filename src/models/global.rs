use serde_derive::{Deserialize, Serialize};
use validator::Validate;

use super::ServerConfig;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct FlatbuffersServer {
    pub enable: bool,
    #[validate(range(min = 1024))]
    pub port: u16,
    #[validate(range(min = 1))]
    pub timeout: u32,
}

impl Default for FlatbuffersServer {
    fn default() -> Self {
        Self {
            enable: true,
            port: 19400,
            timeout: 5,
        }
    }
}

impl ServerConfig for FlatbuffersServer {
    fn port(&self) -> u16 {
        self.port
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, deny_unknown_fields)]
pub struct Forwarder {
    pub enable: bool,
    pub json: Vec<String>,
    pub flat: Vec<String>,
}

impl Default for Forwarder {
    fn default() -> Self {
        Self {
            enable: false,
            json: vec!["127.0.0.1:19446".to_owned()],
            flat: vec!["127.0.0.1:19401".to_owned()],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum FramegrabberType {
    Auto,
    AMLogic,
    DispmanX,
    DirectX9,
    Framebuffer,
    OSX,
    QT,
    X11,
    XCB,
}

impl Default for FramegrabberType {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Framegrabber {
    #[serde(rename = "type")]
    pub ty: FramegrabberType,
    #[validate(range(min = 10))]
    pub width: u32,
    #[validate(range(min = 10))]
    pub height: u32,
    #[serde(rename = "frequency_Hz")]
    #[validate(range(min = 1))]
    pub frequency_hz: u32,
    pub crop_left: u32,
    pub crop_right: u32,
    pub crop_top: u32,
    pub crop_bottom: u32,
    #[validate(range(min = 1, max = 30))]
    pub pixel_decimation: u32,
    #[serde(default)]
    pub display: u32,
}

impl Default for Framegrabber {
    fn default() -> Self {
        Self {
            ty: Default::default(),
            width: 80,
            height: 45,
            frequency_hz: 10,
            crop_left: 0,
            crop_right: 0,
            crop_top: 0,
            crop_bottom: 0,
            pixel_decimation: 8,
            display: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum WatchedVersionBranch {
    Stable,
    Beta,
    Alpha,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct General {
    #[validate(length(min = 4, max = 20))]
    pub name: String,
    pub watched_version_branch: WatchedVersionBranch,
    pub show_opt_help: bool,
}

impl Default for General {
    fn default() -> Self {
        Self {
            name: "My Hyperion Config".to_owned(),
            watched_version_branch: WatchedVersionBranch::Stable,
            show_opt_help: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum V4L2Standard {
    NoChange,
    Pal,
    Ntsc,
    Secam,
}

impl Default for V4L2Standard {
    fn default() -> Self {
        Self::NoChange
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct GrabberV4L2 {
    pub device: String,
    pub input: i32,
    pub standard: V4L2Standard,
    pub width: u32,
    pub height: u32,
    #[validate(range(min = 1))]
    pub fps: u32,
    #[validate(range(min = 1, max = 30))]
    pub size_decimation: u32,
    pub crop_left: u32,
    pub crop_right: u32,
    pub crop_top: u32,
    pub crop_bottom: u32,
    pub cec_detection: bool,
    pub signal_detection: bool,
    #[validate(range(min = 0, max = 100))]
    pub red_signal_threshold: u32,
    #[validate(range(min = 0, max = 100))]
    pub green_signal_threshold: u32,
    #[validate(range(min = 0, max = 100))]
    pub blue_signal_threshold: u32,
    #[serde(rename = "sDVOffsetMin")]
    #[validate(range(min = 0., max = 1.))]
    pub sdv_offset_min: f32,
    #[serde(rename = "sDVOffsetMax")]
    #[validate(range(min = 0., max = 1.))]
    pub sdv_offset_max: f32,
    #[serde(rename = "sDHOffsetMin")]
    #[validate(range(min = 0., max = 1.))]
    pub sdh_offset_min: f32,
    #[serde(rename = "sDHOffsetMax")]
    #[validate(range(min = 0., max = 1.))]
    pub sdh_offset_max: f32,
}

impl Default for GrabberV4L2 {
    fn default() -> Self {
        Self {
            device: "auto".to_owned(),
            input: 0,
            standard: Default::default(),
            width: 0,
            height: 0,
            fps: 15,
            size_decimation: 6,
            crop_left: 0,
            crop_right: 0,
            crop_top: 0,
            crop_bottom: 0,
            cec_detection: false,
            signal_detection: false,
            red_signal_threshold: 5,
            green_signal_threshold: 5,
            blue_signal_threshold: 5,
            sdv_offset_min: 0.25,
            sdv_offset_max: 0.75,
            sdh_offset_min: 0.25,
            sdh_offset_max: 0.75,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, deny_unknown_fields)]
pub struct JsonServer {
    #[validate(range(min = 1024))]
    pub port: u16,
}

impl Default for JsonServer {
    fn default() -> Self {
        Self { port: 19444 }
    }
}

impl ServerConfig for JsonServer {
    fn port(&self) -> u16 {
        self.port
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum LoggerLevel {
    Silent,
    Warn,
    Verbose,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, deny_unknown_fields)]
pub struct Logger {
    pub level: LoggerLevel,
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            level: LoggerLevel::Warn,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Network {
    pub api_auth: bool,
    #[serde(default, rename = "internetAccessAPI")]
    pub internet_access_api: bool,
    #[serde(
        default,
        rename = "restrictedInternetAccessAPI",
        alias = "restirctedInternetAccessAPI"
    )]
    pub restricted_internet_access_api: bool,
    pub ip_whitelist: Vec<String>,
    pub local_api_auth: bool,
    pub local_admin_auth: bool,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            api_auth: true,
            internet_access_api: false,
            restricted_internet_access_api: false,
            ip_whitelist: vec![],
            local_api_auth: false,
            local_admin_auth: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtoServer {
    pub enable: bool,
    #[validate(range(min = 1024))]
    pub port: u16,
    #[validate(range(min = 1))]
    pub timeout: u32,
}

impl Default for ProtoServer {
    fn default() -> Self {
        Self {
            enable: true,
            port: 19445,
            timeout: 5,
        }
    }
}

impl ServerConfig for ProtoServer {
    fn port(&self) -> u16 {
        self.port
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct WebConfig {
    #[serde(rename = "document_root")]
    pub document_root: String,
    #[validate(range(min = 80))]
    pub port: u16,
    #[validate(range(min = 80))]
    pub ssl_port: u16,
    pub crt_path: String,
    pub key_path: String,
    pub key_pass_phrase: String,
    #[validate(range(min = 1))]
    pub max_sessions: u32,
}

impl WebConfig {
    pub const SYSTEM_DOCUMENT_ROOT: &'static str = "$ROOT/webconfig";
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            document_root: String::new(),
            port: 8090,
            ssl_port: 8092,
            crt_path: String::new(),
            key_path: String::new(),
            key_pass_phrase: String::new(),
            max_sessions: 4,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Hooks {
    /// Command to run when an instance is started. HYPERION_INSTANCE_ID environment variable
    /// will hold the instance id.
    pub instance_start: Vec<String>,
    /// Command to run when an instance is stopped. HYPERION_INSTANCE_ID environment variable
    /// will hold the instance id.
    pub instance_stop: Vec<String>,
    /// Command to run when an instance is activated. HYPERION_INSTANCE_ID environment variable
    /// will hold the instance id.
    pub instance_activate: Vec<String>,
    /// Command to run when an instance is deactivated. HYPERION_INSTANCE_ID environment variable
    /// will hold the instance id.
    pub instance_deactivate: Vec<String>,
    /// Command to run when hyperion.rs starts
    pub start: Vec<String>,
    /// Command to run when hyperion.rs stops
    pub stop: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct GlobalConfig {
    pub flatbuffers_server: FlatbuffersServer,
    pub forwarder: Forwarder,
    pub framegrabber: Framegrabber,
    pub general: General,
    #[serde(rename = "grabberV4L2")]
    pub grabber_v4l2: GrabberV4L2,
    pub json_server: JsonServer,
    pub logger: Logger,
    pub network: Network,
    pub proto_server: ProtoServer,
    pub web_config: WebConfig,
    pub hooks: Hooks,
}
