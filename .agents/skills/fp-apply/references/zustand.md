# zustand에서의 함수형

SKILL.md의 원칙을 zustand store에 적용하는 문서입니다. 컴포넌트와 Hook은 [react.md](react.md), TS 문법은 [typescript.md](typescript.md)를 따릅니다.

## 경계

| 코드 | 성격 | 여기에 두는 것 |
| --- | --- | --- |
| zustand store | 공유 상태 (shell) | 원본 값, core에 위임하는 action |
| core 모듈 | core | 상태 전이 함수, selector로 쓰는 계산 함수 |

- core 모듈은 `zustand`를 import하지 않습니다.

## 규칙

- **store에는 원본 값만** 둡니다. 계산할 수 있는 값은 selector로 계산합니다.
- **action은 core 함수에 위임합니다.** `set`에 넘기는 업데이터 함수는 "현재 상태 → 바뀐 부분"을 돌려주는 순수 함수입니다.
- **IO는 업데이터 함수 밖에서** 합니다. 비동기 action은 IO를 끝낸 뒤 그 결과로 `set`합니다. 결과를 가공해야 하면 core 함수로 계산합니다.

```ts
import { produce } from 'immer';
import type { ResultAsync } from 'neverthrow';
import { create } from 'zustand';

// 한 블록에 줄였지만 core와 store는 다른 파일이다. core 파일은 immer만 import한다

// core (zustand를 모른다)
export type CartLine = { readonly sku: string; readonly qty: number };
export type CartState = { readonly lines: readonly CartLine[] };

export const addItem = (s: CartState, sku: string): CartState =>
  produce(s, (draft) => {
    const line = draft.lines.find((l) => l.sku === sku);
    if (line) line.qty += 1;
    else draft.lines.push({ sku, qty: 1 });
  });
export const removeItem = (s: CartState, sku: string): CartState => ({ lines: s.lines.filter((l) => l.sku !== sku) });
export const itemCount = (s: CartState): number => s.lines.reduce((n, l) => n + l.qty, 0);

// store (shell)
type CartStore = CartState & {
  readonly add: (sku: string) => void;
  readonly remove: (sku: string) => void;
  readonly load: (
    fetchLines: () => ResultAsync<readonly CartLine[], LoadError>,
  ) => ResultAsync<readonly CartLine[], LoadError>;
};

export const useCartStore = create<CartStore>()((set) => ({
  lines: [],
  add: (sku) => set((s) => addItem(s, sku)),
  remove: (sku) => set((s) => removeItem(s, sku)),
  // IO는 업데이터 함수 밖에서. 성공 값을 반영하는 사이드 이펙트는 map이 아니라 andTee로 드러낸다
  load: (fetchLines) => fetchLines().andTee((lines) => set({ lines })),
}));

// 컴포넌트: selector도 core 함수를 그대로 쓴다
// const count = useCartStore(itemCount);
// 핸들러: 결과는 마지막에 match로 처리한다
// void useCartStore.getState().load(io.fetchCart).match(() => undefined, (e) => showToast(errorMessage(e)));
```

- `map`, `mapErr`에는 값을 바꾸는 순수 함수만 넘깁니다. 사이드 이펙트는 `andTee`(성공), `orTee`(실패)로 넘겨 계산과 구분하고, 결과를 쓰는 곳에서는 `match`로 두 경우를 모두 처리합니다.
- action 안에 조건 분기가 쌓이기 시작하면 core 함수로 옮길 때입니다.
- **immer는 core 함수 안에서 `produce`로 씁니다.** zustand의 immer 미들웨어(`zustand/middleware/immer`)는 쓰지 않습니다. 미들웨어를 쓰면 action 안에서 상태를 직접 고치게 되고 계산 규칙이 다시 store로 들어옵니다.
- selector가 매번 새 객체나 배열을 만들면 zustand v5에서는 리렌더링이 끝없이 반복되어 에러가 납니다. 원시 값을 고르거나 `zustand/react/shallow`의 `useShallow`로 감쌉니다.

## 테스트

- 상태 전이 함수와 selector 함수는 "인수 → 결과"라서 store 없이 값만 비교합니다.
- 비동기 action은 고정 값을 돌려주는 함수를 넘겨 확인합니다.

## 체크리스트

1. 계산할 수 있는 값을 store에 저장하는가?
2. store action 안에 조건 분기가 쌓였는가? `set`의 업데이터 함수 안에서 IO를 하는가?
3. zustand immer 미들웨어로 action 안에서 상태를 직접 고치는가?
4. selector가 매번 새 객체나 배열을 만드는가?
