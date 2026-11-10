# TypeScript에서의 함수형

SKILL.md의 원칙을 TypeScript 문법과 빌드 환경에 맞게 옮긴 문서입니다. React, zustand, R3F 특유의 내용은 [react.md](react.md), [zustand.md](zustand.md), [r3f.md](r3f.md)에 있습니다.

## 경계

| 층 | 위치 | 성격 |
| --- | --- | --- |
| 도메인 타입, 순수 함수 | 순수 TS 파일 | core |
| IO 경계 | 플랫폼 API(Tauri `invoke`·`listen`, `fetch`, `localStorage`)를 감싼 모듈 | shell |
| 상태, 화면 | store, 컴포넌트, `useFrame` | shell ([react.md](react.md), [zustand.md](zustand.md), [r3f.md](r3f.md)) |

- **core 파일은 `react`, `zustand`, `three`, `@react-three/*`, `@tauri-apps/*`와 IO 경계 모듈을 import하지 않습니다.** 아래의 neverthrow와 immer는 순수 라이브러리라 core에서 써도 됩니다.
- import 방향은 컴포넌트, Hook, store → core 하나뿐입니다. core가 shell을 import하지 않습니다.
- 플랫폼 API는 IO 경계 모듈에서만 부릅니다. 이 모듈에는 호출과 파싱만 두고 계산 규칙은 넣지 않습니다. 바깥에서 온 값은 `unknown`으로 받아 도메인 타입으로 파싱합니다.

## 사용하는 라이브러리

