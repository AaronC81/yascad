# YASCAD Language

## OpenSCAD compatibility

YASCAD is largely compatible with OpenSCAD by design, though some behaviours are deliberately different to fit better with YASCAD's additions.

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

### Little conveniences

YASCAD lets you do arithmetic with scalars and vectors, which isn't possible in OpenSCAD:

```
[1, 2, 3] + 1
// equivalent to
[1 + 1, 2 + 1, 3 + 1]
```

## Inspecting geometry

YASCAD lets you assign geometry to a variable.

```
c = cube(5);
```

You can access properties on this geometry and use it to draw other geometry:

```
// Another cube one unit larger
// TODO: simplify when vector arithmetic works
cube([c.size.x + 1, c.size.y + 1]);
```

Available properties are:

- `min_point` (aliased to `origin`) - the lower coordinate of the bounding box of the geometry.
- `max_point` - the higher coordinate of the bounding box of the geometry.
- `size` - the size of the bounding box of the geometry.

All of these return a vector with the same dimensionality as the geometry.
That is, `square(...).size` is a 2D vector, and `cube(...).size` is a 3D vector.

### The `it` keyword

Sometimes you might want to access properties on geometry which is the target of an operator.

You **cannot** do this; geometry can't be edited after being stored in a variable:

```
c = cube(5);

translate(c.size)
c;
```

Instead, you can use the `it` keyword to refer to the target of the current operator:

```
translate(it.size)
cube(5);
```

## Duplicating geometry

The `copy` module will copy the geometry in a variable:

```
c = cube(5);

translate(c.size)
copy(c);
```

Remember that, in most cases, you'll need to remember to apply an operator to `copy`, else the copied geometry will exactly overlap with the existing geometry, so you won't see anything.

## Virtual geometry

When using a module like `cube`, you are typically creating _physical geometry_.
This refers to geometry which is immediately visible in the output model.

YASCAD also allows you to create _virtual geometry_.
This geometry can be inspected and duplicated like any other, except it does **not** appear in the output model.

The `buffer` operator will draw its children as virtual geometry. In this example, no cube appears in the output, but `c` can be inspected as a 5-unit cube:

```
c = buffer() cube(5);
```

The `copy` module can be used to instantiate virtual geometry as physical geometry. This would draw the cube in the output:

```
copy(c);
```

Virtual geometry is most useful when used in this pattern:

1. Draw some virtual geometry
2. Inspect it
3. Use some properties from inspection to draw physical geometry

This provides another way to implement the `it` example from earlier:

```
c = buffer() cube(5);

translate(c.size)
copy(c);
```

This is very powerful when used to define operators.

Here's an example of an operator which rotates geometry around its own center, by temporarily translating it so its center is at `(0, 0)`:

```
operator rotate_around_centre(degrees) {
    c = buffer() children();

    translate([c.min_point.x + c.size.x / 2, c.min_point.y + c.size.y / 2])
    rotate(degrees)
    translate([-c.min_point.x - c.size.x / 2, -c.min_point.y - c.size.y / 2])
    copy(c);
}

rotate_around_centre(45)
translate([2, 2])
square(1);
```

## Outputs

By default, the output from your YASCAD code is the sum of all physical geometry.

There may be cases where you'd like to have code produce multiple separate models.
For example, if you're designing a storage box, you might like to 3D print the main box and the lid separately, but model them together so they can share dimensions.

To accomplish this, apply the `output` operator to each separate model.
Each `output` requires a string parameter providing a unique name:

```
output("Lid")
cube(...);

output("Box")
cube(...);
```
