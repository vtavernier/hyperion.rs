use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use serde_derive::{Deserialize, Serialize};
use validator::Validate;

use crate::{api::types::PriorityInfo, component::ComponentName, models::Color as RgbColor};

/// Change color adjustement values
#[derive(Debug, Deserialize, Validate)]
pub struct Adjustment {
    #[validate]
    pub adjustment: ChannelAdjustment,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ChannelAdjustment {
    pub id: Option<String>,
    pub white: RgbColor,
    pub red: RgbColor,
    pub green: RgbColor,
    pub blue: RgbColor,
    pub cyan: RgbColor,
    pub magenta: RgbColor,
    pub yellow: RgbColor,
    #[validate(range(min = 0, max = 100))]
    pub backlight_threshold: u32,
    pub backlight_colored: bool,
    #[validate(range(min = 0, max = 100))]
    pub brightness: u32,
    #[validate(range(min = 0, max = 100))]
    pub brightness_compensation: u32,
    #[validate(range(min = 0.1, max = 5.0))]
    pub gamma_red: f32,
    #[validate(range(min = 0.1, max = 5.0))]
    pub gamma_green: f32,
    #[validate(range(min = 0.1, max = 5.0))]
    pub gamma_blue: f32,
}

impl From<crate::models::ChannelAdjustment> for ChannelAdjustment {
    fn from(adj: crate::models::ChannelAdjustment) -> Self {
        Self {
            id: Some(adj.id),
            white: adj.white,
            red: adj.red,
            green: adj.green,
            blue: adj.blue,
            cyan: adj.cyan,
            magenta: adj.magenta,
            yellow: adj.yellow,
            backlight_threshold: adj.backlight_threshold,
            backlight_colored: adj.backlight_colored,
            brightness: adj.brightness,
            brightness_compensation: adj.brightness_compensation,
            gamma_red: adj.gamma_red,
            gamma_green: adj.gamma_green,
            gamma_blue: adj.gamma_blue,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthorizeCommand {
    RequestToken,
    CreateToken,
    RenameToken,
    DeleteToken,
    GetTokenList,
    Logout,
    Login,
    TokenRequired,
    AdminRequired,
    NewPasswordRequired,
    NewPassword,
    AnswerRequest,
    GetPendingTokenRequests,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Authorize {
    pub subcommand: AuthorizeCommand,
    #[validate(length(min = 8))]
    pub password: Option<String>,
    #[validate(length(min = 8))]
    pub new_password: Option<String>,
    #[validate(length(min = 36))]
    pub token: Option<String>,
    #[validate(length(min = 5))]
    pub comment: Option<String>,
    #[validate(length(min = 5, max = 5))]
    pub id: Option<String>,
    pub accept: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct Clear {
    #[validate(range(min = -1, max = 253))]
    pub priority: i32,
}

#[derive(Debug, Deserialize, Validate)]
pub struct Color {
    #[validate(range(min = 1, max = 253))]
    pub priority: i32,
    /// Duration in miliseconds
    #[validate(range(min = 0))]
    pub duration: Option<i32>,
    /// Origin for the command
    #[validate(length(min = 4, max = 20))]
    pub origin: Option<String>,
    pub color: RgbColor,
}

#[derive(Debug, Deserialize)]
pub struct ComponentStatus {
    pub component: ComponentName,
    pub state: bool,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ComponentState {
    pub componentstate: ComponentStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigCommand {
    SetConfig,
    GetConfig,
    GetSchema,
    Reload,
}

#[derive(Debug, Deserialize, Validate)]
pub struct Config {
    pub subcommand: ConfigCommand,
    #[serde(default)]
    pub config: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct EffectCreate {
    pub name: String,
    pub script: String,
    pub args: serde_json::Map<String, serde_json::Value>,
    #[serde(deserialize_with = "crate::serde::from_base64")]
    pub image_data: Vec<u8>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct EffectDelete {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct EffectRequest {
    /// Effect name
    pub name: String,
    /// Effect parameters
    #[serde(default)]
    pub args: serde_json::Map<String, serde_json::Value>,
}

/// Trigger an effect by name
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Effect {
    #[validate(range(min = 1, max = 253))]
    pub priority: i32,
    #[validate(range(min = 0))]
    pub duration: Option<i32>,
    #[validate(length(min = 4, max = 20))]
    pub origin: Option<String>,
    pub effect: EffectRequest,
    pub python_script: Option<String>,
    #[serde(deserialize_with = "crate::serde::from_base64")]
    pub image_data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Auto,
}

impl Default for ImageFormat {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct Image {
    #[validate(range(min = 1, max = 253))]
    pub priority: i32,
    #[validate(length(min = 4, max = 20))]
    pub origin: Option<String>,
    #[validate(range(min = 0))]
    pub duration: Option<i32>,
    pub imagewidth: u32,
    pub imageheight: u32,
    #[serde(deserialize_with = "crate::serde::from_base64")]
    pub imagedata: Vec<u8>,
    #[serde(default)]
    pub format: ImageFormat,
    #[validate(range(min = 25, max = 2000))]
    pub scale: Option<i32>,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstanceCommand {
    CreateInstance,
    DeleteInstance,
    StartInstance,
    StopInstance,
    SaveName,
    SwitchTo,
}

#[derive(Debug, Deserialize, Validate)]
pub struct Instance {
    pub subcommand: InstanceCommand,
    #[validate(range(max = 255))]
    pub instance: Option<u32>,
    #[validate(length(min = 5))]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LedColorsSubcommand {
    #[serde(rename = "ledstream-stop")]
    LedStreamStop,
    #[serde(rename = "ledstream-start")]
    LedStreamStart,
    TestLed,
    #[serde(rename = "imagestream-start")]
    ImageStreamStart,
    #[serde(rename = "imagestream-stop")]
    ImageStreamStop,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LedColors {
    pub subcommand: LedColorsSubcommand,
    pub oneshot: Option<bool>,
    #[validate(range(min = 50))]
    pub interval: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LedDeviceCommand {
    Discover,
    GetProperties,
    Identify,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LedDevice {
    pub subcommand: LedDeviceCommand,
    pub led_device_type: String,
    pub params: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoggingCommand {
    Stop,
    Start,
    Update,
}

#[derive(Debug, Deserialize, Validate)]
pub struct Logging {
    pub subcommand: LoggingCommand,
    pub oneshot: Option<bool>,
    pub interval: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingType {
    MulticolorMean,
    UnicolorMean,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Processing {
    pub mapping_type: MappingType,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ServerInfoRequest {
    pub subscribe: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SourceSelect {
    #[validate(range(min = 0, max = 255))]
    pub priority: i32,
    pub auto: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VideoMode {
    #[serde(rename = "2D")]
    Mode2D,
    #[serde(rename = "3DSBS")]
    Mode3DSBS,
    #[serde(rename = "3DTAB")]
    Mode3DTAB,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct VideoModeRequest {
    pub video_mode: VideoMode,
}

/// Incoming Hyperion JSON command
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", tag = "command")]
pub enum HyperionCommand {
    Adjustment(Adjustment),
    Authorize(Authorize),
    Clear(Clear),
    /// Deprecated
    ClearAll,
    Color(Color),
    ComponentState(ComponentState),
    Config(Config),
    #[serde(rename = "create-effect")]
    EffectCreate(EffectCreate),
    #[serde(rename = "delete-effect")]
    EffectDelete(EffectDelete),
    Effect(Effect),
    Image(Image),
    Instance(Instance),
    LedColors(LedColors),
    LedDevice(LedDevice),
    Logging(Logging),
    Processing(Processing),
    ServerInfo(ServerInfoRequest),
    SourceSelect(SourceSelect),
    SysInfo,
    VideoMode(VideoModeRequest),
}

/// Incoming Hyperion JSON message
#[derive(Debug, Deserialize)]
pub struct HyperionMessage {
    /// Request identifier
    pub tan: Option<i32>,
    #[serde(flatten)]
    pub command: HyperionCommand,
}

impl Validate for HyperionMessage {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match &self.command {
            HyperionCommand::Adjustment(adjustment) => adjustment.validate(),
            HyperionCommand::Authorize(authorize) => authorize.validate(),
            HyperionCommand::Clear(clear) => clear.validate(),
            HyperionCommand::ClearAll => Ok(()),
            HyperionCommand::Color(color) => color.validate(),
            HyperionCommand::ComponentState(component_state) => component_state.validate(),
            HyperionCommand::Config(config) => config.validate(),
            HyperionCommand::EffectCreate(effect_create) => effect_create.validate(),
            HyperionCommand::EffectDelete(effect_delete) => effect_delete.validate(),
            HyperionCommand::Effect(effect) => effect.validate(),
            HyperionCommand::Image(image) => image.validate(),
            HyperionCommand::Instance(instance) => instance.validate(),
            HyperionCommand::LedColors(led_colors) => led_colors.validate(),
            HyperionCommand::LedDevice(led_device) => led_device.validate(),
            HyperionCommand::Logging(logging) => logging.validate(),
            HyperionCommand::Processing(processing) => processing.validate(),
            HyperionCommand::ServerInfo(server_info) => server_info.validate(),
            HyperionCommand::SourceSelect(source_select) => source_select.validate(),
            HyperionCommand::SysInfo => Ok(()),
            HyperionCommand::VideoMode(video_mode) => video_mode.validate(),
        }
    }
}

/// Effect definition details
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EffectDefinition {
    /// User-friendly name of the effect
    pub name: String,
    /// Path to the effect definition file
    pub file: String,
    /// Path to the script to run
    pub script: String,
    /// Extra script arguments
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum LedDeviceClass {
    Dummy,
    PhilipsHue,
    #[serde(rename = "Ws2812SPI")]
    Ws2812Spi,
}

#[derive(Debug, Serialize)]
pub struct LedDevicesInfo {
    pub available: Vec<LedDeviceClass>,
}

impl LedDevicesInfo {
    pub fn new() -> Self {
        use LedDeviceClass::*;

        Self {
            available: vec![Dummy, PhilipsHue, Ws2812Spi],
        }
    }
}

#[derive(Debug, Clone)]
pub enum GrabberClass {
    AmLogic,
    DirectX,
    Dispmanx,
    Framebuffer,
    OSX,
    Qt,
    V4L2 { device: PathBuf },
    X11,
    Xcb,
}

impl std::fmt::Display for GrabberClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrabberClass::AmLogic => write!(f, "AmLogic"),
            GrabberClass::DirectX => write!(f, "DirectX"),
            GrabberClass::Dispmanx => write!(f, "Dispmanx"),
            GrabberClass::Framebuffer => write!(f, "FrameBuffer"),
            GrabberClass::OSX => write!(f, "OSX FrameGrabber"),
            GrabberClass::Qt => write!(f, "Qt"),
            GrabberClass::V4L2 { device } => write!(f, "V4L2:{}", device.display()),
            GrabberClass::X11 => write!(f, "X11"),
            GrabberClass::Xcb => write!(f, "Xcb"),
        }
    }
}

impl serde::ser::Serialize for GrabberClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Serialize)]
pub struct GrabbersInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<GrabberClass>,
    pub available: Vec<GrabberClass>,
}

impl GrabbersInfo {
    pub fn new() -> Self {
        Self {
            // TODO: Report active grabber
            active: None,
            // TODO: Add grabbers when they are implemented
            available: vec![], // TODO: Add v4l2_properties for available V4L2 devices
        }
    }
}

/// Hyperion server info
#[derive(Debug, Serialize)]
pub struct ServerInfo {
    /// Priority information
    pub priorities: Vec<PriorityInfo>,
    pub priorities_autoselect: bool,
    /// Color adjustment information
    pub adjustment: Vec<ChannelAdjustment>,
    /// Effect information
    pub effects: Vec<EffectDefinition>,
    /// LED device information
    #[serde(rename = "ledDevices")]
    pub led_devices: LedDevicesInfo,
    /// Grabber information
    pub grabbers: GrabbersInfo,
    /// Current video mode
    #[serde(rename = "videomode")]
    pub video_mode: VideoMode,
    // TODO: components field
    // TODO: imageToLedMappingType field
    // TODO: sessions field
    #[serde(rename = "instance")]
    pub instances: Vec<InstanceInfo>,
    // TODO: leds field
    pub hostname: String,
    // TODO: (legacy) transform field
    // TODO: (legacy) activeEffects field
    // TODO: (legacy) activeLedColor field
}

/// Hyperion build info
#[derive(Default, Debug, Serialize)]
pub struct BuildInfo {
    /// Version number
    pub version: String,
    /// Build time
    pub time: String,
}

impl BuildInfo {
    pub fn new() -> Self {
        Self {
            version: version(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InstanceInfo {
    pub friendly_name: String,
    pub instance: i32,
    pub running: bool,
}

impl From<&crate::models::Instance> for InstanceInfo {
    fn from(config: &crate::models::Instance) -> Self {
        Self {
            friendly_name: config.friendly_name.clone(),
            instance: config.id,
            // TODO: Runtime state might differ from config enabled
            running: config.enabled,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HyperionResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tan: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    info: Option<HyperionResponseInfo>,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub kernel_type: String,
    pub kernel_version: String,
    pub architecture: String,
    pub cpu_model_name: String,
    pub cpu_model_type: String,
    pub cpu_hardware: String,
    pub cpu_revision: String,
    pub word_size: String,
    pub product_type: String,
    pub product_version: String,
    pub pretty_name: String,
    pub host_name: String,
    pub domain_name: String,
    pub qt_version: String,
    pub py_version: String,
}

impl SystemInfo {
    pub fn new() -> Self {
        // TODO: Fill in other fields
        Self {
            kernel_type: if cfg!(target_os = "windows") {
                "winnt".to_owned()
            } else if cfg!(target_os = "linux") {
                Command::new("uname")
                    .args(&["-s"])
                    .stdout(Stdio::piped())
                    .output()
                    .ok()
                    .and_then(|output| String::from_utf8(output.stdout).ok())
                    .map(|output| output.trim().to_ascii_lowercase())
                    .unwrap_or_else(String::new)
            } else {
                String::new()
            },
            kernel_version: if cfg!(target_os = "linux") {
                Command::new("uname")
                    .args(&["-r"])
                    .stdout(Stdio::piped())
                    .output()
                    .ok()
                    .and_then(|output| String::from_utf8(output.stdout).ok())
                    .map(|output| output.trim().to_ascii_lowercase())
                    .unwrap_or_else(String::new)
            } else {
                String::new()
            },
            host_name: hostname(),
            ..Default::default()
        }
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HyperionInfo {
    pub version: String,
    pub build: String,
    pub gitremote: String,
    pub time: String,
    pub id: uuid::Uuid,
    pub read_only_mode: bool,
}

impl HyperionInfo {
    pub fn new(id: uuid::Uuid) -> Self {
        // TODO: Fill in other fields
        Self {
            // We emulate hyperion.ng 2.0.0-alpha.8
            version: "2.0.0-alpha.8".to_owned(),
            build: version(),
            id,
            read_only_mode: false,
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SysInfo {
    pub system: SystemInfo,
    pub hyperion: HyperionInfo,
}

impl SysInfo {
    pub fn new(id: uuid::Uuid) -> Self {
        Self {
            system: SystemInfo::new(),
            hyperion: HyperionInfo::new(id),
        }
    }
}

/// Hyperion JSON response
#[derive(Debug, Serialize)]
#[serde(tag = "command", content = "info")]
pub enum HyperionResponseInfo {
    /// Server information response
    #[serde(rename = "serverinfo")]
    ServerInfo(ServerInfo),
    /// TokenRequired response
    #[serde(rename = "auth-tokenRequired")]
    TokenRequired {
        /// true if an auth token required
        required: bool,
    },
    /// SysInfo response
    #[serde(rename = "sysinfo")]
    SysInfo(SysInfo),
}

impl HyperionResponse {
    fn success_info(tan: Option<i32>, info: HyperionResponseInfo) -> Self {
        Self {
            success: true,
            tan,
            error: None,
            info: Some(info),
        }
    }

    /// Return a success response
    pub fn success(tan: Option<i32>) -> Self {
        Self {
            success: true,
            tan,
            error: None,
            info: None,
        }
    }

    /// Return an error response
    pub fn error(tan: Option<i32>, error: &impl std::fmt::Display) -> Self {
        Self {
            success: false,
            tan,
            error: Some(error.to_string()),
            info: None,
        }
    }

    /// Return a server information response
    pub fn server_info(
        tan: Option<i32>,
        priorities: Vec<PriorityInfo>,
        adjustment: Vec<ChannelAdjustment>,
        effects: Vec<EffectDefinition>,
        instances: Vec<InstanceInfo>,
    ) -> Self {
        Self::success_info(
            tan,
            HyperionResponseInfo::ServerInfo(ServerInfo {
                priorities,
                // TODO: Actual autoselect value
                priorities_autoselect: true,
                adjustment,
                effects,
                led_devices: LedDevicesInfo::new(),
                grabbers: GrabbersInfo::new(),
                // TODO: Actual video mode
                video_mode: VideoMode::Mode2D,
                instances,
                hostname: hostname(),
            }),
        )
    }

    pub fn token_required(tan: Option<i32>, required: bool) -> Self {
        Self::success_info(tan, HyperionResponseInfo::TokenRequired { required })
    }

    pub fn sys_info(tan: Option<i32>, id: uuid::Uuid) -> Self {
        // TODO: Properly fill out this response
        Self::success_info(tan, HyperionResponseInfo::SysInfo(SysInfo::new(id)))
    }
}

fn hostname() -> String {
    hostname::get()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|_| "<unknown hostname>".to_owned())
}

fn version() -> String {
    git_version::git_version!(prefix = "hyperion.rs-", args = ["--always", "--tags"]).to_owned()
}
