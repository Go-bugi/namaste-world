//! renderer로 보내는 이벤트 이름. `src/renderer/src/types/ipc.ts`에 같은 이름의 상수가 있다.
//! payload 타입은 그 값을 만드는 로직 모듈(`hook::domain::model`)에 정의하고 ts-rs로 내보낸다.

/// 훅 이벤트를 하나 받았다. payload는 `HookEvent`다.
pub const HOOK_EVENT: &str = "hook-event";

/// 훅 설치가 끝났다. payload는 `HookInstallation`이다.
pub const HOOK_INSTALLATION: &str = "hook-installation";
