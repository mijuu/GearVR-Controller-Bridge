import { useFrame, useLoader } from "@react-three/fiber";
import { OrbitControls, useTexture, Html } from "@react-three/drei";
import { Euler, Quaternion, Mesh, MeshStandardMaterial } from "three";
import { OBJLoader } from "three/addons/loaders/OBJLoader.js";
import { Suspense, useMemo, useRef } from "react";
import { ControllerState } from "../ControllerStatus/ControllerStatus";

interface Controller3DViewProps {
  state: ControllerState | null;
}

function Model({ state }: { state: ControllerState | null }) {
  const groupRef = useRef<Mesh>(null);
  
  const obj = useLoader(OBJLoader, '/model/gear_vr_controller.obj');
  const texture = useTexture('/model/gear_vr_controller_color_256.png');
  
  // 创建材质并应用纹理
  const material = useMemo(() => 
    new MeshStandardMaterial({ 
      map: texture,
      roughness: 0.8,
      metalness: 0.2
    }), [texture]);

  // 处理模型和材质
  const model = useMemo(() => {
    const cloned = obj.clone();
    cloned.traverse((child) => {
      if (child instanceof Mesh) {
        child.material = material;
        child.castShadow = true;
        child.receiveShadow = true;
      }
    });
    return cloned;
  }, [obj, material]);


  const quaternion = useMemo(() => new Quaternion(), []);
  const euler = useMemo(() => new Euler(), []);
  // 更新模型旋转
  useFrame(() => {
    if (state && groupRef.current) {
      const { x, y, z, w } = state.orientation;
      quaternion.set(x, y, z, w);
      euler.setFromQuaternion(quaternion);
      groupRef.current.rotation.set(euler.x, euler.y, euler.z);
    }
  });

  return (
    <primitive 
      ref={groupRef} 
      object={model} 
      position={[0, 0, 0]}
      scale={[50, 50, 50]} 
    />
  );
}

export default function Controller3DView({ state }: Controller3DViewProps) {
  return (
    <>
      <ambientLight intensity={0.5} />
      <pointLight position={[10, 10, 10]} intensity={0.8} castShadow />
      <OrbitControls 
        enableZoom={true} 
        enablePan={true} 
        enableRotate={true}
        minDistance={1}
        maxDistance={5}
      />
      <Suspense fallback={<Html center>加载模型中...</Html>}>
        <Model state={state} />
      </Suspense>
    </>
  );
}