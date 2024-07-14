use linux_drm::Card;

fn main() -> std::io::Result<()> {
    let card = Card::open(c"/dev/dri/card0").map_err(|e| {
        let e: linux_io::result::Error = e.into();
        e.into_std_io_error()
    })?;

    let card = card.into_master().map_err(|(e, _)| {
        let e: linux_io::result::Error = e.into();
        e.into_std_io_error()
    })?;

    todo!()
}
