/**
 * Rust가 renderer로 보내는 이벤트 이름. `src-tauri/src/events.rs`의 상수와 같은 값이다.
 * payload 타입은 `types/bindings/`에 있다.
 */

/** 훅 이벤트를 하나 받았다. payload는 HookEvent다. */
export const HOOK_EVENT = 'hook-event';

/** 훅 설치가 끝났다. payload는 HookInstallation이다. */
export const HOOK_INSTALLATION = 'hook-installation';
