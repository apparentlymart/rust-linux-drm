use std::collections::{BTreeMap, HashMap};

use linux_drm::{
    modeset::{ModeProp, ObjectId, PropertyId, PropertyType},
    result::Error,
    Card, ClientCap, DeviceCap,
};

fn main() -> std::io::Result<()> {
    let mut card = Card::open(c"/dev/dri/card0").map_err(map_init_err)?;

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
    }

    card.set_client_cap(ClientCap::UniversalPlanes, 1)
        .map_err(map_err)?;
    card.set_client_cap(ClientCap::Atomic, 1).map_err(map_err)?;

    show_properties(&card).map_err(map_err)
}

fn show_properties(card: &Card) -> Result<(), Error> {
    let res = card.resources()?;

    let mut prop_meta = HashMap::<u32, PropertyMeta>::new();

    println!("");

    for conn_id in res.connector_ids {
        println!("Connector #{conn_id:?}:");
        if let Err(e) = show_object_property_list(conn_id, &mut prop_meta, card) {
            println!("  Error: {e:?}");
        }
        println!("");
    }

    for enc_id in res.encoder_ids {
        println!("Encoder #{enc_id:?}:");
        if let Err(e) = show_object_property_list(enc_id, &mut prop_meta, card) {
            println!("  Error: {e:?}");
        }
        println!("");
    }

    for crtc_id in res.crtc_ids {
        println!("CRTC #{crtc_id:?}:");
        if let Err(e) = show_object_property_list(crtc_id, &mut prop_meta, card) {
            println!("  Error: {e:?}");
        }
        println!("");
    }

    for fb_id in res.fb_ids {
        println!("Framebuffer #{fb_id:?}:");
        if let Err(e) = show_object_property_list(fb_id, &mut prop_meta, card) {
            println!("  Error: {e:?}");
        }
        println!("");
    }

    for plane_id in res.plane_ids {
        println!("Plane #{plane_id:?}:");
        if let Err(e) = show_object_property_list(plane_id, &mut prop_meta, card) {
            println!("  Error: {e:?}");
        }
        println!("");
    }

    Ok(())
}

fn show_object_property_list(
    id: impl Into<ObjectId>,
    prop_meta: &mut HashMap<u32, PropertyMeta>,
    card: &Card,
) -> Result<(), Error> {
    let props = card.object_properties(id)?;
    show_property_list(&props, prop_meta, card)?;
    Ok(())
}

fn show_property_list(
    props: &[ModeProp],
    prop_meta: &mut HashMap<u32, PropertyMeta>,
    card: &Card,
) -> Result<(), Error> {
    for prop in props {
        let meta = property_meta(prop.prop_id, prop_meta, card)?;
        print!("  {}: ", meta.name);
        match meta.typ {
            PropertyType::Enum => {
                if let Some(name) = meta.enum_names.get(&prop.value) {
                    println!("{name}")
                } else {
                    println!("out-of-range value {}", prop.value)
                }
            }
            PropertyType::Bitmask => {
                let v = prop.value;
                let mut valid = 0_u64;
                let mut printed_one = false;
                for (bit, name) in meta.enum_names.iter() {
                    let mask = 1_u64 << *bit;
                    if (v & mask) != 0 {
                        if printed_one {
                            print!(" | ");
                        }
                        print!("{name}");
                        printed_one = true;
                    }
                    valid |= mask;
                }
                let invalid = v & !valid;
                if invalid != 0 {
                    if printed_one {
                        print!(" | ");
                    }
                    print!("{invalid:#x}");
                }
                println!("");
            }
            PropertyType::Blob => {
                println!("blob #{}", prop.value)
            }
            _ => println!("{}", prop.value),
        }
    }
    Ok(())
}

fn property_meta<'a, 'card>(
    prop_id: PropertyId,
    prop_meta: &'a mut HashMap<u32, PropertyMeta>,
    card: &Card,
) -> Result<&'a PropertyMeta, Error> {
    Ok(prop_meta.entry(prop_id.0).or_insert_with(|| {
        card.property_meta(prop_id)
            .map(|meta| {
                let mut enum_names = BTreeMap::new();
                for member in meta.enum_members().unwrap() {
                    enum_names.insert(member.value(), member.name().to_string());
                }
                PropertyMeta {
                    name: meta.name().to_string(),
                    typ: meta.property_type(),
                    immutable: meta.is_immutable(),
                    values: meta.values().unwrap(),
                    enum_names,
                }
            })
            .unwrap_or(PropertyMeta {
                name: String::from("<unknown>"),
                typ: PropertyType::Unknown,
                immutable: true,
                values: Vec::new(),
                enum_names: BTreeMap::new(),
            })
    }))
}

#[derive(Debug)]
pub struct PropertyMeta {
    pub name: String,
    pub typ: PropertyType,
    pub immutable: bool,
    pub values: Vec<u64>,
    pub enum_names: BTreeMap<u64, String>,
}

fn map_init_err(e: linux_drm::result::InitError) -> std::io::Error {
    let e: linux_io::result::Error = e.into();
    e.into_std_io_error()
}

fn map_err(e: linux_drm::result::Error) -> std::io::Error {
    let e: linux_io::result::Error = e.into();
    e.into_std_io_error()
}
