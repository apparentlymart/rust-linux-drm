use linux_drm::{
    event::{DrmEvent, GenericDrmEvent},
    modeset::{
        AtomicCommitFlags, CardResources, ConnectionState, ConnectorId, ConnectorState, CrtcId,
        DumbBuffer, DumbBufferRequest, ModeInfo, ObjectId, PlaneId, PropertyId,
    },
    result::Error,
    Card, ClientCap, DeviceCap,
};
use std::{
    borrow::Cow,
    env,
    ffi::{CStr, CString},
};

fn main() -> std::io::Result<()> {
    let card_path = card_path();
    let mut card = Card::open(card_path).map_err(map_init_err)?;
    card.become_master().map_err(map_err)?;

    {
        let name = card.driver_name().map_err(map_err)?;
        let name = String::from_utf8_lossy(&name);
        println!("Driver name: {name}");
    }

    if card
        .get_device_cap(DeviceCap::DumbBuffer)
        .map_err(map_err)?
        == 0
    {
        return Err(std::io::Error::other(
            "device does not support 'dumb buffers'",
        ));
    } else {
        println!("Device supports 'dumb buffers'");
    }

    card.set_client_cap(ClientCap::UniversalPlanes, 1)
        .map_err(map_err)?;
    card.set_client_cap(ClientCap::Atomic, 1).map_err(map_err)?;

    display_demo(&mut card).map_err(map_err)
}

fn card_path<'a>() -> Cow<'a, CStr> {
    static DEFAULT_PATH: &'static CStr = c"/dev/dri/card0";

    let mut args = env::args();
    if !args.next().is_some() {
        // skip the executable name
        return Cow::Borrowed(DEFAULT_PATH);
    }

    args.next().map_or(Cow::Borrowed(DEFAULT_PATH), |s| {
        Cow::Owned(CString::new(s).unwrap())
    })
}

fn display_demo(card: &mut Card) -> Result<(), Error> {
    let mut outputs = prepare_outputs(&card)?;
    let mut req = linux_drm::modeset::AtomicRequest::new();

    for output in &mut outputs {
        println!("preparing output {output:#?}");
        let conn = card.connector_state(output.conn_id)?;

        let mode = &output.mode;
        let mode_name = String::from_utf8_lossy(&mode.name);
        println!(
            "{:?} connector uses {mode_name} ({}x{}@{}Hz)",
            conn.connector_type, mode.hdisplay, mode.vdisplay, mode.vrefresh,
        );

        let rows = output.db.height() as usize;
        let pitch = output.db.pitch() as usize;
        let data = output.db.buffer_mut();
        for i in 0..rows {
            if (i % 8) > 3 {
                let row = &mut data[(i * pitch)..(i * pitch) + pitch];
                row.fill(0xff);
            }
        }

        println!(
            "configuring CRTC {:?} for framebuffer {:?} and mode {mode_name} on connector {:?}",
            output.crtc_id,
            output.db.framebuffer_id(),
            conn.id
        );

        req.set_property(
            ObjectId::Connector(output.conn_id),
            output.conn_prop_ids.crtc_id,
            output.crtc_id,
        );
        req.set_property(
            ObjectId::Crtc(output.crtc_id),
            output.crtc_prop_ids.active,
            true,
        );
        req.set_property(
            ObjectId::Plane(output.plane_id),
            output.plane_prop_ids.fb_id,
            output.db.framebuffer_id(),
        );
        req.set_property(
            ObjectId::Plane(output.plane_id),
            output.plane_prop_ids.crtc_id,
            output.crtc_id,
        );
        req.set_property(
            ObjectId::Plane(output.plane_id),
            output.plane_prop_ids.crtc_x,
            0,
        );
        req.set_property(
            ObjectId::Plane(output.plane_id),
            output.plane_prop_ids.crtc_y,
            0,
        );
        req.set_property(
            ObjectId::Plane(output.plane_id),
            output.plane_prop_ids.crtc_w,
            output.db.width(),
        );
        req.set_property(
            ObjectId::Plane(output.plane_id),
            output.plane_prop_ids.crtc_h,
            output.db.height(),
        );
        req.set_property(
            ObjectId::Plane(output.plane_id),
            output.plane_prop_ids.src_x,
            0,
        );
        req.set_property(
            ObjectId::Plane(output.plane_id),
            output.plane_prop_ids.src_y,
            0,
        );
        req.set_property(
            ObjectId::Plane(output.plane_id),
            output.plane_prop_ids.src_w,
            (output.db.width() as u64) << 16,
        );
        req.set_property(
            ObjectId::Plane(output.plane_id),
            output.plane_prop_ids.src_h,
            (output.db.height() as u64) << 16,
        );
    }

    println!("atomic commit {req:#?}");
    card.atomic_commit(
        &req,
        AtomicCommitFlags::ALLOW_MODESET | AtomicCommitFlags::PAGE_FLIP_EVENT,
        0,
    )?;

    let mut evt_buf = vec![0_u8; 1024];
    loop {
        println!("waiting for events (send SIGINT to exit)");
        for evt in card.read_events(&mut evt_buf)? {
            println!("event {evt:?}");
            match evt {
                DrmEvent::Generic(GenericDrmEvent::FlipComplete(_)) => {
                    // In a real program this would be a time place to draw the next frame
                    // for the reported crtc.
                }
                _ => {
                    // Ignore any unrecognized event types.
                }
            }
        }
    }
}

