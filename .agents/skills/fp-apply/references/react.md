# React에서의 함수형

SKILL.md의 원칙을 React 컴포넌트와 Hook에 적용하는 문서입니다. zustand store는 [zustand.md](zustand.md), R3F `useFrame`은 [r3f.md](r3f.md)에 있습니다. TS 문법(`Result`, branded type, 판별 유니온, 빌드 타깃 제약)은 [typescript.md](typescript.md)를 따릅니다.

## 경계

| 코드 | 성격 | 여기에 두는 것 |
| --- | --- | --- |
| 컴포넌트 본문(렌더링) | 순수해야 함 | props·state로 JSX 계산 |
| 이벤트 핸들러 | shell | 입력 받기 → core 호출 → 상태 갱신 |
| `useEffect` | shell | 외부 시스템과 동기화(구독, 타이머, IO), 클린업 |
| 커스텀 Hook | shell 어댑터 | IO 경계 모듈과 core를 이어 컴포넌트에 값만 넘김 |
| core 모듈 | core | 계산, reducer, 파생 값 계산 |

- core 모듈은 `react`를 import하지 않습니다. Hook도, JSX도 쓰지 않는 평범한 TS 함수입니다. import하면 안 되는 전체 목록은 typescript.md에 있습니다.
- 최상위 컴포넌트는 화면을 조립만 합니다. 상태와 계산은 core와 store에 둡니다.

## 컴포넌트: 렌더링은 순수 함수

컴포넌트 본문은 "props와 state → JSX" 계산입니다. React는 컴포넌트를 언제든, 몇 번이든 렌더링할 수 있습니다.

- 렌더링 중에 props, state, 바깥 변수를 바꾸지 않습니다.
- 렌더링 중에 IO나 구독을 하지 않고, `Date.now()`나 `Math.random()`도 부르지 않습니다. 이런 값이 필요하면 state나 props로 받거나 `Effect`, 이벤트 핸들러에서 만듭니다.
- `StrictMode`는 개발 모드에서 컴포넌트를 두 번 렌더링하고, `Effect`의 설정과 클린업을 한 번 더 실행합니다. 순수하지 않은 렌더링이나 클린업 없는 `Effect`가 있으면 이때 이상 동작이 드러납니다. 증상이 보이면 `StrictMode`를 끄지 말고 원인을 고칩니다.

### 파생 값은 저장하지 않고 계산

```tsx
// 피할 것: 파생 값을 state에 두고 Effect로 맞춘다 → 한 박자 늦고 어긋날 수 있다
// const [visible, setVisible] = useState<readonly Todo[]>([]);
// useEffect(() => setVisible(todos.filter((t) => !t.done)), [todos]);

// 할 것: 렌더링 중에 계산한다
const visible = visibleTodos(todos, filter); // core의 순수 함수
```

- `useMemo`는 계산이 실제로 무겁다고 측정됐을 때만 씁니다.
- props를 state로 복사하지 않습니다. 복사하는 순간부터 원본과 어긋날 수 있습니다.
- props가 바뀔 때 state를 초기값으로 되돌려야 하면 `Effect`로 맞추지 말고 `key`를 바꿉니다. `key`가 다르면 React가 컴포넌트를 새로 만들므로 state는 "이 `key`의 초기값"이라는 값 하나가 됩니다.

```tsx
// 피할 것: useEffect(() => setDraft(''), [userId]);
// 할 것: 사용자마다 별개의 편집기로 본다
<Editor key={userId} userId={userId} />
```

### 상태 갱신은 "이전 값 → 새 값"

```tsx
setCount((c) => c + 1);
setItems((xs) => addItem(xs, item)); // core 함수로 위임
```

- 이전 값에 의존하는 갱신은 업데이터 함수로 넘깁니다. `StrictMode`에서 두 번 불릴 수 있으므로 업데이터 함수는 순수해야 합니다.
- state 객체나 배열을 직접 고치지 않고 새 값을 만듭니다.
- 상태 전이 규칙이 여러 개면 `useReducer`에 core의 순수 reducer를 넘깁니다.

```ts
// core
export type TodoAction =
  | { readonly kind: 'add'; readonly id: string; readonly title: string }
  | { readonly kind: 'toggle'; readonly id: string }
  | { readonly kind: 'remove'; readonly id: string };

export type Todo = { readonly id: string; readonly title: string; readonly done: boolean };

export const todosReducer = (todos: readonly Todo[], action: TodoAction): readonly Todo[] => {
  switch (action.kind) {
    case 'add':
      return [...todos, { id: action.id, title: action.title, done: false }];
    case 'toggle':
      return todos.map((t) => (t.id === action.id ? { ...t, done: !t.done } : t));
    case 'remove':
      return todos.filter((t) => t.id !== action.id);
    default:
      return assertNever(action); // typescript.md
  }
};
```

- 새 id(랜덤 값)나 현재 시각처럼 불순한 값은 reducer 안에서 만들지 않습니다. action을 만드는 핸들러에서 만들어 action에 담습니다. id 생성 함수도 빌드 타깃 제약을 지킵니다(typescript.md).

