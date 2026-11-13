//! Tests for nested data (2.8): JSON paths through lists, XML, HL7 sources,
//! explode and flatten steps, and nested exports, using the sample files.

use super::export::{Export, ExportKind, Output, hl7_template};
use super::import::{Input, Source, choices};
use super::{Kind, Table, TableStep};

const ORDERS: &str = include_str!("../../samples/tables/orders.json");
const APPTS: &str = include_str!("../../samples/tables/appointments.xml");
const LAB: &str = include_str!("../../samples/tables/lab-results.hl7");

fn col<'a>(t: &'a Table, name: &str) -> Vec<&'a str> {
    let c = t
        .find(name)
        .unwrap_or_else(|| panic!("no column {name}: {:?}", t.names()));
    t.columns[c].cells.iter().collect()
}

fn text(out: Output) -> String {
    match out {
        Output::Text(t) => t,
        Output::Bytes(_) => panic!("bytes"),
    }
}

#[test]
fn json_path_through_a_list_copies_parent_fields() {
    let src = Source::Json {
        rows: "orders.lines".into(),
        parents: true,
        detect_types: true,
    };
    let t = src.read(Input::Text(ORDERS)).unwrap();
    assert_eq!(t.rows, 6);
    assert_eq!(
        col(&t, "orders.id")[..4],
        ["SO-10421", "SO-10421", "SO-10421", "SO-10422"]
    );
    assert_eq!(col(&t, "orders.clinic.address.city")[3], "Millbrook");
    assert_eq!(col(&t, "sku")[5], "THM-DIG-01");
    assert_eq!(t.columns[t.find("qty").unwrap()].kind, Kind::Integer);
    // Lists on the way (tags, lines) aren't copied.
    assert!(t.find("orders.tags").is_none());
    let bare = Source::Json {
        rows: "orders.lines".into(),
        parents: false,
        detect_types: true,
    };
    assert_eq!(bare.read(Input::Text(ORDERS)).unwrap().columns.len(), 4);
}

#[test]
fn guesses_and_offers_paths() {
    assert!(matches!(Source::guess(ORDERS, None), Source::Json { .. }));
    assert!(matches!(Source::guess(APPTS, None), Source::Xml { .. }));
    assert!(matches!(Source::guess(LAB, None), Source::Hl7 { .. }));
    assert_eq!(choices(ORDERS).json, ["orders", "orders.lines"]);
    assert!(choices(APPTS).xml.contains(&"appointment".to_owned()));
    assert_eq!(choices(LAB).segments[..3], ["MSH", "PID", "PV1"]);
}

#[test]
fn xml_elements_attributes_and_text() {
    let src = Source::guess(APPTS, None);
    let t = src.read(Input::Text(APPTS)).unwrap();
    assert_eq!(t.rows, 3);
    assert_eq!(col(&t, "id"), ["A-3301", "A-3302", "A-3303"]);
    assert_eq!(col(&t, "patient.name.family")[1], "Okafor-Lindqvist");
    assert_eq!(col(&t, "patient.mrn")[2], "800124002");
    // An element with text and attributes: its text is its own column.
    assert_eq!(col(&t, "provider")[0], "Dana Halvorsen, MD");
    assert_eq!(col(&t, "provider.npi")[0], "1234567893");
    assert_eq!(col(&t, "reason")[2], "Sore throat & cough");
    // Repeated elements are a JSON list, to explode.
    assert!(col(&t, "reason")[1].starts_with('['));
    // A path with or without the root element.
    let at = |rows: &str| {
        Source::Xml {
            rows: rows.into(),
            parents: true,
            detect_types: true,
        }
        .read(Input::Text(APPTS))
        .unwrap()
    };
    assert_eq!(at("schedule.appointment").rows, 3);
    let reasons = at("appointment.reason");
    assert_eq!(reasons.rows, 4);
    assert_eq!(col(&reasons, "appointment.id")[2], "A-3302");
    assert_eq!(col(&reasons, "code")[2], "I10");
    assert_eq!(col(&reasons, "reason")[3], "Sore throat & cough");
}

#[test]
fn hl7_rows_per_segment() {
    let src = Source::Hl7 {
        segment: "OBX".into(),
        components: true,
        keep: vec!["MSH.10".into(), "PID.5.1".into()],
        detect_types: true,
    };
    let t = src.read(Input::Text(LAB)).unwrap();
    assert_eq!(t.rows, 9);
    assert_eq!(t.names()[..2], ["MSH.10", "PID.5.1"]);
    assert_eq!(col(&t, "PID.5.1")[5], "Okafor-Lindqvist");
    assert_eq!(t.columns[t.find("OBX.5").unwrap()].kind, Kind::Text);
}

