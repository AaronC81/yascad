# YASCAD's OpenSCAD Compatibility

YASCAD is largely similar to OpenSCAD by design, though some behaviours are deliberately different to fit better with YASCAD's additions.

## Differences

### Module and operator definitions

In OpenSCAD, a module definition can be used as a module or an operator.
Module definitions can use `children` to access children when used as an operator.

In YASCAD, there are separate definition syntaxes for modules and operators:

```
module foo() {
    ...
}

operator bar() {
    ...
}
```

They can only be used in those defined positions.
It is an error to use a module definition as an operator, and vice-versa.

Only operator definitions can use `children`.

### Stricter operators

In OpenSCAD, various operations produce a warning and then continue evaluation with `undef`.
For example, using identifiers which haven't been defined, or passing invalid arguments.

In YASCAD, these are runtime errors instead.

### 3D operations on 2D geometry

OpenSCAD permits performing some 3D transformations on 2D geometry.
If the geometry is extruded, the operation acts on a 2D projection on the X-Y plane.
For example, this operation extrudes a cuboid whose length on the X axis is `sin(45deg)`:

```
linear_extrude(1)
rotate([0, 45, 0])
square(1);
```

YASCAD does not support this.
3D transformations on 2D geometry are an error.

### Little conveniences

YASCAD lets you do arithmetic with scalars and vectors, which isn't possible in OpenSCAD:

```
[1, 2, 3] + 1
// equivalent to
[1 + 1, 2 + 1, 3 + 1]
```

## Standard library support

The OpenSCAD feature set on this table comes from the [cheat sheet](https://openscad.org/cheatsheet/).

### Modules

| Name | Compatibility |
|----------|---------------|
| `circle` | ✅ |
| `square` | ✅ |
| `polygon` | 🟠 Only points, no paths |
| `text` | 🟠 No font, size, or alignment control |
| `sphere` | ✅ |
| `cube` | ✅ |
| `cylinder` | 🟠 No cone forms |
| `polyhedron` | ❌ |
| `linear_extrude` | 🟠 No direction, centering, or twist |
| `rotate_extrude` | ✅ |
| `import` | ❌ Although SVG import is available with the experimental `__svg` module |
| `surface` | ❌ |
| `children` | ✅ Although usage is different, see [differences](#module-and-operator-definitions) |

### Operators

| Name     | Compatibility |
|----------|---------------|
| `union` | ✅ |
| `difference` | ✅ |
| `intersection` | ✅ |
| `transform` | ✅ |
| `rotate` | 🟠 Only simple vector, no arbitrary axes |
| `scale` | ✅ |
| `resize` | ❌ |
| `mirror` | ✅ |
| `multmatrix` | ❌ |
| `color` | ❌ |
| `offset` | ❌ |
| `hull` | ✅ |
| `minkowski` | ❌ |

### Functions

| Name | Compatibility |
|----------|---------------|
| `abs` | ❌ |
| `sign` | ❌ |
| `sin` | ❌ |
| `cos` | ❌ |
| `tan` | ❌ |
| `acos` | ❌ |
| `asin` | ❌ |
| `atan` | ❌ |
| `atan2` | ❌ |
| `floor` | ❌ |
| `round` | ❌ |
| `ceil` | ❌ |
| `ln` | ❌ |
| `len` | ✅ |
| `let` | ❌ |
| `log` | ❌ |
| `pow` | ❌ |
| `sqrt` | ✅ |
| `exp` | ❌ |
| `rands` | ❌ |
| `min` | ✅ |
| `max` | ✅ |
| `norm` | ❌ |
| `cross` | ❌ |
| `is_undef` | ❌ |
| `is_bool` | ❌ |
| `is_num` | ❌ |
| `is_string` | ❌ |
| `is_list` | ❌ |
| `is_function` | ❌ |
| `concat` | ❌ |
| `lookup` | ❌ |
| `str` | ❌ |
| `chr` | ❌ |
| `ord` | ❌ |
| `search` | ❌ |
| `version` | ❌ |
| `version_num` | ❌ |
| `parent_module` | ❌ |
