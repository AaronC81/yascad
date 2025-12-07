import { PerspectiveCamera, OrbitControls } from "@react-three/drei";
import { Canvas, useLoader } from "@react-three/fiber";
import { STLLoader } from "three/examples/jsm/Addons.js";
import { Suspense, useEffect, useMemo, useRef } from "react";
import { BufferGeometry } from "three";

export default function RenderCanvas({ stl }: { stl: string | undefined }) {
  // TODO: lighting looks pretty awful, spotlight becomes obvious when zooming in

  return (
    <Canvas>
      <ambientLight intensity={Math.PI / 3} />
      {stl &&
        <Suspense>
          <STL stl={stl} />
        </Suspense>
      }

      {/* TODO: automatically zoom to fit on render */}
      <PerspectiveCamera makeDefault position={[2, 2, 2]}>
        <directionalLight intensity={0.8} />
      </PerspectiveCamera>
      <OrbitControls makeDefault />
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