fn prepare_outputs(card: &Card) -> Result<Vec<Output>, Error> {
    println!("preparing outputs");

    let resources = card.resources()?;
    let mut outputs = Vec::<Output>::new();

    for id in resources.connector_ids.iter().copied() {
        println!("preparing output for connector #{id:?}");

        let conn = card.connector_state(id)?;
        if conn.connection_state != ConnectionState::Connected {
            println!("ignoring unconnected connector {id:?}");
            continue;
        }
        if conn.current_encoder_id.0 == 0 {
            println!("ignoring encoderless connector {id:?}");
            continue;
        }
        if conn.modes.len() == 0 {
            println!("ignoring modeless connector {id:?}");
            continue;
        }

        let output = prepare_output(card, conn, &resources)?;
        outputs.push(output);
    }

    Ok(outputs)
}

fn prepare_output(
    card: &Card,
    conn: ConnectorState,
    resources: &CardResources,
) -> Result<Output, Error> {
    if conn.current_encoder_id.0 == 0 {
        // It could be reasonable to go hunting for a suitable encoder and
        // CRTC to activate this connector, but for this simple example
        // we'll just use whatever connectors are already producing some
        // output and keep using whatever modes they are currently in.
        return Err(Error::NotSupported);
    }
    let _ = resources; // (don't actually need this when we're just using the already-selected encoder/crtc)

    let enc = card.encoder_state(conn.current_encoder_id)?;
    let crtc_id = enc.current_crtc_id;
    let crtc = card.crtc_state(crtc_id)?;
    let mode = crtc.mode;
    let db = card.create_dumb_buffer(DumbBufferRequest {
        width: mode.hdisplay as u32,
        height: mode.vdisplay as u32,
        depth: 24,
        bpp: 32,
    })?;

    // We need to find the primary plane that's currently assigned to this CRTC.
    // The following is not really a correct way to do it, but it'll work for
    // now just to test if anything is working here at all. (This makes some
    // assumptions about how the card is already configured which might not
    // actually hold in practice.)
    let mut chosen_plane_id: Option<PlaneId> = None;
    for plane_id in resources.plane_ids.iter().copied() {
        let plane = card.plane_state(plane_id)?;
        if plane.crtc_id == crtc_id {
            chosen_plane_id = Some(plane_id);
            break;
        }
    }
    let Some(chosen_plane_id) = chosen_plane_id else {
        return Err(Error::NonExist);
    };

    println!("collecting properties");
    let conn_prop_ids = ConnectorPropIds::new(conn.id, card)?;
    let crtc_prop_ids = CrtcPropIds::new(crtc_id, card)?;
    let plane_prop_ids = PlanePropIds::new(chosen_plane_id, card)?;

    println!("collected properties");
    Ok(Output {
        conn_id: conn.id,
        conn_prop_ids,
        crtc_id,
        crtc_prop_ids,
        plane_id: chosen_plane_id,
        plane_prop_ids,
        mode,
        db,
    })
}