| 라이브러리 | 맡는 일 | 쓰는 곳 |
| --- | --- | --- |
| [neverthrow](https://github.com/supermacro/neverthrow) | 실패를 값으로 다루는 `Result`, `ResultAsync` | 실패할 수 있는 모든 함수. 특히 IO 경계와 파싱 |
| [immer](https://immerjs.github.io/immer/) | 바꾸는 것처럼 써도 새 값을 만드는 `produce` | 중첩이 깊은 객체를 갱신하는 core 함수 |

## 빌드 타깃 제약

불변 배열 메서드(`toSorted`, `with` 등)는 함수형 코드에 잘 맞지만 최근에 생긴 API입니다. 번들러가 문법만 낮추고 메서드는 polyfill하지 않으면 타입 검사를 통과해도 오래된 런타임에서 오류가 납니다. 빌드 타깃이 지원하지 않는 API는 아래 대안으로 씁니다.

| 타깃이 지원하지 않을 때 | 대신 |
| --- | --- |
| `toSorted`, `toReversed` | `[...xs].sort(cmp)`, `[...xs].reverse()` |
| `toSpliced`, `with` | `xs.slice(0, i).concat([x], xs.slice(i))`, `xs.map((v, j) => (j === i ? x : v))` |
| `findLast`, `findLastIndex` | `reduce`, 뒤집은 복사본에서 `find` |
| `Array.prototype.at` | `xs[xs.length - 1]` |
| `Object.hasOwn` | `Object.prototype.hasOwnProperty.call(o, k)` |
| `structuredClone` | 필요한 부분만 spread로 복사 |
| `String.prototype.replaceAll` | `s.replace(/x/g, y)`, `s.split(x).join(y)` |
| `Promise.any` | 동시에 실행할 필요가 없으면 `ResultAsync`의 `orElse`로 대안을 차례로 잇기 |
| `crypto.randomUUID` | `crypto.getRandomValues`로 만든 id 함수 (shell에서 호출) |

## 불변 값

- 타입에 `readonly`를 붙입니다. 배열은 `readonly T[]`, 집합과 맵은 `ReadonlySet<T>`, `ReadonlyMap<K, V>`.
- 상수 데이터는 `as const`로 리터럴 타입과 readonly를 함께 얻습니다.
- `sort`, `reverse`, `splice`, `push`는 원본을 바꿉니다. 인수로 받은 배열에는 쓰지 않고, 쓰려면 먼저 복사합니다.
- `readonly`는 얕습니다. 중첩 객체도 타입마다 `readonly`를 붙입니다.

### spread와 immer 고르기

| 갱신 모양 | 쓰는 것 |
| --- | --- |
| 한 단계 (`{ ...s, count: s.count + 1 }`, `[...xs, x]`) | spread |
| 두 단계 이상 중첩, 배열 안 객체 하나 고치기 | immer `produce` |
| 같은 갱신을 여러 곳에서 씀 | 커링한 producer를 core 함수로 |

```ts
// 관계를 문장으로 쓰고 문장 하나를 식 하나로 옮긴다
// - index는 before의 위치, 앞부분은 index 이전, 뒷부분은 index부터 끝
export function insertBefore<T>(list: readonly T[], item: T, before: T): readonly T[] {
  const index = list.indexOf(before);
  if (index === -1) return [...list, item];
  return [...list.slice(0, index), item, ...list.slice(index)];
}
```

```ts
import { produce, type Draft } from 'immer';

type Task = { readonly id: string; readonly title: string; readonly done: boolean };
type Board = {
  readonly columns: readonly { readonly id: string; readonly tasks: readonly Task[] }[];
};

// spread로 쓰면 columns.map → tasks.map → { ...t } 세 겹이 된다
export const completeTask = (board: Board, columnId: string, taskId: string): Board =>
  produce(board, (draft) => {
    const task = draft.columns.find((c) => c.id === columnId)?.tasks.find((t) => t.id === taskId);
    if (task) task.done = true; // draft만 바꾼다. board는 그대로 남는다
  });

// 커링한 producer: (상태, ...인수) => 새 상태 모양의 순수 함수가 된다
// draft 타입은 Draft<Board>로 적는다 (readonly가 풀린 타입)
export const renameTask = produce((draft: Draft<Board>, taskId: string, title: string) => {
  for (const column of draft.columns) {
    const task = column.tasks.find((t) => t.id === taskId);
    if (task) task.title = title;
  }
});
```

- `produce`는 원본을 바꾸지 않고 새 값을 반환하므로 순수 함수입니다. 바뀌지 않은 부분은 원본과 같은 참조를 공유해서, React와 zustand가 바뀐 곳만 다시 그립니다.
- draft를 바꾸는 코드는 `produce` 콜백 안에만 둡니다. draft를 바깥 변수에 담거나 비동기 콜백에서 쓰지 않습니다.
- 콜백에서 draft를 바꾸거나 새 값을 `return`하거나, 둘 중 하나만 합니다. 둘 다 하면 에러가 납니다.
- immer는 결과를 얼립니다(auto freeze). 결과를 다시 고치려 하면 에러가 나므로 다음 갱신도 `produce`나 spread로 합니다.
- 일반 객체와 배열에만 씁니다. `Map`과 `Set`을 draft로 다루려면 앱 시작 시 `enableMapSet()`을 한 번 불러야 합니다. 클래스 인스턴스와 three.js 객체는 넣지 않습니다.
- 콜백 안에서는 계산만 합니다. IO, 현재 시각, 랜덤은 인수로 받은 값을 씁니다.

## 순수 함수

```ts
// 상태를 인수로 받고, 파생 값은 저장하지 않고 계산한다
export const discountPercent = (items: readonly string[]): number => (items.includes('book') ? 5 : 0);

// 현재 시각은 인수로 받는다 (Date.now()를 안에서 읽지 않음)
export const isExpired = (expiresAt: number, now: number): boolean => now >= expiresAt;
```

- 반환 타입을 명시합니다. 시그니처가 문서 역할을 합니다.
- `void`를 반환하는 함수는 shell에만 둡니다.

## 고차 함수와 커링

```ts
export const rankBy =
  <T>(score: (x: T) => number) =>
  (items: readonly T[]): readonly T[] =>
    [...items].sort((a, b) => score(b) - score(a));

export const scoringAbove =
  <T>(score: (x: T) => number) =>
  (threshold: number) =>
  (items: readonly T[]): readonly T[] =>
    items.filter((x) => score(x) > threshold);

const wordScore = (w: string): number => w.replace(/a/g, '').length;
const highScoring = scoringAbove(wordScore); // 덜 바뀌는 인수를 먼저 넣어 둔다
highScoring(1)(['ada', 'haskell', 'scala']);
```

| 연산 | TS |
| --- | --- |
| map / filter / flatMap | `map`, `filter`, `flatMap` |
| fold | `reduce(f, init)` (초기값을 항상 넘깁니다) |
| all / any | `every`, `some` |
| 포함 | `includes`, `Set.has` |

## 파이프라인

```ts
type Book = { readonly title: string; readonly authors: readonly string[] };

export const recommendations = (
  books: readonly Book[],
  adaptations: (author: string) => readonly string[],
): readonly string[] =>
  books.flatMap((book) =>
    book.authors.flatMap((author) => adaptations(author).map((movie) => `${movie} (${author}의 ${book.title} 원작)`)),
  );
```

- 삼항 연산자, 즉시 `return`하는 `switch`로 분기도 식으로 씁니다.
- 일반화된 필터: `xs.flatMap((x) => (keep(x) ? [x] : []))`.

## 없음과 실패: neverthrow

- 없을 수 있는 값은 `T | undefined`. `null`과 섞지 않습니다. 바깥에서 온 `null`(다른 언어의 없음 값, JSON의 `null`)은 경계에서 파싱할 때 `undefined`로 바꿉니다.
- 실패 이유가 필요하면 neverthrow의 `Result<T, E>`를 반환합니다. **타입 파라미터는 성공 타입이 먼저, 에러 타입이 뒤**입니다.
- 비동기 작업의 실패는 `ResultAsync<T, E>`로 반환합니다.

| 하고 싶은 것 | neverthrow |
| --- | --- |
| 성공 / 실패 만들기 | `ok(v)`, `err(e)`, 비동기는 `okAsync(v)`, `errAsync(e)` |
| 성공 값 바꾸기 | `.map(f)` |
| 에러 바꾸기 | `.mapErr(f)` |
| 실패할 수 있는 다음 단계 잇기 | `.andThen(f)` (`f`가 `Result`나 `ResultAsync`를 반환) |
| 실패하면 대안 | `.orElse(f)` |
| 마지막에 두 경우 처리 | `.match(onOk, onErr)` |
| 기본값으로 꺼내기 | 상수는 `.unwrapOr(default)`, 계산이 필요하면 `.match((v) => v, () => compute())` |
| 분기로 꺼내기 | `if (r.isErr()) return ...;` 뒤에는 `r.value` 사용 가능 |
| 여러 개 all-or-nothing | `Result.combine(list)`, `ResultAsync.combine(list)` |
| 여러 개 실패를 모두 모으기 | `Result.combineWithAllErrors(list)` |
| 예외를 던지는 함수 감싸기 | `fromThrowable(fn, toError)` |
| reject될 수 있는 Promise 감싸기 | `ResultAsync.fromPromise(promise, toError)` |
| 단계가 많을 때 순서대로 쓰기 | `safeTry(function* () { const a = yield* r1; ... })` |

```ts
import { err, ok, Result, safeTry } from 'neverthrow';

export type ParseError =
  | { readonly kind: 'emptyName' }
  | { readonly kind: 'notNumber'; readonly raw: string }
  | { readonly kind: 'outOfRange'; readonly value: number };

export const parseAge = (raw: string): Result<number, ParseError> => {
  // Number('')와 Number(' ')는 0이 되므로 숫자 모양부터 확인한다
  if (!/^-?\d+$/.test(raw)) return err({ kind: 'notNumber', raw });
  const n = Number(raw);
  if (n < 0 || n > 150) return err({ kind: 'outOfRange', value: n });
  return ok(n);
};

export const parseName = (raw: string): Result<string, ParseError> =>
  raw.trim() === '' ? err({ kind: 'emptyName' }) : ok(raw.trim());

// 작은 검증을 이어 붙인다. 첫 실패가 결과가 된다
export const parsePerson = (name: string, age: string): Result<{ name: string; age: number }, ParseError> =>
  parseName(name).andThen((n) => parseAge(age).map((a) => ({ name: n, age: a })));

// 단계가 많으면 safeTry로 위에서 아래로 쓴다
export const parsePerson2 = (name: string, age: string): Result<{ name: string; age: number }, ParseError> =>
  safeTry(function* () {
    const n = yield* parseName(name);
    const a = yield* parseAge(age);
    return ok({ name: n, age: a });
  });

// all-or-nothing: 하나라도 실패하면 첫 실패
export const parseAges = (raws: readonly string[]): Result<readonly number[], ParseError> =>
  Result.combine(raws.map(parseAge));

// best-effort: 성공은 모으고 실패 이유도 버리지 않는다 (neverthrow에 없어서 직접 만든다)
export const partition = <T, E>(
  results: readonly Result<T, E>[],
): { readonly values: readonly T[]; readonly errors: readonly E[] } => ({
  values: results.flatMap((r) => (r.isOk() ? [r.value] : [])),
  errors: results.flatMap((r) => (r.isErr() ? [r.error] : [])),
});
```

- 에러 타입은 `string`보다 판별 유니온으로 닫습니다. 처리하는 쪽에서 `switch (e.kind)`와 `assertNever`로 빠짐없이 분기할 수 있습니다.
- `throw`는 불가능한 상태에만 씁니다. 예상 가능한 실패는 `err(...)`로 돌려줍니다.
- 대안을 넘길 때 `unwrapOr(f())`, `orElse(() => saved)`처럼 미리 만든 값을 넘기면 성공했을 때도 `f()`가 실행됩니다. 계산이나 IO가 필요한 대안은 `orElse(() => f())`처럼 실패했을 때 부르게 합니다. `a ?? f()`, `a || f()`는 원래 지연 실행되므로 그대로 씁니다.
- `_unsafeUnwrap()`, `_unsafeUnwrapErr()`는 테스트 코드에서만 씁니다.
- 반환된 `Result`를 쓰지 않고 버리면 실패도 함께 사라집니다. 호출한 쪽은 `match`, `isErr`, `andThen` 중 하나로 반드시 처리합니다.

## 외부 입력은 경계에서 파싱

```ts
import { err, fromThrowable, ok, Result } from 'neverthrow';

// Brand는 아래 "요구사항을 타입으로" 절에 있다
export type Percent = Brand<number, 'Percent'>;

export type PercentError =
  | { readonly kind: 'notInteger'; readonly raw: string }
  | { readonly kind: 'outOfRange'; readonly value: number };

export type ConfigError =
  | PercentError
  | { readonly kind: 'invalidJson'; readonly message: string }
  | { readonly kind: 'missingField'; readonly field: string };

export const parsePercent = (raw: unknown): Result<Percent, PercentError> => {
  if (typeof raw !== 'number' || !Number.isInteger(raw)) return err({ kind: 'notInteger', raw: String(raw) });
  if (raw < 0 || raw > 100) return err({ kind: 'outOfRange', value: raw });
  return ok(raw as Percent); // branded 값은 여기서만 만든다
};

// 예외를 던지는 API는 경계에서 한 번 감싼다
export const parseJson = fromThrowable(
  (text: string): unknown => JSON.parse(text),
  (e): ConfigError => ({ kind: 'invalidJson', message: e instanceof Error ? e.message : String(e) }),
);

export const readVolume = (text: string): Result<Percent, ConfigError> =>
  parseJson(text).andThen(
    (json): Result<Percent, ConfigError> =>
      typeof json === 'object' && json !== null && 'volume' in json
        ? parsePercent(json.volume)
        : err({ kind: 'missingField', field: 'volume' }),
  );

// null로 올 수 있는 필드는 경계에서 undefined로 바꾼다
export const optional = <T>(v: T | null | undefined): T | undefined => v ?? undefined;
```

- 바깥에서 온 값은 `unknown`으로 받습니다. `any`나 `as SomeType` 캐스팅으로 검증을 건너뛰지 않습니다.
- 작은 파싱 함수의 에러 타입(`PercentError`)을 큰 함수의 에러 타입(`ConfigError`)이 포함하게 두면, 조합할 때 변환 없이 이어집니다.
- 작은 필드 검증 함수를 먼저 만들고 `andThen`으로 조합합니다. 새 검증은 단계 하나 추가로 들어옵니다.

## 요구사항을 타입으로

```ts
type Brand<T, B extends string> = T & { readonly __brand: B };
export type UserId = Brand<string, 'UserId'>;
export type OrderId = Brand<string, 'OrderId'>; // UserId 자리에 OrderId를 넘기면 컴파일 에러

// isLoading, error, data를 따로 두면 "로딩 중인데 에러도 있음" 같은 조합이 생긴다
export type Remote<T, E> =
  | { readonly kind: 'idle' }
  | { readonly kind: 'loading' }
  | { readonly kind: 'success'; readonly data: T }
  | { readonly kind: 'failure'; readonly error: E };

export const assertNever = (x: never): never => {
  throw new Error(`처리하지 않은 경우: ${JSON.stringify(x)}`);
};

export function remoteLabel<T, E>(r: Remote<T, E>): string {
  switch (r.kind) {
    case 'idle':
      return '대기';
    case 'loading':
      return '불러오는 중';
    case 'success':
      return '완료';
    case 'failure':
      return '실패';
    default:
      return assertNever(r); // 경우를 추가하면 여기서 컴파일 에러가 난다
  }
}

// Result를 화면 상태로 옮길 때
export const toRemote = <T, E>(r: Result<T, E>): Remote<T, E> =>
  r.match(
    (data): Remote<T, E> => ({ kind: 'success', data }),
    (error): Remote<T, E> => ({ kind: 'failure', error }),
  );
```

- 판별 필드 이름은 `kind`로 통일합니다.
- 타입 파라미터 순서는 neverthrow와 맞춰 성공 타입을 먼저 씁니다(`Remote<T, E>`).
- `switch`의 `default`에는 `assertNever`만 둡니다. 다른 값을 돌려주는 `default`는 누락을 숨깁니다.
- 정해진 몇 가지 값은 `string` 대신 리터럴 유니온(`'small' | 'medium' | 'large'`)으로 받습니다.
- 선택 필드(`?:`) 여러 개가 함께 있거나 없어야 한다면 유니온 케이스로 묶습니다.

**동작을 데이터로:**

```ts
type Item = { readonly tags: readonly string[]; readonly createdAt: number };

export type Condition =
  | { readonly kind: 'hasAnyTag'; readonly tags: readonly string[] }
  | { readonly kind: 'createdBetween'; readonly from: number; readonly to: number };

const matches = (item: Item, c: Condition): boolean => {
  switch (c.kind) {
    case 'hasAnyTag':
      return c.tags.some((t) => item.tags.includes(t));
    case 'createdBetween':
      return item.createdAt >= c.from && item.createdAt <= c.to;
    default:
      return assertNever(c);
  }
};

// 조건이 없으면 every가 true → "빈 배열 = 전체"가 결과로 나온다
export const search = (items: readonly Item[], conditions: readonly Condition[]): readonly Item[] =>
  items.filter((item) => conditions.every((c) => matches(item, c)));
```

## 사이드 이펙트와 비동기

- `Promise`와 `ResultAsync`는 만들자마자 실행됩니다. 나중에 실행하거나 다시 시도할 동작은 `() => ResultAsync<T, E>`처럼 함수로 넘깁니다. 대안 요청도 같습니다. `primary().orElse(() => backup())`은 실패했을 때만 `backup`을 부르지만, `const b = backup()`을 먼저 만들어 두면 성공해도 요청이 나갑니다.
- IO 경계 모듈에서 reject될 수 있는 Promise를 `ResultAsync.fromPromise`로 감쌉니다. 그 안쪽은 reject 없이 값으로만 실패를 다룹니다.

```ts
import { errAsync, ResultAsync } from 'neverthrow';

type User = { readonly id: UserId; readonly name: string };
export type LoadError = { readonly kind: 'network'; readonly message: string };

const toLoadError = (e: unknown): LoadError => ({
  kind: 'network',
  message: e instanceof Error ? e.message : String(e),
});

// 필요한 외부 기능의 시그니처 묶음 (테스트에서는 okAsync/errAsync를 돌려주는 함수를 넘긴다)
export type Deps = {
  readonly fetchUsers: () => ResultAsync<readonly User[], LoadError>;
  readonly currentHour: () => number; // 현재 시각과 시간대는 둘 다 숨은 입력이라 함께 바깥에 맡긴다
};

// IO 경계 모듈 쪽: 실제 구현 (invoke는 @tauri-apps/api/core)
export const fetchUsersVia =
  (invoke: (cmd: string) => Promise<unknown>) => (): ResultAsync<readonly User[], LoadError> =>
    ResultAsync.fromPromise(invoke('list_users') as Promise<readonly User[]>, toLoadError);

// 순수: 계산만
export const greeting = (users: readonly User[], hour: number): string =>
  `${hour < 12 ? '좋은 아침' : '안녕하세요'}, ${users.map((u) => u.name).join(', ')}`;

// 조립: 의존성을 앞 인수로 받는다 (부분 적용 = 의존성 주입)
export const loadGreeting = (deps: Deps) => (): ResultAsync<string, LoadError> =>
  deps.fetchUsers().map((users) => greeting(users, deps.currentHour()));

// shell에서 넘기는 실제 구현: { fetchUsers: io.fetchUsers, currentHour: () => new Date().getHours() }

// 재시도와 대안도 값의 조합
export const retry = <T, E>(action: () => ResultAsync<T, E>, times: number): ResultAsync<T, E> =>
  action().orElse((e) => (times > 0 ? retry(action, times - 1) : errAsync(e)));
```

- 캐시, 타임아웃, 재시도는 `Deps`의 구현을 감싸 바꿔 끼웁니다. 계산 함수는 그대로 둡니다.
- 위 `fetchUsersVia`의 `as` 캐스팅은 예시를 줄이려고 쓴 것입니다. 실제로는 결과를 `unknown`으로 받아 파싱 함수로 `andThen`합니다.

## 동시성, 흐름, 자원

- 독립 작업만 동시에 실행합니다. 모두 성공해야 하면 `ResultAsync.combine([a(), b()])`, 일부 실패를 허용하면 각 결과를 모아 위의 `partition`으로 나눕니다. 앞 결과가 필요한 작업은 `andThen`으로 순서대로 잇습니다.
- 취소는 `AbortController`로 하고, 백그라운드 작업을 시작하는 함수는 `{ stop: () => void }` 같은 핸들을 반환합니다.
- 개수를 모르는 흐름은 async iterator(`for await`)나 구독으로 받고, 판단은 배열을 받는 순수 함수로 둡니다.

```ts
import { errAsync, okAsync, type ResultAsync } from 'neverthrow';

// 최근 값들이 계속 오르는가: 흐름과 무관하게 테스트할 수 있는 순수 함수
export const isRising = (values: readonly number[]): boolean =>
  values.length > 1 && values.slice(1).every((v, i) => v > values[i]);

// 획득, 사용, 해제를 한 함수로 묶어 사용이 실패해도 해제한다
export const withResource = <R, A, E>(
  acquire: () => ResultAsync<R, E>,
  release: (r: R) => ResultAsync<void, E>,
  use: (r: R) => ResultAsync<A, E>,
): ResultAsync<A, E> =>
  acquire().andThen((r) =>
    use(r)
      // 사용이 실패하면 해제한 뒤 사용의 에러를 돌려준다 (해제 실패보다 원인이 중요하다)
      .orElse((e) => release(r).orElse(() => okAsync(undefined)).andThen(() => errAsync(e)))
      // 사용이 성공하면 해제하고 값을 돌려준다
      .andThen((a) => release(r).map(() => a)),
  );
```

- 해제는 성공과 실패 경로에서 정확히 한 번씩만 일어납니다. 획득이 실패하면 해제할 것이 없으므로 부르지 않습니다.

## 테스트

예시는 Vitest와 fast-check로 썼습니다. 속성 기반 테스트는 fast-check의 생성기로 입력을 만들어 돌립니다.

```ts
// profile.test.ts
import { fc, test } from '@fast-check/vitest';
import { describe, expect, it } from 'vitest';
import { parseAge, partition } from './profile';

describe('parseAge', () => {
  it('빈 문자열은 0이 아니라 실패다', () => {
    expect(parseAge('')._unsafeUnwrapErr()).toEqual({ kind: 'notNumber', raw: '' });
  });

  // 속성: 0~150의 정수는 문자열로 넣어도 같은 수로 돌아온다
  test.prop([fc.integer({ min: 0, max: 150 })])('범위 안의 정수는 그대로 파싱된다', (n) => {
    expect(parseAge(String(n))._unsafeUnwrap()).toBe(n);
  });
});

// 속성: best-effort도 입력을 버리지 않는다. 성공과 실패를 더하면 입력 개수다
test.prop([fc.array(fc.string())])('partition은 모든 결과를 어느 한쪽에 담는다', (raws) => {
  const { values, errors } = partition(raws.map(parseAge));
  expect(values.length + errors.length).toBe(raws.length);
});
```

- core는 "인수 → 결과"만으로 검증할 수 있게 만듭니다.
- 예시 테스트는 요구사항에 적힌 경우와 경계값을, 속성 테스트는 "어떤 입력이든 성립해야 하는 것"을 맡습니다. 위의 빈 문자열처럼 사람이 놓친 입력은 속성 테스트가 자주 찾아냅니다.
- stub은 `Deps` 타입의 객체 리터럴입니다: `{ fetchUsers: () => okAsync([user]), currentHour: () => 9 }`. 실패 경로는 `errAsync(...)`를 넘겨 확인합니다.
- `Result`는 `expect(r.isOk()).toBe(true)`처럼 확인하거나, 테스트에서만 `_unsafeUnwrap()`으로 값을 꺼냅니다.
- `produce`를 쓴 함수는 "원본이 그대로인가"와 "바뀌지 않은 부분의 참조가 같은가"도 함께 확인합니다.
- 기대값은 요구사항으로 계산해 주석에 남깁니다.

## 체크리스트

1. 바깥에서 온 값이 `unknown`으로 들어와 경계에서 파싱되는가? `any`, 근거 없는 `as`가 있는가?
2. 실패할 수 있는 함수가 `Result`나 `ResultAsync`를 반환하는가? 예상 가능한 실패를 `throw`하는가?
3. 반환된 `Result`를 처리하지 않고 버리는 곳이 있는가? 운영 코드에 `_unsafeUnwrap()`이 있는가?
4. `ResultAsync`나 `Promise`를 미리 만들어 두고 나중에 쓰려 하는가? 계산이나 IO가 필요한 대안을 미리 만들어 넘기는가?
5. 인수로 받은 배열에 `sort`, `reverse`, `splice`, `push`를 쓰는가?
6. `produce` 밖에서 draft를 쓰거나, 콜백에서 draft 변경과 `return`을 함께 하는가? 콜백 안에서 IO를 하는가?
7. 타입에 `readonly`가 있는가?
8. 판별 유니온 `switch`의 `default`가 `assertNever`인가?
9. 빌드 타깃이 지원하지 않는 메서드를 쓰는가?
10. core 파일이 `react`, `zustand`, `three`, `@react-three/*`, `@tauri-apps/*`, IO 경계 모듈을 import하는가? IO 경계 밖에서 플랫폼 API를 부르는가?
