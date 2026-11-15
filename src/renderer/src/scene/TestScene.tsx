import { useRef } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { OrbitControls, StatsGl } from '@react-three/drei';
import { useControls } from 'leva';
import type { Mesh } from 'three';

function SpinningBox(): React.JSX.Element {
  const mesh = useRef<Mesh>(null);
  // 개발 중에는 leva 패널로 조절하고, 빌드에서는 여기 적힌 기본값이 그대로 쓰인다
  const { speed, color } = useControls('큐브', {
    speed: { value: 1, min: 0, max: 5, step: 0.1 },
    color: '#6988e6',
  });

  // 프레임 간격(delta)을 곱해 주사율과 상관없이 같은 속도로 돈다
  useFrame((_, delta) => {
    if (!mesh.current) return;
    mesh.current.rotation.x += delta * 0.6 * speed;
    mesh.current.rotation.y += delta * 0.9 * speed;
  });

  return (
    <mesh ref={mesh}>
      <boxGeometry args={[1.6, 1.6, 1.6]} />
      <meshStandardMaterial color={color} />
    </mesh>
  );
}

function DemoScene(): React.JSX.Element {
  return (
    <div className="demo-scene">
      <Canvas camera={{ position: [0, 0, 4], fov: 50 }} dpr={[1, 2]}>
        <ambientLight intensity={0.6} />
        <directionalLight position={[3, 4, 5]} intensity={2} />
        <SpinningBox />
        <OrbitControls enableZoom={false} enablePan={false} />
        {/* FPS, CPU, GPU 시간 패널. 개발 중에만 띄운다 */}
        {import.meta.env.DEV && <StatsGl />}
      </Canvas>
    </div>
  );
}

export default DemoScene;
