use linux_drm::Card;

fn main() -> std::io::Result<()> {
    let mut card = Card::open(c"/dev/dri/card0").map_err(map_init_err)?;
    card.become_master().map_err(map_err)?;

    let mut name = vec![0_u8; 64];
    let name = &mut name[..];
    let name = card.read_driver_name(name).map_err(map_err)?;
    let name = String::from_utf8_lossy(name);
    println!("Driver name: {name}");

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
