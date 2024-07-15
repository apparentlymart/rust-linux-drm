use alloc::vec::Vec;

#[derive(Debug)]
pub struct CardResources {
    pub fb_ids: Vec<u32>,
    pub crtc_ids: Vec<u32>,
    pub connector_ids: Vec<u32>,
    pub encoder_ids: Vec<u32>,
    pub min_width: u32,
    pub max_width: u32,
    pub min_height: u32,
    pub max_height: u32,
}

#[derive(Debug)]
pub struct ConnectorState {
    pub id: u32,
    pub current_encoder_id: u32,
    pub connector_type: u32,
    pub connector_type_id: u32,
    pub connection_state: ConnectionState,
    pub width_mm: u32,
    pub height_mm: u32,
    pub subpixel_type: SubpixelType,
    pub modes: Vec<ModeInfo>,
    pub props: Vec<ModeProp>,
    pub available_encoder_ids: Vec<u32>,
}

#[derive(Debug)]
#[repr(u32)]
pub enum ConnectionState {
    Connected = 1,
    Disconnected = 2,
    Unknown = 3,
}

impl From<u32> for ConnectionState {
    fn from(value: u32) -> Self {
        match value {
            1 => Self::Connected,
            2 => Self::Disconnected,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug)]
pub struct EncoderState {
    pub encoder_id: u32,
    pub encoder_type: u32,
    pub current_crtc_id: u32,
    pub possible_crtcs: u32,
    pub possible_clones: u32,
}

#[derive(Debug)]
pub struct ModeInfo {
    pub name: Vec<u8>,
    pub clock: u32,
    pub hdisplay: u16,
    pub hsync_start: u16,
    pub hsync_end: u16,
    pub htotal: u16,
    pub hskew: u16,
    pub vdisplay: u16,
    pub vsync_start: u16,
    pub vsync_end: u16,
    pub vtotal: u16,
    pub vscan: u16,
    pub vrefresh: u32,
    pub flags: u32,
    pub typ: u32,
}

#[derive(Debug)]
pub struct ModeProp {
    pub prop_id: u32,
    pub value: u64,
}

#[derive(Debug)]
#[repr(u32)]
pub enum SubpixelType {
    Unknown = 1,
    HorizontalRgb = 2,
    HorizontalBgr = 3,
    VerticalRgb = 4,
    VerticalBgr = 5,
    None = 6,
}

impl From<u32> for SubpixelType {
    fn from(value: u32) -> Self {
        match value {
            2 => Self::HorizontalRgb,
            3 => Self::HorizontalBgr,
            4 => Self::VerticalRgb,
            5 => Self::VerticalBgr,
            _ => Self::Unknown,
        }
    }
}
