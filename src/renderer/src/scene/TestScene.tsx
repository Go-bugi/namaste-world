import { useRef } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import type { Mesh } from 'three';

/** 계속 도는 도형. 렌더링이 멈추지 않았는지 눈으로 확인하는 시험용이다. */
function SpinningKnot(): React.JSX.Element {
  const mesh = useRef<Mesh>(null);

  // 프레임 간격(delta)을 곱해 주사율과 상관없이 같은 속도로 돈다
  useFrame((_, delta) => {
    if (!mesh.current) return;
    mesh.current.rotation.x += delta * 0.6;
    mesh.current.rotation.y += delta * 0.9;
  });

  return (
    <mesh ref={mesh}>
      <torusKnotGeometry args={[0.8, 0.28, 128, 16]} />
      <meshStandardMaterial color="#6988e6" roughness={0.35} metalness={0.2} />
    </mesh>
  );
}

/** 작은 창의 시험용 3D. 배경을 그리지 않아 창 뒤의 화면이 비쳐 보인다. */
function TestScene(): React.JSX.Element {
  return (
    <Canvas
      camera={{ position: [0, 0, 4], fov: 50 }}
      dpr={[1, 2]}
      // alpha를 켜고 배경색을 정하지 않아야 투명한 창에서 검은 배경이 생기지 않는다
      gl={{ alpha: true, antialias: true }}
    >
      <ambientLight intensity={0.6} />
      <directionalLight position={[3, 4, 5]} intensity={2} />
      <SpinningKnot />
    </Canvas>
  );
}

export default TestScene;
