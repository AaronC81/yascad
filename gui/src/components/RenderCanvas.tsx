import { PerspectiveCamera, OrbitControls, Grid, Line, Bounds } from "@react-three/drei";
import { Canvas, useLoader } from "@react-three/fiber";
import { STLLoader } from "three/examples/jsm/Addons.js";
import { Suspense, useMemo } from "react";
import * as THREE from "three";

// Make Z up
THREE.Object3D.DEFAULT_UP.set(0, 0, 1);

export default function RenderCanvas({ stl, autoScale }: { stl: string | undefined, autoScale?: boolean }) {
  // Three.js `STLLoader` throws an exception when an STL has no triangles.
  //
  // Guard against this with a completely rubbish heuristic, by checking for the text "facet normal"
  // in the STL text, which appears for every triangle.
  const stlHasTriangles = useMemo(() => stl?.includes("facet normal"), [stl])

  return (
    <Canvas>
      <ambientLight intensity={Math.PI / 3} />

      <PerspectiveCamera makeDefault position={[2, 2, 2]}>
        <directionalLight intensity={0.8} />
      </PerspectiveCamera>
      <OrbitControls makeDefault />

      <Grid
        sectionColor="#999"
        infiniteGrid={true}
        followCamera={true}
        fadeDistance={300}
        fadeStrength={8}
        rotation={[Math.PI / 2, 0, 0]}
        side={2 /* Double-sided */} />
      <Axes />

      {stl && stlHasTriangles &&
        <Suspense>
          <STL stl={stl} autoScale={autoScale} />
        </Suspense>
      }
    </Canvas>
  )
}

function STL({ stl, autoScale }: { stl: string, autoScale?: boolean }) {
  const stlDataUri = useMemo(() => `data:text/plain;base64,${btoa(stl)}`, [stl]);
  const stlAsset = useLoader(STLLoader, stlDataUri);

  return (
    <Bounds fit clip observe={autoScale} margin={2}>
      <mesh geometry={stlAsset}>
        <meshStandardMaterial color="orange" />
      </mesh>
    </Bounds>
  )
}

export function Axes() {
  const size = 1000;
  const lineThickness = 2;
  const xColour = "#f00";
  const yColour = "#0f0";
  const zColour = "#00f";

  return (
    <>
      <Line points={[[-size, 0, 0], [size, 0, 0]]} color={xColour} lineWidth={lineThickness} />
      <Line points={[[0, -size, 0], [0, size, 0]]} color={yColour} lineWidth={lineThickness} />
      <Line points={[[0, 0, -size], [0, 0, size]]} color={zColour} lineWidth={lineThickness} />
    </>
  )
}
