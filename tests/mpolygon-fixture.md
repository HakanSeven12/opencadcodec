# MPOLYGON fixture

`mpolygon-fixture.dwg` is a synthetic AutoCAD 2013 (AC1027) drawing with six
MPOLYGON entities (handles 8B to 90); it contains no customer content. It was
generated as DXF with ezdxf by the script below, then opened in AutoCAD and
saved as "AutoCAD 2013 DWG", so every byte of the DWG was written by AutoCAD.

| # | shape | expected values |
|---|---|---|
| 1 | square (0,0)-(10,10) | solid, entity color index 1, fill color index 5 |
| 2 | square (20,0)-(30,10) with hole (22,2)-(26,6) | two loops |
| 3 | (40,10), (50,10), (50,0) bulge -1, (40,0) | one bulged vertex |
| 4 | square (60,0)-(70,10) | pattern ANSI31, not solid, entity 1, fill 5 |
| 5 | square (80,0)-(90,10) | layer MP_DUCALQUE, entity ByLayer, linetype DASHED, gradient enabled with two stops RGB(200,100,50), fill color ByLayer |
| 6 | bow-tie, stored by AutoCAD as (100,0), (100,10), (110,0), (110,10) | self-intersecting, kept by AutoCAD as ONE valid loop, no invalid loop |

AutoCAD stores the squares clockwise from their minimum corner, e.g.
(0,0), (0,10), (10,10), (10,0), without repeating the first vertex. The offset
vector stored after the fill color is (0,0) for all six objects. AutoCAD's
LIST command reports the same values.

## Where the layout comes from

Each difference from HATCH maps to a public DXF description of MPOLYGON
(ezdxf, `entities/mpolygon.py` and its `dxfentities/mpolygon` documentation):

- No associative bit: MPOLYGON "does not support associated source path
  entities" (ezdxf docs).
- No hatch-style short before the pattern type: "the hatch style tag is not
  supported" (ezdxf docs); ezdxf never exports group 75.
- Loops are polylines with bulges only: the docs describe polyline paths
  including bulges, and ezdxf does not export edge paths for MPOLYGON.
- Leading short = object version: DXF group 70 `version` (default 1).
- Trailing 2RD = offset vector, loop vertices relative to it: DXF group 11
  `offset_vector`; ezdxf's renderer (`addons/drawing/frontend.py`,
  `draw_mpolygon_entity`) translates the loops by it.
- Trailing long + loops = degenerate ("invalid") loops: DXF group 99
  `degenerated_loops`, "a border that is ignored by the hatch".
- Per-loop leading bit: read and discarded, meaning unconfirmed. ezdxf's
  `annotated_boundary` (group 73) is an entity-level flag; no link to this
  bit is confirmed.
- True-colour fill stored as a monochrome gradient: ezdxf
  `set_solid_fill(rgb=...)` calls `set_solid_rgb_gradient`, two stops of the
  same colour (as in shape 5).

Each field was then confirmed by exact bit consumption: 6/6 objects of this
AutoCAD-written fixture, and 6 461 MPOLYGON objects in two real-world files
(2 977 + 3 484, AC1027, not redistributable). The previous HATCH-based
decoding matched the data end on none of the 2 977 objects of the first
file. The binary encoding of degenerate loops is not exercised by any file:
none of these objects has one.

## Regenerating

To regenerate, run the script with ezdxf, open `mpolygon_reference.dxf` in
AutoCAD and save it as AutoCAD 2013 DWG. Handles and vertex order may differ
on another AutoCAD version; the tests find shapes by geometry, not by handle.

```python
# Generates the MPOLYGON reference DXF (ground truth by construction).
import ezdxf

doc = ezdxf.new("R2013", setup=True)  # setup: standard linetypes (DASHED)
doc.layers.add("MP_DUCALQUE", color=3)
msp = doc.modelspace()


def carre(x, y, c):
    return [(x, y), (x + c, y), (x + c, y + c), (x, y + c)]


# 1. solid square, red entity (1), blue fill (5)
mp = msp.add_mpolygon(color=1, fill_color=5)
mp.paths.add_polyline_path(carre(0, 0, 10), is_closed=True)

# 2. square with a hole
mp = msp.add_mpolygon(color=1, fill_color=5)
mp.paths.add_polyline_path(carre(20, 0, 10), is_closed=True)
mp.paths.add_polyline_path(carre(22, 2, 4), is_closed=True)

# 3. arc: (40,10) -> (50,10) -> (50,0) -> half circle through (45,-5) -> (40,0).
#    bulge -1 on (50,0)->(40,0): checked by ezdxf flattening (y min = -5).
mp = msp.add_mpolygon(color=1, fill_color=5)
mp.paths.add_polyline_path(
    [(40, 10, 0), (50, 10, 0), (50, 0, -1.0), (40, 0, 0)], is_closed=True
)

# 4. ANSI31 pattern, red entity, blue fill
mp = msp.add_mpolygon(color=1, fill_color=5)
mp.paths.add_polyline_path(carre(60, 0, 10), is_closed=True)
mp.set_pattern_fill("ANSI31", color=5, scale=1.0)
mp.dxf.color = 1

# 5. ByLayer entity on MP_DUCALQUE (green 3), true-color fill 200,100,50, linetype DASHED
mp = msp.add_mpolygon(dxfattribs={"layer": "MP_DUCALQUE", "linetype": "DASHED"})
mp.paths.add_polyline_path(carre(80, 0, 10), is_closed=True)
mp.set_solid_fill(color=256, rgb=(200, 100, 50))

# 6. self-intersecting bow-tie
mp = msp.add_mpolygon(color=1, fill_color=5)
mp.paths.add_polyline_path([(100, 0), (110, 10), (110, 0), (100, 10)], is_closed=True)

doc.saveas("mpolygon_reference.dxf")
print("ok", len(msp.query("MPOLYGON")))
```
