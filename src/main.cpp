#include <iostream>
#include <manifold/manifold.h>
#include <manifold/common.h>
#include <manifold/meshIO.h>

int main() {
    std::cout << "Hello, world!" << std::endl;

    auto outer = manifold::Manifold::Cube({5.0, 5.0, 1.0});
    auto inner = manifold::Manifold::Cube({3.0, 3.0, 1.0});

    auto frame = outer.Boolean(
        inner.Translate({1.0, 1.0, 0.0}),
        manifold::OpType::Subtract
    );

    manifold::ExportMesh("out.stl", frame.GetMeshGL(), {});
    std::cout << "exported mesh." << std::endl;
}
