use std::{
    borrow::Cow,
    env,
    ffi::{CStr, CString},
};

use linux_drm::{
    event::{DrmEvent, GenericDrmEvent},
    modeset::{
        CardResources, ConnectionState, ConnectorId, ConnectorState, CrtcId, DumbBuffer,
        DumbBufferRequest, ModeInfo, PageFlipFlags,
    },
    result::Error,
    Card, DeviceCap,
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
            "configuring CRTC {:?} for framebuffer {:?} and mode {mode_name} on connection {:?}",
            output.crtc_id,
            output.db.framebuffer_id(),
            conn.id
        );
        card.set_crtc_dumb_buffer(output.crtc_id, &output.db, mode, &[output.conn_id])?;
        card.crtc_page_flip_dumb_buffer(output.crtc_id, &output.db, PageFlipFlags::EVENT)?;
    }

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
    let resources = card.resources()?;
    let mut outputs = Vec::<Output>::new();

    for id in resources.connector_ids.iter().copied() {
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
    Ok(Output {
        conn_id: conn.id,
        crtc_id,
        mode,
        db,
    })
}

#[derive(Debug)]
struct Output {
    db: DumbBuffer,
    mode: ModeInfo,
    conn_id: ConnectorId,
    crtc_id: CrtcId,
}

fn map_init_err(e: linux_drm::result::InitError) -> std::io::Error {
    let e: linux_io::result::Error = e.into();
    e.into_std_io_error()
}

fn map_err(e: linux_drm::result::Error) -> std::io::Error {
    let e: linux_io::result::Error = e.into();
    e.into_std_io_error()
}