### UI 상태도 판별 유니온으로

`isLoading`, `error`, `data`를 따로 두지 않고 유니온 하나로 둡니다(typescript.md의 `Remote`). 화면은 `switch (state.kind)`로 경우마다 그립니다. 로딩 표시와 에러 메시지가 함께 뜨는 것 같은 불가능한 화면이 생기지 않습니다.

## Effect와 커스텀 Hook: 얇은 shell

- `Effect`는 **외부 시스템과 동기화할 때만** 씁니다(구독, 타이머, 창 이벤트, IO). 사용자 행동에 대한 반응은 이벤트 핸들러에서 처리합니다.
- `Effect`에서 획득한 것은 **클린업에서 반드시 해제**합니다(구독 해제, 타이머 정리, 요청 취소). 획득과 해제를 한 곳에 짝으로 둡니다.
- 비동기 `Effect`에서는 늦게 도착한 결과가 최신 상태를 덮어쓰지 않게 합니다.

```tsx
// fetchUsers는 IO 경계 모듈의 함수 (ResultAsync를 반환하므로 reject되지 않는다)
export function useUsers(
  fetchUsers: () => ResultAsync<readonly User[], LoadError>,
): Remote<readonly User[], LoadError> {
  const [state, setState] = useState<Remote<readonly User[], LoadError>>({ kind: 'loading' });

  useEffect(() => {
    let ignore = false;
    void fetchUsers().then((result) => {
      if (!ignore) setState(toRemote(result)); // Result → 화면 상태 (typescript.md)
    });
    return () => {
      ignore = true; // 클린업: 늦게 온 결과를 버린다
    };
  }, [fetchUsers]);

  return state;
}
```

- Hook은 IO 경계 모듈을 부르고, 결과 해석(파싱, 계산)은 core 함수에 맡기고, 컴포넌트에는 값만 돌려줍니다. 컴포넌트에서 플랫폼 API(`invoke` 등)를 직접 부르지 않습니다.
- 위의 `fetchUsers`처럼 의존성을 인수로 받는 Hook은 다른 구현으로 바꿔 끼우기 쉽습니다. 인수로 넘기는 함수는 모듈 수준 함수나 IO 경계 모듈의 함수처럼 참조가 바뀌지 않는 것을 넘깁니다.
- **의존성 배열은 `Effect`의 시그니처입니다.** 안에서 쓰는 값을 빼서 속이지 않습니다. 최신 값을 읽기만 하고 다시 동기화할 이유는 되지 않는 로직(알림 문구에 넣을 테마, 로그에 남길 설정값 등)은 `useEffectEvent`로 분리합니다. 그러면 무엇이 동기화를 일으키는지가 의존성 배열에 그대로 드러납니다.

```tsx
const onConnected = useEffectEvent(() => showToast(formatConnected(roomId, theme))); // theme은 읽기만 한다
useEffect(() => {
  const conn = createConnection(roomId);
  conn.on('connected', () => onConnected()); // Effect Event는 Effect 안에서 호출만 한다
  conn.connect();
  return () => conn.disconnect();
}, [roomId]); // roomId가 바뀔 때만 다시 연결한다
```

- 브라우저의 외부 상태(온라인 여부, 창 크기, `matchMedia`)를 구독해 값으로 읽기만 할 때는 `Effect`와 `setState` 대신 `useSyncExternalStore`를 씁니다. `subscribe`가 해제 함수를 돌려주므로 획득과 해제가 짝을 이루고, `getSnapshot`은 바깥 상태를 읽어 값 하나를 돌려줍니다. 상태가 그대로면 `getSnapshot`은 같은 값(같은 참조)을 돌려줘야 하므로 매번 새 객체를 만들지 않습니다.

## 테스트

- core부터 테스트합니다. reducer와 파생 값을 계산하는 함수는 모두 "인수 → 결과"라서 값만 비교하면 됩니다.
- Hook과 컴포넌트는 렌더링 없이 검증할 수 없습니다. 검증할 규칙이 Hook이나 컴포넌트에 있으면 core 함수로 빼서 테스트합니다. `useUsers`에서는 `toRemote`가 그 함수입니다.

## 체크리스트

1. 렌더링 중에 무언가를 바꾸거나 IO, 현재 시각, 랜덤을 쓰는가?
2. 계산할 수 있는 값을 `useState`에 저장하고 `Effect`로 맞추는가? props가 바뀔 때 `Effect`로 state를 초기화하는가(`key`로 대신할 수 있는가)?
3. 이전 값에 의존하는 갱신을 업데이터 함수로 넘기는가? 업데이터 함수가 순수한가?
4. 여러 boolean으로 UI 상태를 표현하는가?
5. `Effect`에 클린업이 있는가? 늦게 온 비동기 결과를 버리는가? 의존성 배열에서 쓰는 값을 빼지 않았는가?
6. 컴포넌트에서 `invoke`를 직접 부르거나, 계산 규칙을 Hook이나 컴포넌트에 두는가?