#[test]
fn explode_and_flatten() {
    let t = Source::guess(APPTS, None).read(Input::Text(APPTS)).unwrap();
    let t = TableStep::ExplodeColumn {
        column: "reason".into(),
        separator: String::new(),
    }
    .apply(&t)
    .unwrap();
    assert_eq!(t.rows, 4);
    assert_eq!(col(&t, "id"), ["A-3301", "A-3302", "A-3302", "A-3303"]);
    let t = TableStep::FlattenColumn {
        column: "reason".into(),
    }
    .apply(&t)
    .unwrap();
    // Plain values stay under the column's own name; objects add columns.
    assert_eq!(col(&t, "reason")[1], "Diabetes follow-up");
    assert_eq!(col(&t, "reason")[0], "Annual wellness visit");
    assert_eq!(col(&t, "reason.code")[2], "I10");
    let split = TableStep::ExplodeColumn {
        column: "reason".into(),
        separator: " ".into(),
    }
    .apply(&t)
    .unwrap();
    assert!(split.rows > t.rows);
}

#[test]
fn nested_json_yaml_and_xml() {
    let src = Source::Json {
        rows: "orders".into(),
        parents: true,
        detect_types: true,
    };
    let t = src.read(Input::Text(ORDERS)).unwrap();
    let mut json = ExportKind::Json.default_export();
    if let Export::Json { nested, .. } = &mut json {
        *nested = true;
    }
    let v: serde_json::Value = serde_json::from_str(&text(json.write(&t).unwrap())).unwrap();
    assert_eq!(v[0]["clinic"]["address"]["city"], "Fernwood");
    let yaml = text(ExportKind::Yaml.default_export().write(&t).unwrap());
    assert!(yaml.contains("  address:\n"), "{yaml}");
    let appts = Source::guess(APPTS, None).read(Input::Text(APPTS)).unwrap();
    let appts = TableStep::RenameColumn {
        column: "id".into(),
        to: "@id".into(),
    }
    .apply(&appts)
    .unwrap();
    let xml = text(
        Export::Xml {
            root: "visits".into(),
            row: "visit".into(),
            skip_empty: true,
        }
        .write(&appts)
        .unwrap(),
    );
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let first = doc.root_element().first_element_child().unwrap();
    assert_eq!(first.attribute("id"), Some("A-3301"));
    let family = first
        .descendants()
        .find(|n| n.has_tag_name("family"))
        .and_then(|n| n.text());
    assert_eq!(family, Some("Quillfeather"));
    assert!(xml.contains("Sore throat &amp; cough"));
}

#[test]
fn hl7_round_trip_through_a_template() {
    let src = Source::Hl7 {
        segment: String::new(),
        components: true,
        keep: Vec::new(),
        detect_types: false,
    };
    let t = src.read(Input::Text(LAB)).unwrap();
    let template = hl7_template(&t);
    assert!(
        template.starts_with("MSH|^~\\&|{MSH.3}|{MSH.4.1}^{MSH.4.2}"),
        "{template}"
    );
    assert!(template.contains("\nPID|{PID.1}||{PID.3.1}^^^{PID.3.4}^{PID.3.5}~{PID.3[2].1}"));
    let out = text(
        Export::Hl7 {
            template: String::new(),
        }
        .write(&t)
        .unwrap(),
    );
    // Read back, the same table.
    let again = src.read(Input::Text(&out)).unwrap();
    assert_eq!(again.names(), t.names());
    for c in 0..t.columns.len() {
        assert_eq!(
            again.columns[c].cells.iter().collect::<Vec<_>>(),
            t.columns[c].cells.iter().collect::<Vec<_>>(),
            "{}",
            t.columns[c].name
        );
    }
    // A table that isn't HL7 gets a template to start from.
    let csv = Source::default()
        .read(Input::Text("id,name\n7,Pat\n"))
        .unwrap();
    let start = hl7_template(&csv);
    assert!(start.contains("PID|1||{id}||{name}"));
    let msg = text(Export::Hl7 { template: start }.write(&csv).unwrap());
    assert!(msg.contains("MSG1|P"));
}
