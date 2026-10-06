# VIEWPORT angle-unit reference

`reference-angle-units.dxf` is a hand-authored minimal AC1032 ASCII DXF.
It has not been exported or rendered by AutoCAD and is not an application
interoperability sample. Neither opencadcodec writer generated its angle values.

VIEWPORT group 50 specifies a snap angle of -45 degrees; group 51 specifies a
view twist of 30 degrees. The internal values must therefore be -pi/4 and pi/6
radians, respectively. The writer test checks literal DXF degrees independently
of the reader, including negative, positive and zero angles.

Autodesk documents groups 50-58 as degrees in DXF:
https://help.autodesk.com/cloudhelp/2018/ENU/AutoCAD-DXF/files/GUID-3F0380A5-1C15-464D-BC66-2C5F094BCFB9.htm

The VIEWPORT reference identifies groups 50 and 51:
https://help.autodesk.com/cloudhelp/2020/ENU/AutoCAD-DXF/files/GUID-2602B0FB-02E4-4B9A-B03C-B1D904753D34.htm
