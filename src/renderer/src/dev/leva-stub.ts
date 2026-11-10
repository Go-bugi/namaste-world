// 프로덕션 빌드에서 `leva` 대신 쓰는 스텁. vite.config.ts가 `vite build`일 때만 별칭으로 연결한다.
// 패널은 그리지 않고, useControls는 스키마의 기본값만 돌려준다.
// 컴포넌트는 그대로 `import { useControls } from 'leva'`로 쓰고, 타입은 실제 leva에서 온다.

type AnyRecord = Record<string, unknown>;

function isRecord(value: unknown): value is AnyRecord {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** leva 스키마에서 기본값을 꺼낸다. folder()는 펼치고, button()은 값이 없으므로 건너뛴다. */
function defaultsOf(schema: AnyRecord, out: AnyRecord = {}): AnyRecord {
  for (const [key, input] of Object.entries(schema)) {
    if (isRecord(input) && isRecord(input.schema)) {
      defaultsOf(input.schema, out);
    } else if (isRecord(input) && typeof input.onClick === 'function') {
      continue;
    } else if (isRecord(input) && 'value' in input) {
      out[key] = input.value;
    } else {
      out[key] = input;
    }
  }
  return out;
}

export function useControls(...args: unknown[]): unknown {
  // useControls(schema) 또는 useControls('폴더 이름', schema)
  const raw = typeof args[0] === 'string' ? args[1] : args[0];
  // 스키마를 함수로 넘기면 [값, set] 튜플을 돌려준다
  if (typeof raw === 'function') {
    return [defaultsOf((raw as () => AnyRecord)()), () => {}];
  }
  return defaultsOf(isRecord(raw) ? raw : {});
}

export const folder = (schema: AnyRecord, settings?: AnyRecord) => ({ schema, settings });
export const button = (onClick: () => void) => ({ onClick });

export function Leva(): null {
  return null;
}
