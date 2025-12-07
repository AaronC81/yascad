import { PerspectiveCamera, OrbitControls } from "@react-three/drei";
import { Canvas, useLoader } from "@react-three/fiber";
import { STLLoader } from "three/examples/jsm/Addons.js";
import { Suspense, useMemo } from "react";

export default function RenderCanvas({ stl }: { stl: string | undefined }) {
  return (
    <Canvas>
      <ambientLight intensity={Math.PI / 3} />
      {stl &&
        <Suspense>
          <STL stl={stl} />
        </Suspense>
      }

      <PerspectiveCamera makeDefault position={[2, 2, 2]}>
        <directionalLight intensity={0.8} />
      </PerspectiveCamera>
      <OrbitControls makeDefault />

      {/* TODO: Temporary - need to draw real axes with measurements at some point */}
      <axesHelper args={[100]} />
    </Canvas>
  )
}

function STL({ stl }: { stl: string }) {
  const stlDataUri = useMemo(() => `data:text/plain;base64,${btoa(stl)}`, [stl]);
  const stlAsset = useLoader(STLLoader, stlDataUri);

  return (
    <mesh geometry={stlAsset}>
      <meshStandardMaterial color="gray" />
    </mesh>
  )
}
