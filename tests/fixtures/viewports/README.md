# Viewport clipping fixture

`reference-clip-activation.dxf` is a small, hand-authored ASCII DXF regression
fixture. It specifies VIEWPORT status group 90 as 98304 (0x8000 | 0x10000) and
boundary group 340 as 1002, referring forward to a CIRCLE. It was not generated
by the writer under test and is not an AutoCAD-produced interoperability sample.

The fixture isolates decoding of the activation flag and boundary reference.
The companion test constructs full documents and checks AC1032 DXF and DWG
writer/reader roundtrips for each independent activation/boundary-presence state.
