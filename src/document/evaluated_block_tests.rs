use super::*;
use crate::objects::{Dictionary, ObjectType};

#[test]
fn copying_evaluated_metadata_keeps_the_source_independent() {
    let mut doc = CadDocument::new();
    let mut handles = Vec::new();
    for name in ["*U1", "*U2", "Ordinary"] {
        let mut record = BlockRecord::new(name);
        record.handle = doc.allocate_handle();
        handles.push(record.handle);
        doc.block_records.add(record).unwrap();
    }
    let (source, target) = (handles[0], handles[1]);
    let data = vec![(0x50, vec![70, 1, 0])];
    doc.eed_by_handle.insert(source, data.clone());
    let original_dictionary = doc.ensure_extension_dictionary(source);
    doc.copy_evaluated_block_metadata(source, target).unwrap();
    assert_eq!(doc.eed_by_handle[&target], data);
    let copied_dictionary = doc.extension_dictionary_handle(target).unwrap();
    assert_ne!(original_dictionary, copied_dictionary);
    let ObjectType::Dictionary(copied) = &doc.objects[&copied_dictionary] else {
        panic!()
    };
    assert_eq!(copied.owner, target);
    assert!(copied.entries.is_empty());
    assert!(doc
        .copy_evaluated_block_metadata(source, handles[2])
        .is_err());

    let child = doc.allocate_handle();
    let mut dictionary = Dictionary::new();
    dictionary.handle = child;
    dictionary.owner = original_dictionary;
    doc.objects
        .insert(child, ObjectType::Dictionary(dictionary));
    let ObjectType::Dictionary(original) = doc.objects.get_mut(&original_dictionary).unwrap()
    else {
        panic!()
    };
    original.entries.push(("Nested".into(), child));
    let before = doc.objects.clone();
    assert!(doc.copy_evaluated_block_metadata(source, target).is_err());
    assert_eq!(
        before, doc.objects,
        "reject additional relationships without partial writes"
    );
}
