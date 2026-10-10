//! Envelope lab: load a BricsCAD-accepted silver rewrite (an authored
//! document envelope), append ONE constructed gen_all-class leader, and
//! write the result out.
//!
//! Strict-loader interpretation:
//! - The loaded records were already proven to load (the source file opens
//!   in the strict target).
//! - If the APPENDED leader (the last entity) is accepted too, the earlier
//!   drop fault lies in the gen_all document's ENVELOPE (tables/context),
//!   not the constructed record; the two envelopes can then be diffed
//!   structurally.
//! - If the appended leader drops ("Object improperly read (<its handle>)"),
//!   the record itself is at fault after all.
//!
//! Usage: cargo run --example xleader_lab [source.dwg [out.dwg]]

use opencadcodec::entities::*;
use opencadcodec::io::dwg::DwgReader;
use opencadcodec::types::Vector3;
use opencadcodec::{CadDocument, DwgWriter, LineWeight, Transparency};

fn main() {
    let src = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "gen_all_rewrite_authored_2018_leader.dwg".to_string());
    let out = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "gen_xleader_lab.dwg".to_string());

    let mut reader = match DwgReader::from_file(&src) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("open {}: {}", src, e);
            std::process::exit(1);
        }
    };
    let mut doc: CadDocument = match reader.read() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("read {}: {}", src, e);
            std::process::exit(1);
        }
    };
    println!("loaded {}", src);

    // One constructed leader of the current gen_all class.
    let mtext_handle = match doc
        .add_entity(EntityType::MText(MText::with_value(
            "Lab note",
            Vector3::new(30.0, 12.0, 0.0),
        )))
    {
        Ok(h) => h,
        Err(e) => {
            eprintln!("mtext add: {}", e);
            std::process::exit(1);
        }
    };
    let mut leader = Leader::from_vertices(vec![
        Vector3::new(5.0, 5.0, 0.0),
        Vector3::new(15.0, 15.0, 0.0),
        Vector3::new(25.0, 15.0, 0.0),
    ]);
    leader.creation_type = LeaderCreationType::WithText;
    leader.annotation_handle = mtext_handle;
    leader.path_type = LeaderPathType::Spline;
    leader.arrow_enabled = false;
    leader.arrowhead_type = 322;
    leader.hookline_direction = HooklineDirection::Same;
    leader.dwg_unknown_bit4 = true;
    leader.text_height = 0.0;
    leader.text_width = -0.09;
    leader.dimension_style = "Standard".to_string();
    leader.common.linetype = "Continuous".to_string();
    leader.common.line_weight = LineWeight::Value(5);
    leader.common.linetype_scale = 1.5;
    leader.common.transparency = Transparency::Explicit(169);

    match doc.add_entity(EntityType::Leader(leader)) {
        Ok(leader_handle) => {
            doc.ensure_extension_dictionary(leader_handle);
            println!("appended leader at handle 0x{:X}", leader_handle.value());
            if let Some(m) = doc.get_entity(mtext_handle) {
                let _ = m;
            }
        }
        Err(e) => {
            eprintln!("leader add: {}", e);
            std::process::exit(1);
        }
    }

    match DwgWriter::write_to_file(&out, &doc) {
        Ok(()) => println!("wrote {}", out),
        Err(e) => {
            eprintln!("write {}: {}", out, e);
            std::process::exit(1);
        }
    }
}
