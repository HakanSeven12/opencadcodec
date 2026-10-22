# V1 typed storage field manifest

V1 is the experimental `typed-records` kind. Root projections and exceptional
payloads are explicit; ordinary payload structs use their existing Serde shapes.
No whole-document Serde, generic property tree, name-based stripping or original
record replay is used. The reviewed implementation resolves rmp-serde 1.3.1,
rmp 0.8.15 and serde_ignored 0.1.14; these are verification versions, not an exact
dependency pin.

## Envelope and records

All five envelope fields are required: `kind`, `writer_codec_version`, `namespace`,
`capture_report` (with `entries`) and binary `body`. Magic is eight bytes
`OCCTSNAP`, followed by u32 BE version 1. The body requires `entities`, `objects`,
`classes`, `tables`, `context` and `extended_data_evidence`.

`tables` requires layers, line_types, text_styles, block_records, dim_styles,
app_ids, views, vports, ucss and vx_table. Each has required `control_handle` and
ordered `entries`; lookup keys are omitted. Classes retain exact order and class
numbers without their lookup map. Source map fields become keyed pair lists;
duplicate keys within each list are rejected, including NULL keys. Duplicate
non-NULL top-level entity/object/table/control identities and object-key/intrinsic
handle mismatches are rejected. Cross-view overlap is permitted; reconstruction
consistency and nested identity qualification remain future work.

The required context fields are version, maintenance_version, header,
allocation_floor, vx_control_entries, block_visibility_params, context_scales,
block_representations, fields, summary_info, source_path, dgn_ls_definitions,
dgn_ls_components, object_xdata, xdictionary_links, reactor_links,
dwg_source_version, preview, section_view_style, view_rep_refs and
section_view_reps. Optional context values must be present, possibly nil.
The allocation floor is max(source private next_handle, header.handle_seed);
it is capture context, not a validated reservation or license to allocate.

## Audit decisions and witnesses

| Audit group | Storage decision / implementation | Regression witness |
|---|---|---|
| A0 completeness | Fresh model census and explicit root/exception/enum projections; future changes require review | manifest and exhaustive construction/matches |
| A1 document fields | Exhaustive 46-field projection, ordered tables/classes, control handles, independent typed context; no lookup caches | private root-projection test, typed_storage_records, typed_storage_envelope |
| A2 retained Serde values | Eleven EntityCommon references/flags, Spline.dwg_scenario and AssociativeObject.source_version retain merged Serde behavior | typed_storage_serde, typed_storage_records |
| A3 remaining skipped fields | Nested common and source-record exclusions, source EED decoding with no raw replay | typed_storage_extended_data, typed_storage_exceptions, private root test |
| A4 non-skip pitfalls | XRecord/framing, PlotSettings and SortEntities exception DTOs; standalone ACIS/VBA/CFB/proxy and authored compatibility scalars retained | typed_storage_boundaries, typed_storage_payloads, typed_storage_exceptions |
| A5 relation authority | Typed context retained independently; actual EED context and authored precedence; relation reconciliation and allocator restore deferred | typed_storage_extended_data, typed_storage_records; no restore qualification |
| A6 honest transport scope | Named binary envelope, strict fields/tags, retained evidence and float bits, no CAD-writeback claim | typed_storage_envelope, typed_storage_literal, private float test |
| A7 census accounting | Thirty-four skips remain after the thirteen merged fixes; numerical handle exceptions stay PR3 reservation work | source audit, typed_storage_serde, typed_storage_boundaries |

Audit labels refer to the reviewed storage field audit; they are not a public
field-classification protocol. Its working documents are local; the table above
records the lasting implementation decisions without requiring those documents.

## Exceptions and excluded state

- EntityCommon: graphic_data, raw_record, has_ds_data excluded by existing skips.
  Their absence decodes as None/None/false. Typed references and flags remain.
- UnknownEntity: common/type identity retained; DWG bytes, DXF codes, source
  version and dwg_handle_bits omitted. UnknownObject retains type/handle/owner;
  original bytes/codes/version/handle-bit framing omitted. Both are shells.
- XRecord: entries, entries_complete, authored metadata and references retained;
  raw_data and DWG framing omitted. Incomplete entries never become complete.
- RegisteredClassObject: typed properties, proxy payload and references retained;
  raw DWG storage/version/handle-bit framing omitted.
- LayoutPrintConfigData and CsacDocumentOptions retain typed class version and
  known flags; source DWG/framing omitted. FormatData is an empty typed shell;
  source DWG/DXF/framing omitted.
- PlotSettings omits only cached_scale; authored scale values remain and cache
  is None after decode. SortEntitiesTable omits entry_map and source DXF codes/version;
  ordered entries have entity_handle: Handle and sort_value: u64. A sort value
  is draw-order data, not a future allocator reservation. Lookup is rebuilt from
  retained entries only; no duplicate entry is silently dropped.
- Layout raw_plot_settings_codes and TableStyle raw_dxf_codes remain excluded.
- ExtendedData.raw_dwg_eed is decoded with existing source-context logic or
  reported as excluded; it is never a wire field. This includes nested entity,
  embedded attribute/marker MTEXT, surface/history construction geometry,
  Field-owned, object-side and private document EED stores. Geometry-body common
  values remain nested content; this does not assign them database identities.
- Private document raw_records, original_objects, raw_record_owners,
  raw_acds_data, raw_acds_fingerprint, dwg_data_store_handles and R12 raw header
  are excluded. Notifications, entity indexes, block writer-expansion caches
  and ACIS writer indexes are derived, not records.

Typed content such as SectionSymbol compatibility scalars, CellValue raw type/unit
codes, AssocVariable cached_value, header fingerprint GUID, source version,
entity-owned SAT/SAB/extra ACIS, standalone embedded geometry bodies,
preview/OLE bytes, VBA original/compressed/source
and CFB ANSI encoding evidence is retained independently. No SAT-token rebuild,
binary regeneration or floating-point normalization is applied.

## Defaults, reports and strictness

Root and exception DTO fields are required, including nullable exception fields.
Ordinary payload defaults remain the model's documented defaults; the thirteen
merged Serde fields default to None/zero when missing. No missing document context
is fabricated. Rejected unknown keys include fields known by ordinary Serde but
excluded here (such as cached_scale, raw_data or dwg_handle_bits). Duplicate known
struct fields and unsupported enum variants fail.

Capture codes:

- `storage.typed_only`, `storage.source_storage_excluded`
- `storage.graphic_data_excluded`, `storage.raw_record_excluded`, `storage.data_store_flag_excluded`
- `context.side_view_unqualified`
- `entity.unknown_shell`, `entity.format_shell`, `entity.layout_print_config_partial`, `object.unknown_shell`
- `xrecord.incomplete`, `xrecord.raw_data_excluded`, `sort_entities.raw_codes_excluded`
- `layout.raw_plot_settings_excluded`, `table_style.raw_codes_excluded`, `class.raw_record_excluded`
- `eed.source_version_missing`, `eed.undecodable`, `eed.decoded`, `eed.already_typed`, `eed.decoded_source_values`,
  `eed.authored_values_retained`, `eed.unresolved_application`, `eed.unresolved_layer`

Locations use zero-based record/child positions or decimal handles within named
context maps, and slash-separated model field paths. EED evidence includes
location, source handle, application handle, actual optional name and ordered
values; unresolved layers are explicit Handle values. Reports describe capture
provenance, not trusted proof of safe reconstruction or CAD writeback. Version 1
has no restoration action report because restoration is not yet implemented.
