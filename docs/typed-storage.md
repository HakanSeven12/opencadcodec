# Typed record storage

Enable `typed-storage` to capture the available typed content of a `CadDocument`,
store it independently of that live document and inspect it after reopening.
The feature includes `serde` and optional `rmp-serde` 1.3 and `serde_ignored` 0.1.
Default and `serde`-only builds do not enable the transport dependencies.

```rust
use acadrust::{CadDocument, storage::{StorageNamespace, TypedRecordContainer}};
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let source = CadDocument::new();
let captured = TypedRecordContainer::capture_document(
    &source, StorageNamespace::new("drawing-A".into())?,
)?;
let bytes = captured.container.to_bytes()?;
drop(source);
let stored = TypedRecordContainer::from_bytes(&bytes)?;
assert_eq!(stored.metadata().namespace.as_str(), "drawing-A");
assert!(!stored.records().layers().is_empty());
assert_eq!(stored.capture_report(), &captured.report);
# Ok(())
# }
```

The caller supplies a nonempty opaque namespace. Handles identify records within
that source namespace; they are not globally unique identifiers. Storage performs
no filesystem access, external resource lookup or interpretation of `source_path`.
Callers own file transport and limits appropriate to their input size.

`RecordViews` exposes entity/object records, ordered classes, all ten symbol
table entry lists, typed document context and separate decoded EED evidence.
The source is not mutated. Classes and table entries retain order and duplicate
display names; lookup caches are not authoritative storage. Private backing
entities, including BLOCK/ENDBLK markers, are captured instead of only the
filtered public entity iterator. Object and context maps use sorted keyed lists.
An allocation watermark is retained, without allocating or reserving handles.

## What capture means

Capture is broad across available typed records. It does not mean every source
entity has a complete typed representation, nor that the container can already
be written back to CAD. `CaptureReport` survives storage alongside the records.
Each entry has a location, coverage, stable code and explanatory message:

- `CompleteTyped`: typed values at this location are retained. This does not
  qualify reconstruction or CAD writeback.
- `PartialTyped`: available typed values survive, with stated exclusions or
  unresolved interpretation.
- `ShellOnly`: only a typed common/type shell survives an opaque source payload.

The document-level `storage.typed_only` entry records the overall boundary.
`context.side_view_unqualified` retains supplementary typed views without deciding
their agreement with backing records. Exclusion and EED codes are listed in the
[V1 field manifest](typed-storage-v1-fields.md). Free-text messages are descriptive;
consumers should use code/location/coverage rather than parse messages.

Raw DWG records, replay framing, source DXF code bags, opaque graphics, private
original-object baselines and shared AcDs storage are excluded. Entity-owned
ACIS SAT/SAB, OLE/CFB, VBA, proxy content and binary XRecord values are retained
where they are standalone model content. A byte field is not excluded merely
because it is binary or named `raw`.

Source EED uses the existing DWG decoder with actual source version, APPID and
layer context. Authored XDATA wins. Values that cannot safely be installed under
real names remain separately inspectable with source handles and explicit
unresolved-layer values. Undecodable source bytes are excluded and reported;
no decoded prefix or invented name is substituted.

## Storage and compatibility

The V1 experimental format uses `OCCTSNAP`, a big-endian u32 version, and one
named MessagePack envelope containing kind, codec version, namespace, capture
report and a binary body holding one named typed record map. Struct fields and
enum tags are names; positional struct encoding and numeric enum ordinals are
rejected. Both layers reject trailing input, unknown fields and attempts to
inject excluded source/cache fields. Unsupported format versions and kinds fail
before interpreting typed record bodies. See the field manifest for defaults.

Transport is binary and preserves float widths and bit patterns (including NaN,
negative zero and subnormals), byte content and full u64 handles. It makes no
canonical byte-order claim for nested model maps. It is not JSON serialization
of the whole document and does not carry a filesystem or compression policy.

V1 reuses current typed payload Serde shapes plus explicit exception DTOs. An
upstream model change therefore requires reviewing the manifest and compatibility
fixtures. `writer_codec_version` is provenance, not permission to ignore a schema
mismatch. This first version is experimental, not a promise that arbitrary future
crate versions can read all earlier or later model shapes.

## Scope of this contribution

There is no document restore, fresh-handle import, selection/dependency closure,
native merge policy or CAD writeback qualification in this API yet. It is the
shared storage basis for those separate contributions. Read-only capture and
inspection do not require successful future reconstruction. Relation consistency,
nested object identity and checked allocator reservation belong at that next
boundary, rather than being implied by successful byte decoding.

Tests exercise authored records and bounded production-reader recipes for ASCII
DXF, binary DXF and AC1032 DWG through capture, file transport and inspection.
They do not qualify restored CAD export. The current DXF recipe loses PELLIPSE
before storage; direct typed capture and the DWG recipe independently retain
the true value. Storage does not repair upstream reader/writer losses.
