use linux_drm::{modeset::DumbBufferRequest, Card, ClientCap, DeviceCap};

fn main() -> std::io::Result<()> {
    let mut card = Card::open(c"/dev/dri/card0").map_err(map_init_err)?;
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

    let resources = card.resources().map_err(map_err)?;
    println!("resources: {resources:#?}");
    for id in resources.connector_ids {
        let conn = card.connector_state(id).map_err(map_err)?;
        println!("connector: {conn:#?}");
    }
    for id in resources.encoder_ids {
        let enc = card.encoder_state(id).map_err(map_err)?;
        println!("encoder: {enc:#?}");
    }
    for id in resources.crtc_ids {
        let crtc = card.crtc_state(id).map_err(map_err)?;
        println!("CRTC: {crtc:#?}");
    }

    let buf = card
        .create_dumb_buffer(DumbBufferRequest {
            width: 640,
            height: 480,
            depth: 24,
            bpp: 32,
        })
        .map_err(map_err)?;
    println!("buffer {buf:?}");

    Ok(())
}

fn map_init_err(e: linux_drm::result::InitError) -> std::io::Error {
    let e: linux_io::result::Error = e.into();
    e.into_std_io_error()
}

fn map_err(e: linux_drm::result::Error) -> std::io::Error {
    let e: linux_io::result::Error = e.into();
    e.into_std_io_error()
}