#[derive(Debug)]
struct Output {
    db: DumbBuffer,
    mode: ModeInfo,
    conn_id: ConnectorId,
    conn_prop_ids: ConnectorPropIds,
    crtc_id: CrtcId,
    crtc_prop_ids: CrtcPropIds,
    plane_id: PlaneId,
    plane_prop_ids: PlanePropIds,
}

#[derive(Debug)]
struct ConnectorPropIds {
    crtc_id: PropertyId,
}

impl ConnectorPropIds {
    pub fn new(conn_id: ConnectorId, card: &linux_drm::Card) -> Result<Self, Error> {
        let mut ret: Self = unsafe { core::mem::zeroed() };
        card.each_object_property_meta(conn_id, |meta, _| ret.populate_from(meta))?;
        Ok(ret)
    }

    pub fn populate_from<'card>(&mut self, from: linux_drm::modeset::ObjectPropMeta<'card>) {
        match from.name() {
            "CRTC_ID" => self.crtc_id = from.property_id(),
            _ => {}
        }
    }
}

#[derive(Debug)]
struct CrtcPropIds {
    active: PropertyId,
}

impl CrtcPropIds {
    pub fn new(crtc_id: CrtcId, card: &linux_drm::Card) -> Result<Self, Error> {
        let mut ret: Self = unsafe { core::mem::zeroed() };
        card.each_object_property_meta(linux_drm::modeset::ObjectId::Crtc(crtc_id), |meta, _| {
            ret.populate_from(meta)
        })?;
        Ok(ret)
    }

    pub fn populate_from<'card>(&mut self, from: linux_drm::modeset::ObjectPropMeta<'card>) {
        match from.name() {
            "ACTIVE" => self.active = from.property_id(),
            _ => {}
        }
    }
}

#[derive(Debug)]
struct PlanePropIds {
    typ: PropertyId,
    fb_id: PropertyId,
    crtc_id: PropertyId,
    crtc_x: PropertyId,
    crtc_y: PropertyId,
    crtc_w: PropertyId,
    crtc_h: PropertyId,
    src_x: PropertyId,
    src_y: PropertyId,
    src_w: PropertyId,
    src_h: PropertyId,
}

impl PlanePropIds {
    pub fn new(plane_id: PlaneId, card: &linux_drm::Card) -> Result<Self, Error> {
        let mut ret: Self = unsafe { core::mem::zeroed() };
        card.each_object_property_meta(plane_id, |meta, _| ret.populate_from(meta))?;
        Ok(ret)
    }

    pub fn populate_from<'card>(&mut self, from: linux_drm::modeset::ObjectPropMeta<'card>) {
        let field: &mut PropertyId = match from.name() {
            "type" => &mut self.typ,
            "FB_ID" => &mut self.fb_id,
            "CRTC_ID" => &mut self.crtc_id,
            "CRTC_X" => &mut self.crtc_x,
            "CRTC_Y" => &mut self.crtc_y,
            "CRTC_W" => &mut self.crtc_w,
            "CRTC_H" => &mut self.crtc_h,
            "SRC_X" => &mut self.src_x,
            "SRC_Y" => &mut self.src_y,
            "SRC_W" => &mut self.src_w,
            "SRC_H" => &mut self.src_h,
            _ => return,
        };
        *field = from.property_id()
    }
}

fn map_init_err(e: linux_drm::result::InitError) -> std::io::Error {
    let e: linux_io::result::Error = e.into();
    e.into_std_io_error()
}

fn map_err(e: linux_drm::result::Error) -> std::io::Error {
    let e: linux_io::result::Error = e.into();
    e.into_std_io_error()
}
