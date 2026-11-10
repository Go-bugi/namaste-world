# React Three Fiber에서의 함수형

SKILL.md의 원칙을 React Three Fiber의 `useFrame`에 적용하는 문서입니다. 컴포넌트와 Hook은 [react.md](react.md), store는 [zustand.md](zustand.md)를 따릅니다.

## 경계

| 코드 | 성격 | 여기에 두는 것 |
| --- | --- | --- |
| `useFrame` | shell (매 프레임) | ref 변경. 계산은 core 함수 |
| core 모듈 | core | 위치, 각도, 상태 전이 같은 수학 |

- core 모듈은 `three`, `@react-three/*`를 import하지 않습니다. three의 `Vector3` 같은 가변 객체 대신 숫자나 `readonly [number, number, number]`로 계산하고, three 객체에는 shell에서 반영합니다.

## `useFrame`

- `useFrame` 콜백은 매 프레임 실행되는 shell입니다. 위치, 회전, 속도처럼 매 프레임 바뀌는 값은 store에 넣지 않고 ref로 직접 바꿉니다. 이 가변성은 허용합니다.
- 다만 **"어디로, 얼마나"는 순수 함수로 계산**합니다. 프레임 간격 `delta`를 인수로 받는 함수는 주사율과 상관없이 같은 결과를 내고, 렌더링 없이 테스트할 수 있습니다.

```tsx
// core: three를 모르는 수학
// delta를 지수로 반영해야 프레임을 어떻게 나눠도 같은 거리만큼 다가간다
export const approachFactor = (rate: number, delta: number): number => 1 - Math.exp(-rate * delta);

// 각도 차이를 [-π, π]로 접어 짧은 쪽으로 돈다
export const angleDiff = (from: number, to: number): number => Math.atan2(Math.sin(to - from), Math.cos(to - from));

export const approachAngle = (current: number, target: number, rate: number, delta: number): number =>
  current + angleDiff(current, target) * approachFactor(rate, delta);

export const facingYaw = (from: readonly [number, number], to: readonly [number, number]): number =>
  Math.atan2(to[0] - from[0], to[1] - from[1]);

// shell
function Follower(): React.JSX.Element {
  const mesh = useRef<Mesh>(null);
  useFrame((_, delta) => {
    if (!mesh.current) return;
    const { target } = useTargetStore.getState(); // useFrame에서는 selector Hook 대신 getState
    const yaw = facingYaw([mesh.current.position.x, mesh.current.position.z], target);
    mesh.current.rotation.y = approachAngle(mesh.current.rotation.y, yaw, 4, delta);
  });
  return (
    <mesh ref={mesh}>
      <boxGeometry />
      <meshStandardMaterial />
    </mesh>
  );
}
```

- `useFrame` 안에서 `setState`나 store의 `set`을 매 프레임 부르지 않습니다. 그러면 리렌더링도 매 프레임 일어납니다. 상태가 실제로 바뀌는 순간(도착, 충돌, 모드 전환)에만 store에 반영합니다.
- 상태 기계(닫힘 → 열리는 중 → 열림)는 core의 판별 유니온과 전이 함수로 두고, `useFrame`은 전이 결과를 화면에 반영하기만 합니다.
- JSX로 선언한 geometry와 material은 언마운트될 때 R3F가 정리합니다. 코드에서 직접 만든 three 객체(`new BufferGeometry()`, 직접 관리하는 텍스처 등)는 `Effect` 클린업에서 `dispose()`합니다.
- 개발용 조절 패널(GUI 컨트롤) 값은 shell 입력입니다. 컴포넌트에서 읽어 core 함수의 인수로 넘깁니다.
- 재질, 조명, 성능 같은 3D 자체는 이 문서의 범위가 아닙니다.

## 테스트

- 수학 함수와 전이 함수는 `delta`와 위치 값을 넘겨 결과만 비교합니다. 렌더링이나 Canvas가 필요 없습니다.

## 체크리스트

1. `useFrame`에서 매 프레임 `set`을 부르는가? 계산이 ref 변경 코드에 섞여 있는가?
2. `delta`를 인수로 받아 주사율과 상관없는 결과를 내는가?
3. 직접 만든 three 객체를 `dispose`하는가?
